//! 已知模型的元数据目录（上下文窗口 / 最大输出）。
//!
//! 数据来源：`https://models.dev/api.json` 的 `opencode-go` provider 目录
//! （与网关 `GET /models` 返回的 36 个模型 id 一一对应）。
//!
//! 为什么需要内置这份表：网关的 `GET /models` 只返回
//! `id / object / created / owned_by` 四个字段，**不含上下文长度**，
//! 而各模型上限差异极大（20 万 ~ 105 万），无法用一个全局值近似。
//!
//! 生效优先级（见 `AiSettingsPort` 实现）：
//! 1. 用户在设置页按模型覆盖的值
//! 2. 本目录中的已知值
//! 3. [`DEFAULT_CONTEXT_LIMIT`]（目录未收录的模型，例如自建网关）

/// 单个模型的规格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    /// 模型 id（与网关返回一致）。
    pub id: &'static str,
    /// 上下文窗口上限（token）。
    pub context_limit: usize,
    /// 单次请求最大输出（token）。
    pub max_output: usize,
}

/// 目录未收录模型时的默认上下文预算。
///
/// 取偏保守的值：低估会提前提醒，高估才会导致超限报错。
pub const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

/// OpenCode Go 目录（36 个模型）。
pub const OPENCODE_GO_MODELS: &[ModelSpec] = &[
    ModelSpec { id: "deepseek-flash", context_limit: 1_000_000, max_output: 384_000 },
    ModelSpec { id: "deepseek-v4-flash", context_limit: 1_000_000, max_output: 384_000 },
    ModelSpec { id: "deepseek-v4-flash-vision-exp", context_limit: 1_000_000, max_output: 384_000 },
    ModelSpec { id: "deepseek-v4-pro", context_limit: 1_000_000, max_output: 384_000 },
    ModelSpec { id: "glm-5", context_limit: 202_752, max_output: 32_768 },
    ModelSpec { id: "glm-5.1", context_limit: 202_752, max_output: 32_768 },
    ModelSpec { id: "glm-5.2", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "glm-5.3", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "glm-5.3-flash", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "gpt-5.6-luna", context_limit: 1_050_000, max_output: 128_000 },
    ModelSpec { id: "grok-4.5", context_limit: 500_000, max_output: 500_000 },
    ModelSpec { id: "grok-4.6", context_limit: 500_000, max_output: 500_000 },
    ModelSpec { id: "hy3", context_limit: 256_000, max_output: 128_000 },
    ModelSpec { id: "hy4-preview", context_limit: 1_024_000, max_output: 64_000 },
    ModelSpec { id: "kimi-k2.5", context_limit: 262_144, max_output: 65_536 },
    ModelSpec { id: "kimi-k2.6", context_limit: 262_144, max_output: 65_536 },
    ModelSpec { id: "kimi-k2.7-code", context_limit: 262_144, max_output: 262_144 },
    ModelSpec { id: "kimi-k3", context_limit: 1_048_576, max_output: 131_072 },
    ModelSpec { id: "longcat-2.0", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "mimo-v2-omni", context_limit: 262_144, max_output: 128_000 },
    ModelSpec { id: "mimo-v2-pro", context_limit: 1_048_576, max_output: 128_000 },
    ModelSpec { id: "mimo-v2.5", context_limit: 1_000_000, max_output: 128_000 },
    ModelSpec { id: "mimo-v2.5-pro", context_limit: 1_048_576, max_output: 128_000 },
    ModelSpec { id: "minimax-m2.5", context_limit: 204_800, max_output: 65_536 },
    ModelSpec { id: "minimax-m2.7", context_limit: 204_800, max_output: 131_072 },
    ModelSpec { id: "minimax-m3", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "muse-spark-1.2-contributor", context_limit: 1_048_576, max_output: 131_072 },
    ModelSpec { id: "muse-spark-1.3-contributor", context_limit: 1_048_576, max_output: 131_072 },
    ModelSpec { id: "omen-alpha", context_limit: 500_000, max_output: 128_000 },
    ModelSpec { id: "ox-alpha-free", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "qwen3.5-plus", context_limit: 262_144, max_output: 65_536 },
    ModelSpec { id: "qwen3.6-plus", context_limit: 1_000_000, max_output: 65_536 },
    ModelSpec { id: "qwen3.7-max", context_limit: 1_000_000, max_output: 65_536 },
    ModelSpec { id: "qwen3.7-plus", context_limit: 1_000_000, max_output: 65_536 },
    ModelSpec { id: "qwen3.8-flash", context_limit: 1_000_000, max_output: 131_072 },
    ModelSpec { id: "qwen3.8-max", context_limit: 1_000_000, max_output: 131_072 },
];

/// 按 id 查询模型规格（未收录返回 None）。
pub fn find_model(model: &str) -> Option<&'static ModelSpec> {
    OPENCODE_GO_MODELS.iter().find(|m| m.id == model)
}

/// 按 id 查询上下文上限（未收录返回 None）。
pub fn context_limit_for(model: &str) -> Option<usize> {
    find_model(model).map(|m| m.context_limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_models_report_their_real_context_window() {
        assert_eq!(context_limit_for("mimo-v2.5"), Some(1_000_000));
        assert_eq!(context_limit_for("minimax-m2.5"), Some(204_800));
        assert_eq!(context_limit_for("glm-5"), Some(202_752));
        assert_eq!(context_limit_for("kimi-k3"), Some(1_048_576));
    }

    #[test]
    fn unknown_model_has_no_entry() {
        assert_eq!(context_limit_for("not-a-real-model"), None);
    }

    #[test]
    fn catalog_ids_are_unique() {
        let mut ids: Vec<&str> = OPENCODE_GO_MODELS.iter().map(|m| m.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "目录中存在重复的模型 id");
    }
}
