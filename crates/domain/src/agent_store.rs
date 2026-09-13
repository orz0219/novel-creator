//! Agent 会话与记忆的端口及实体类型。
//!
//! 原本这些类型与 trait 定义在 `agent` crate（内存实现）。为支撑持久化，
//! 将端口与实体上移到 `domain`（与 `PromptRepositoryPort` 一致），
//! 由 `db` crate 提供 Postgres 实现，`agent` 仅保留内存实现。

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 单条对话消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// "user" | "assistant" | "system"
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// 一次 Agent 引导会话。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: Uuid,
    /// 所属项目（NOT NULL）：会话必须绑定到一个项目（对话-项目绑定，P2）。
    pub project_id: Uuid,
    pub title: Option<String>,
    pub messages: Vec<ChatMessage>,
    /// 当前引导阶段（对应 GUIDE.md 8 步；P1 仅记录，Workflow 引擎在 P2/P3 驱动）。
    pub current_step: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentSession {
    pub fn new(project_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            title: None,
            messages: Vec::new(),
            // 会话级字段仅为兼容保留；**引导阶段的真源是 project.config.current_step**。
            // 这里与 guide 的初始步骤 key 保持一致，避免出现不在步骤清单里的值。
            current_step: "premise".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 会话存储端口。
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, session: AgentSession) -> Result<()>;
    async fn get(&self, id: Uuid) -> Result<Option<AgentSession>>;
    async fn update(&self, session: AgentSession) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    /// 列出某项目下的全部会话（按项目隔离，用于历史侧栏）。
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<AgentSession>>;
}

/// 一条记忆项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 用户偏好 / 故事风格 / 重要设定
    pub memory_type: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// 记忆端口（项目级：记忆绑定到某个项目，同一项目下多个会话共享，跨刷新保留）。
#[async_trait]
pub trait AgentMemory: Send + Sync {
    async fn save(&self, project_id: Uuid, memory_type: &str, content: &str) -> Result<()>;
    async fn list(&self, project_id: Uuid) -> Result<Vec<MemoryItem>>;
}
