//! Entity Service - 实体 / 关系 / 角色子数据的业务逻辑层。
//!
//! 读操作（list/get/character 子数据）通过 EntityRepositoryPort 完成。
//! 写操作（create/update/delete 实体）统一经由 MutationCommitter 提交，
//! 不再直接调用 repo 做 Canon mutation（提案 四 / 二十四）。
//!
//! 关系与事实的语义化写入（EndRelation / SupersedeFact 等）在后续阶段接入。

use anyhow::Result;
use domain::mutation::MutationCommand;
use domain::ports::{EntityRepositoryPort, ProjectResolverPort};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::mutation::MutationCommitter;

/// Entity Service - 实体服务
pub struct EntityService {
    repo: Arc<dyn EntityRepositoryPort>,
    committer: Arc<MutationCommitter>,
    resolver: Arc<dyn ProjectResolverPort>,
}

impl EntityService {
    pub fn new(
        repo: Arc<dyn EntityRepositoryPort>,
        committer: Arc<MutationCommitter>,
        resolver: Arc<dyn ProjectResolverPort>,
    ) -> Self {
        Self {
            repo,
            committer,
            resolver,
        }
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

    /// 创建实体：经由 MutationCommitter（提案 四）。
    pub async fn create_entity(
        &self,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value> {
        let project_id = self
            .resolver
            .project_id_for_world(world_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for world {}", world_id))?;

        let cmd = MutationCommand::create_entity(
            project_id,
            world_id,
            entity_type_name,
            name,
            summary,
            description,
        );
        // 实体创建属于「影响该世界的 Canon 写」：显式声明 affected_worlds，
        // 让提交者在同一事务内推进这个世界的 world_version（ChatGPT 评审 P2/B）。
        let results = self
            .committer
            .commit_with_worlds(cmd, vec![world_id])
            .await?;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("mutation returned no result"))?;
        let entity_id = *result
            .created_ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("mutation did not return created entity"))?;
        self.repo
            .get_entity(entity_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("created entity {} not found", entity_id))
    }

    /// 更新实体：经由 MutationCommitter（提案 四）。带乐观锁。
    pub async fn update_entity(
        &self,
        id: Uuid,
        name: Option<&str>,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: Option<&Value>,
    ) -> Result<Value> {
        let existing = self
            .repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} not found", id))?;
        let project_id = extract_project_id(&existing)?;
        let expected_version = extract_version(&existing)?;

        let cmd = MutationCommand::update_entity(
            project_id,
            id,
            Some(expected_version),
            name.map(|s| s.to_string()),
            summary.map(|s| s.to_string()),
            description.map(|s| s.to_string()),
            attributes.map(|v| v.clone()),
        );
        self.committer.commit(cmd).await?;
        self.repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} disappeared after update", id))
    }

    /// 删除实体：经由 MutationCommitter（语义化软删除，绝不物理 DELETE）。
    pub async fn delete_entity(&self, id: Uuid) -> Result<Value> {
        let existing = self
            .repo
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("entity {} not found", id))?;
        let project_id = extract_project_id(&existing)?;
        let expected_version = extract_version(&existing)?;

        let cmd = MutationCommand::delete_entity(project_id, id, expected_version);
        self.committer.commit(cmd).await?;
        Ok(existing)
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

    /// 删除关系：语义化结束（EndRelation），绝不物理 DELETE（提案 五）。
    pub async fn delete_relation(&self, id: Uuid) -> Result<()> {
        let project_id = self
            .resolver
            .project_id_for_relation(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for relation {}", id))?;
        let cmd = MutationCommand::end_relation(project_id, id, None);
        self.committer.commit(cmd).await?;
        Ok(())
    }

    pub async fn get_character_profile(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_character_profile(id).await
    }

    pub async fn get_character_state(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_character_state(id).await
    }

    pub async fn update_character_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        self.repo.update_character_profile(id, profile).await
    }

    pub async fn update_character_state(&self, id: Uuid, state: Value) -> Result<Value> {
        self.repo.update_character_state(id, state).await
    }

    pub async fn get_location_profile(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_location_profile(id).await
    }

    pub async fn upsert_location_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        self.repo.upsert_location_profile(id, profile).await
    }

    pub async fn get_faction_profile(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_faction_profile(id).await
    }

    pub async fn upsert_faction_profile(&self, id: Uuid, profile: Value) -> Result<Value> {
        self.repo.upsert_faction_profile(id, profile).await
    }

    pub async fn get_character_knowledge(&self, id: Uuid) -> Result<Vec<Value>> {
        self.repo.get_character_knowledge(id).await
    }

    pub async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<Value>> {
        self.repo.get_character_relationships(id).await
    }
}

fn extract_project_id(entity: &Value) -> Result<Uuid> {
    entity
        .get("project_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("entity missing project_id"))
}

fn extract_version(entity: &Value) -> Result<i32> {
    entity
        .get("version")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .ok_or_else(|| anyhow::anyhow!("entity missing version"))
}
