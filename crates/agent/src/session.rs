//! Agent 会话与状态管理（GPT 方案 3.3 的 `agent_sessions`）。
//!
//! 端口与实体已上移至 `domain::agent_store`；此处仅保留内存实现（测试 / 无 DB 场景）。

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub use domain::agent_store::{AgentSession, ChatMessage, SessionStore};

/// 内存实现（测试 / 无 DB 场景）。
pub struct InMemorySessionStore {
    sessions: RwLock<HashMap<Uuid, AgentSession>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self, session: AgentSession) -> Result<()> {
        if let Ok(mut g) = self.sessions.write() {
            g.insert(session.id, session);
        }
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<AgentSession>> {
        Ok(self.sessions.read().ok().and_then(|g| g.get(&id).cloned()))
    }

    async fn update(&self, session: AgentSession) -> Result<()> {
        if let Ok(mut g) = self.sessions.write() {
            g.insert(session.id, session);
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        if let Ok(mut g) = self.sessions.write() {
            g.remove(&id);
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<AgentSession>> {
        Ok(self
            .sessions
            .read()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default())
    }
}
