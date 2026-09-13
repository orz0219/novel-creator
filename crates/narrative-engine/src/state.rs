//! Application State

use std::sync::Arc;

use sqlx::PgPool;

use agent::AgentRuntime;
use domain::ports::AiSettingsPort;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// P1 引导式 Agent 运行时（会话 / 工具 / 记忆，内存实现）。
    pub agent: Arc<AgentRuntime>,
    /// 运行时 AI 配置端口（设置页可改的模型 / 网关参数）。
    ///
    /// 对话、生成、抽取三处的 LLM Provider 共用它，因此设置页保存后
    /// 对所有 AI 功能立即生效。
    pub ai_settings: Arc<dyn AiSettingsPort>,
}

impl AppState {
    pub fn new(pool: PgPool, agent: Arc<AgentRuntime>, ai_settings: Arc<dyn AiSettingsPort>) -> Self {
        Self {
            pool,
            agent,
            ai_settings,
        }
    }
}
