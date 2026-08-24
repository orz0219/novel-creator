//! 内存版 PromptRepository（测试与无 DB 场景使用）。
//!
//! 生产环境由 `crates/db` 的 SQL 实现接管；此处仅供单元测试与本地无依赖运行。

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;

use domain::ports::{AgentPromptConfig, PromptRepositoryPort};

/// 内存提示词存储：按 scope 幂等保存。
pub struct InMemoryPromptRepo {
    store: RwLock<HashMap<String, AgentPromptConfig>>,
}

impl InMemoryPromptRepo {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryPromptRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PromptRepositoryPort for InMemoryPromptRepo {
    async fn load(&self, scope: &str) -> Result<Option<AgentPromptConfig>> {
        Ok(self.store.read().unwrap().get(scope).cloned())
    }

    async fn save(&self, config: &AgentPromptConfig) -> Result<()> {
        let mut g = self.store.write().unwrap();
        g.insert(
            config.scope.clone(),
            AgentPromptConfig {
                id: config.id,
                scope: config.scope.clone(),
                project_id: config.project_id,
                system_prompt: config.system_prompt.clone(),
                updated_at: config.updated_at,
            },
        );
        Ok(())
    }

    async fn delete(&self, scope: &str) -> Result<()> {
        self.store.write().unwrap().remove(scope);
        Ok(())
    }
}
