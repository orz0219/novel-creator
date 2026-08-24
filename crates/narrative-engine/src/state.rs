//! Application State

use std::sync::Arc;

use sqlx::PgPool;

use agent::AgentRuntime;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// P1 引导式 Agent 运行时（会话 / 工具 / 记忆，内存实现）。
    pub agent: Arc<AgentRuntime>,
}

impl AppState {
    pub fn new(pool: PgPool, agent: Arc<AgentRuntime>) -> Self {
        Self { pool, agent }
    }
}
