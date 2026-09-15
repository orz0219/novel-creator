//! `AiSettingsPort` 的 PostgreSQL 实现。
//!
//! 读取 `app_settings.settings`（JSONB）中的 AI 字段，供 LLM Provider 在
//! 每次请求前取用——因此设置页保存后立即生效，无需重启服务。
//!
//! 字段约定（由前端「设置」页写入）：
//! - `defaultModel`   → 模型名
//! - `aiBaseUrl`      → OpenAI 兼容网关前缀
//! - `aiApiKey`       → 网关密钥
//! - `contextLimit`   → 未收录模型的默认上下文预算
//! - `contextLimits`  → 按模型的上下文上限覆盖：`{ "<模型id>": <token 数> }`
//!
//! 某字段缺失或为空串时，回落到构造时传入的环境变量默认值。

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;

use domain::model_catalog;
use domain::ports::{AiRuntimeConfig, AiSettingsPort, GenerationPurpose};

/// 环境变量提供的默认值（设置页未配置对应字段时使用）。
#[derive(Debug, Clone)]
pub struct AiSettingsDefaults {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    /// 目录未收录模型的默认上下文预算。
    pub context_limit: usize,
    /// 单次输出 token 默认上限（设置页未配置时使用）。
    pub max_output_tokens: u32,
}

impl AiSettingsDefaults {
    /// 按启动约定从环境变量读取默认值。
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("OPENCODE_BASE_URL")
                .unwrap_or_else(|_| "https://opencode.ai/zen/go/v1".to_string()),
            api_key: std::env::var("OPENCODE_API_KEY").ok(),
            model: std::env::var("OPENCODE_MODEL").unwrap_or_else(|_| "mimo-v2.5".to_string()),
            context_limit: std::env::var("OPENCODE_CONTEXT_LIMIT")
                .ok()
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(model_catalog::DEFAULT_CONTEXT_LIMIT),
            max_output_tokens: std::env::var("OPENCODE_MAX_OUTPUT_TOKENS")
                .ok()
                .and_then(|v| v.trim().parse::<u32>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(22_000),
        }
    }
}

/// 从 `app_settings` 读取运行时 AI 配置。
pub struct DbAiSettingsPort {
    pool: PgPool,
    defaults: AiSettingsDefaults,
}

impl DbAiSettingsPort {
    pub fn new(pool: PgPool, defaults: AiSettingsDefaults) -> Self {
        Self { pool, defaults }
    }

    /// 取出 app_settings 的 JSON 对象（无记录时为空对象）。
    async fn settings_json(&self) -> Result<Value> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT settings::text FROM app_settings WHERE id = 'default'",
        )
        .fetch_optional(&self.pool)
        .await
        .context("读取 app_settings 失败（AI 配置）")?;

        match row {
            Some((raw,)) => {
                serde_json::from_str(&raw).context("app_settings.settings 不是合法 JSON")
            }
            None => Ok(Value::Object(serde_json::Map::new())),
        }
    }
}

#[async_trait]
impl AiSettingsPort for DbAiSettingsPort {
    async fn load(&self) -> Result<AiRuntimeConfig> {
        let settings = self.settings_json().await?;

        let model = non_empty_str(&settings, "defaultModel")
            .unwrap_or_else(|| self.defaults.model.clone());

        let max_output_tokens = parse_limit(settings.get("maxOutputTokens"))
            .map(|v| v as u32)
            .unwrap_or(self.defaults.max_output_tokens);

        Ok(AiRuntimeConfig {
            base_url: non_empty_str(&settings, "aiBaseUrl")
                .unwrap_or_else(|| self.defaults.base_url.clone()),
            api_key: non_empty_str(&settings, "aiApiKey")
                .or_else(|| self.defaults.api_key.clone()),
            context_limit: resolve_context_limit(&settings, &model, self.defaults.context_limit),
            max_output_tokens,
            model,
            task_models: parse_task_models(&settings)?,
            task_temperatures: parse_task_temperatures(&settings)?,
        })
    }
}

/// 解析「按用途分配的模型」：`taskModels: { "agent": "模型名", ... }`。
///
/// 用途键必须是系统认识的（[`GenerationPurpose::parse`]）：写错的键直接报错，
/// 而不是当成一个永远不会被读到的配置静静躺着——那正是"设了却没生效"的来源。
fn parse_task_models(settings: &Value) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    let Some(raw) = settings.get("taskModels") else {
        return Ok(out);
    };
    let Some(obj) = raw.as_object() else {
        anyhow::bail!("taskModels 应为对象（{{\"用途\": \"模型名\"}}），收到：{}", raw);
    };
    for (key, value) in obj {
        let purpose = GenerationPurpose::parse(key)?;
        let model = value
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "taskModels.{} 应为非空字符串（模型名），收到：{}",
                    key,
                    value
                )
            })?;
        out.insert(purpose.key().to_string(), model.to_string());
    }
    Ok(out)
}

/// 解析「按用途分配的温度」：`taskTemperatures: { "prose": 0.95, ... }`。
fn parse_task_temperatures(settings: &Value) -> Result<HashMap<String, f32>> {
    let mut out = HashMap::new();
    let Some(raw) = settings.get("taskTemperatures") else {
        return Ok(out);
    };
    let Some(obj) = raw.as_object() else {
        anyhow::bail!(
            "taskTemperatures 应为对象（{{\"用途\": 温度}}），收到：{}",
            raw
        );
    };
    for (key, value) in obj {
        let purpose = GenerationPurpose::parse(key)?;
        let temperature = match value {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
        .ok_or_else(|| {
            anyhow::anyhow!("taskTemperatures.{} 应为数字，收到：{}", key, value)
        })? as f32;
        // 温度超出 [0,2] 是配置错误而不是"会自动夹紧"：夹紧会掩盖用户写错的意图
        if !(0.0..=2.0).contains(&temperature) {
            anyhow::bail!(
                "taskTemperatures.{} 应在 0 与 2 之间，收到：{}",
                key,
                temperature
            );
        }
        out.insert(purpose.key().to_string(), temperature);
    }
    Ok(out)
}

/// 解析当前模型的上下文上限，优先级由高到低：
/// 1. 用户在设置页按模型覆盖的值（`contextLimits[model]`）
/// 2. 内置模型目录中的已知值
/// 3. 默认预算（目录未收录的模型）
fn resolve_context_limit(settings: &Value, model: &str, fallback: usize) -> usize {
    parse_limit(settings.get("contextLimits").and_then(|v| v.get(model)))
        .or_else(|| model_catalog::context_limit_for(model))
        .unwrap_or(fallback)
}

/// 把 JSON 值解析为合法的 token 上限（正整数）；其余情况视为未配置。
fn parse_limit(value: Option<&Value>) -> Option<usize> {
    let raw = match value? {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.trim().to_string(),
        _ => return None,
    };
    raw.parse::<usize>().ok().filter(|n| *n > 0)
}

/// 取 JSON 对象中指定 key 的非空字符串；缺失 / 空串 / 非字符串均视为「未配置」。
fn non_empty_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn non_empty_str_treats_blank_and_missing_as_unset() {
        let settings = json!({
            "defaultModel": "mimo-v2.5",
            "aiBaseUrl": "   ",
            "aiApiKey": "",
        });

        assert_eq!(
            non_empty_str(&settings, "defaultModel").as_deref(),
            Some("mimo-v2.5")
        );
        // 只有空白的地址视为未配置 → 回落到默认值
        assert_eq!(non_empty_str(&settings, "aiBaseUrl"), None);
        // 显式空串的密钥视为未配置 → 回落到默认值
        assert_eq!(non_empty_str(&settings, "aiApiKey"), None);
        // 完全没有的 key 同样视为未配置
        assert_eq!(non_empty_str(&settings, "absent"), None);
    }

    #[test]
    fn non_empty_str_trims_surrounding_whitespace() {
        let settings = json!({ "defaultModel": "  mimo-v2  " });
        assert_eq!(
            non_empty_str(&settings, "defaultModel").as_deref(),
            Some("mimo-v2")
        );
    }

    #[test]
    fn context_limit_prefers_override_then_catalog_then_default() {
        let settings = json!({ "contextLimits": { "mimo-v2.5": 50_000 } });

        // 1) 用户按模型覆盖的值优先
        assert_eq!(resolve_context_limit(&settings, "mimo-v2.5", 128_000), 50_000);
        // 2) 无覆盖时用内置目录的真实上限
        assert_eq!(
            resolve_context_limit(&settings, "minimax-m2.5", 128_000),
            204_800
        );
        // 3) 目录未收录（自建网关）时用默认预算
        assert_eq!(
            resolve_context_limit(&settings, "self-hosted-model", 128_000),
            128_000
        );
    }

    #[test]
    fn parse_limit_accepts_number_and_numeric_string_only() {
        assert_eq!(parse_limit(Some(&json!(1000))), Some(1000));
        assert_eq!(parse_limit(Some(&json!("2000"))), Some(2000));
        assert_eq!(parse_limit(Some(&json!(0))), None);
        assert_eq!(parse_limit(Some(&json!("abc"))), None);
        assert_eq!(parse_limit(Some(&json!(null))), None);
        assert_eq!(parse_limit(None), None);
    }

    #[test]
    fn parse_task_models_accepts_known_purposes() {
        let s = json!({"taskModels": {"agent": "logic-model", "prose": "prose-model"}});
        let m = parse_task_models(&s).unwrap();
        assert_eq!(m.get("agent").map(String::as_str), Some("logic-model"));
        assert_eq!(m.get("prose").map(String::as_str), Some("prose-model"));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn parse_task_models_rejects_unknown_purpose() {
        // 拼错的键必须报错：静默忽略会让用户以为"设了却在生效"，
        // 而真相是这个键永远不会被任何调用点读到。
        let s = json!({"taskModels": {"agnt": "x"}});
        let err = parse_task_models(&s).unwrap_err().to_string();
        assert!(err.contains("未知的 LLM 用途键"), "{}", err);
        assert!(err.contains("agent"), "错误信息要列出合法键：{}", err);
    }

    #[test]
    fn parse_task_models_rejects_blank_model_name() {
        // 空串必须报错而不是当成"未配置"：否则用户清空下拉后，
        // 保存会在库里留下一个空模型名，之后每次调用都失败。
        let s = json!({"taskModels": {"agent": "   "}});
        assert!(parse_task_models(&s).is_err());
    }

    #[test]
    fn parse_task_models_rejects_non_object() {
        let s = json!({"taskModels": ["agent"]});
        assert!(parse_task_models(&s).is_err());
    }

    #[test]
    fn parse_task_temperatures_accepts_numbers_and_numeric_strings() {
        let s = json!({"taskTemperatures": {"prose": 0.95, "agent": "0.4"}});
        let t = parse_task_temperatures(&s).unwrap();
        assert!((t["prose"] - 0.95).abs() < 1e-6);
        assert!((t["agent"] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn parse_task_temperatures_rejects_out_of_range() {
        // 越界报错而不是夹紧：夹紧会掩盖用户写错的意图
        let s = json!({"taskTemperatures": {"prose": 3.0}});
        let err = parse_task_temperatures(&s).unwrap_err().to_string();
        assert!(err.contains("应在 0 与 2 之间"), "{}", err);
    }

    #[test]
    fn parse_task_temperatures_rejects_non_numeric() {
        let s = json!({"taskTemperatures": {"prose": "热一点"}});
        assert!(parse_task_temperatures(&s).is_err());
    }

    #[test]
    fn missing_keys_mean_no_override() {
        let s = json!({});
        assert!(parse_task_models(&s).unwrap().is_empty());
        assert!(parse_task_temperatures(&s).unwrap().is_empty());
    }
}
