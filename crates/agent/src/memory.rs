//! Agent 记忆（GPT 方案 3.3 的 `agent_memory`）。
//!
//! 端口与实体已上移至 `domain::agent_store`；此处仅保留内存实现（测试 / 无 DB 场景）。

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub use domain::agent_store::{AgentMemory, MemoryItem};

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
}
