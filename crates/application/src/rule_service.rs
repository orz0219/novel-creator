//! Rule Service - canon_rule 规则的业务逻辑层。
//!
//! 通过 RuleRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::RuleRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Rule Service - 规则服务
pub struct RuleService {
    repo: Arc<dyn RuleRepositoryPort>,
}

impl RuleService {
    pub fn new(repo: Arc<dyn RuleRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_rules(&self, world_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_rules(world_id).await
    }

    pub async fn create_rule(
        &self,
        world_id: Uuid,
        rule_content: &str,
        rule_level: Option<&str>,
        affected_scope: Option<&str>,
        enforcement: Option<&str>,
    ) -> Result<Value> {
        self.repo
            .create_rule(world_id, rule_content, rule_level, affected_scope, enforcement)
            .await
    }

    pub async fn get_rule(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_rule(id).await
    }

    pub async fn update_rule(
        &self,
        id: Uuid,
        rule_content: Option<&str>,
        rule_level: Option<&str>,
    ) -> Result<Value> {
        self.repo.update_rule(id, rule_content, rule_level).await
    }

    pub async fn delete_rule(&self, id: Uuid) -> Result<()> {
        self.repo.delete_rule(id).await
    }
}
