//! Agent 记忆（GPT 方案 3.3 的 `agent_memory`）。
//!
//! 端口与实体已上移至 `domain::agent_store`；此处仅保留内存实现（测试 / 无 DB 场景）。

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub use domain::agent_store::{AgentMemory, MemoryItem};
pub use domain::session_summary::{SessionSummary, SessionSummaryPort, StoredSessionSummary};

/// 内存实现（测试 / 无 DB 场景）。
pub struct InMemoryAgentMemory {
    items: RwLock<HashMap<Uuid, Vec<MemoryItem>>>,
}

impl InMemoryAgentMemory {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryAgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentMemory for InMemoryAgentMemory {
    async fn save(&self, project_id: Uuid, memory_type: &str, content: &str) -> Result<()> {
        if let Ok(mut g) = self.items.write() {
            g.entry(project_id).or_default().push(MemoryItem {
                memory_type: memory_type.to_string(),
                content: content.to_string(),
                created_at: chrono::Utc::now(),
            });
        }
        Ok(())
    }

    async fn list(&self, project_id: Uuid) -> Result<Vec<MemoryItem>> {
        Ok(self
            .items
            .read()
            .map(|g| g.get(&project_id).and_then(|v| Some(v.clone())).unwrap_or_default())
            .unwrap_or_default())
    }

    async fn replace_by_type(
        &self,
        project_id: Uuid,
        memory_type: &str,
        content: &str,
    ) -> Result<()> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| anyhow::anyhow!("记忆存储锁已损坏（替换）"))?;
        let items = guard.entry(project_id).or_default();
        items.retain(|m| m.memory_type != memory_type);
        items.push(MemoryItem {
            memory_type: memory_type.to_string(),
            content: content.to_string(),
            created_at: chrono::Utc::now(),
        });
        Ok(())
    }
}

/// 滚动摘要的内存实现（测试 / 无 DB 场景）。
///
/// 与 `InMemoryAgentMemory` 一样，**每个项目恒定一份**：再次保存是覆盖。
pub struct InMemorySessionSummary {
    items: RwLock<HashMap<Uuid, StoredSessionSummary>>,
}

impl InMemorySessionSummary {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionSummaryPort for InMemorySessionSummary {
    async fn load(&self, project_id: Uuid) -> Result<Option<StoredSessionSummary>> {
        let guard = self
            .items
            .read()
            .map_err(|_| anyhow::anyhow!("会话摘要存储锁已损坏（读）"))?;
        Ok(guard.get(&project_id).cloned())
    }

    async fn save(
        &self,
        project_id: Uuid,
        summary: &SessionSummary,
    ) -> Result<StoredSessionSummary> {
        let mut guard = self
            .items
            .write()
            .map_err(|_| anyhow::anyhow!("会话摘要存储锁已损坏（写）"))?;
        let record = StoredSessionSummary {
            content: summary.clone(),
            updated_at: chrono::Utc::now(),
        };
        // 覆盖写：每个项目只保留最新那一份
        guard.insert(project_id, record.clone());
        Ok(record)
    }
}
