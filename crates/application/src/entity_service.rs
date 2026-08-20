//! Entity Service - 实体 / 关系 / 角色子数据的业务逻辑层。
//!
//! 通过 EntityRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! 具体 SQL 与 project-scope 校验已下沉到 db crate 的 port 实现。

use anyhow::Result;
use domain::ports::EntityRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Entity Service - 实体服务
pub struct EntityService {
    repo: Arc<dyn EntityRepositoryPort>,
}

impl EntityService {
    pub fn new(repo: Arc<dyn EntityRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_entities(
        &self,
        world_id: Uuid,
        entity_type: Option<&str>,
    ) -> Result<Vec<Value>> {
        self.repo.list_entities(world_id, entity_type).await
    }

    pub async fn get_entity(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_entity(id).await
    }

    pub async fn create_entity(
        &self,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value> {
        self.repo
            .create_entity(world_id, entity_type_name, name, summary, description)
            .await
    }

    pub async fn update_entity(
        &self,
        id: Uuid,
        name: Option<&str>,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value> {
        self.repo.update_entity(id, name, summary, description).await
    }

    pub async fn delete_entity(&self, id: Uuid) -> Result<Value> {
        self.repo.delete_entity(id).await
    }

    pub async fn list_relations(&self, world_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_relations(world_id).await
    }

    pub async fn create_relation(
        &self,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        self.repo
            .create_relation(source_entity_id, target_entity_id, relation_type, description)
            .await
    }

    pub async fn delete_relation(&self, id: Uuid) -> Result<()> {
        self.repo.delete_relation(id).await
    }

    pub async fn get_character_profile(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_character_profile(id).await
    }

    pub async fn get_character_state(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_character_state(id).await
    }

    pub async fn get_character_knowledge(&self, id: Uuid) -> Result<Vec<Value>> {
        self.repo.get_character_knowledge(id).await
    }

    pub async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<Value>> {
        self.repo.get_character_relationships(id).await
    }
}
