//! ⚠️ 本文件由 tmp/gen_sqlite_backend.py 自动生成，请勿手工编辑。
//! 如需修改逻辑，请改 PG 侧的对应文件后重新生成。

//! 全局应用设置接口（设置页持久化 + AI 网关连通性测试）。

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::error::AppError;
use crate::state::AppState;
use sqlite_db::application_ports::DbSettingsRepositoryPort;
use domain::ports::AiRuntimeConfig;
use infrastructure::llm::{
    LlmProvider, LlmRequest, Message, OpenAiCompatibleProvider, StaticAiSettings,
};

pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let port = DbSettingsRepositoryPort::new(state.pool.clone());
    let settings = port.get_settings().await?;
    Ok(Json(settings))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let port = DbSettingsRepositoryPort::new(state.pool.clone());
    let settings = port.upsert_settings(input).await?;
    Ok(Json(settings))
}

/// 「测试连接」入参：三个字段都可选，未传（或空串）时取当前生效配置。
#[derive(Deserialize)]
pub struct TestConnectionInput {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// 用给定（或当前生效）的网关参数真发一次最小请求，验证连通性。
///
/// 连通性失败属于「测试结果」而非请求错误，因此统一返回 200：
/// `{ ok: false, error }` 由前端直接展示原因。
pub async fn test_connection(
    State(state): State<AppState>,
    Json(input): Json<TestConnectionInput>,
) -> Result<Json<Value>, AppError> {
    let current = state.ai_settings.load().await?;
    let config = AiRuntimeConfig {
        base_url: non_empty(input.base_url).unwrap_or(current.base_url),
        api_key: non_empty(input.api_key).or(current.api_key),
        model: non_empty(input.model).unwrap_or(current.model),
        context_limit: current.context_limit,
        max_output_tokens: current.max_output_tokens,
    };

    let provider = OpenAiCompatibleProvider::new(Arc::new(StaticAiSettings::new(config.clone())));
    let request = LlmRequest {
        messages: vec![Message {
            role: "user".to_string(),
            content: "ping".to_string(),
        }],
        max_tokens: 16,
        temperature: 0.0,
        model: config.model.clone(),
    };

    let started = std::time::Instant::now();
    match provider.generate(request).await {
        Ok(response) => Ok(Json(json!({
            "ok": true,
            "model": response.model,
            "latency_ms": started.elapsed().as_millis(),
            "reply": response.content,
        }))),
        Err(error) => Ok(Json(json!({
            "ok": false,
            "model": config.model,
            "latency_ms": started.elapsed().as_millis(),
            "error": error.to_string(),
        }))),
    }
}

/// 「获取模型列表」入参：两个字段都可选，未传（或空串）时取当前生效配置。
#[derive(Deserialize)]
pub struct ListModelsInput {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// 拉取网关可用模型列表（设置页模型名下拉的数据源）。
///
/// 与「测试连接」一致：失败属于测试结果而非请求错误，返回 200
/// `{ ok: false, error }` 交给前端展示。
pub async fn list_models(
    State(state): State<AppState>,
    Json(input): Json<ListModelsInput>,
) -> Result<Json<Value>, AppError> {
    let current = state.ai_settings.load().await?;
    let config = AiRuntimeConfig {
        base_url: non_empty(input.base_url).unwrap_or(current.base_url),
        api_key: non_empty(input.api_key).or(current.api_key),
        model: current.model,
        context_limit: current.context_limit,
        max_output_tokens: current.max_output_tokens,
    };

    let provider = OpenAiCompatibleProvider::new(Arc::new(StaticAiSettings::new(config)));
    match provider.list_models().await {
        Ok(models) => Ok(Json(json!({ "ok": true, "models": models }))),
        Err(error) => Ok(Json(json!({ "ok": false, "error": error.to_string() }))),
    }
}

/// 内置模型目录：模型 id → 上下文上限（token）。
///
/// 网关 `GET /models` 不返回上下文长度，这里给出内置的真实目录，
/// 供设置页在切换模型时自动带出该模型的上限。
pub async fn model_catalog() -> Json<Value> {
    let context_limits: serde_json::Map<String, Value> =
        domain::model_catalog::OPENCODE_GO_MODELS
            .iter()
            .map(|m| (m.id.to_string(), json!(m.context_limit)))
            .collect();

    Json(json!({
        "default_limit": domain::model_catalog::DEFAULT_CONTEXT_LIMIT,
        "context_limits": context_limits,
    }))
}

/// 空串视为未传。
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}
