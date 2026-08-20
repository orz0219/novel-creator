//! World Service - 世界管理的业务逻辑层
//!
//! 负责 Entity/State/Event/Relation 的创建、查询、更新。
//! 通过 WorldRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! 具体 SQL 与事务实现已下沉到 db crate 的 port 实现。
//!
//! 核心原则：AI 只能通过 ProposedChange 提出变更，不能直接修改世界。

use anyhow::Result;
use domain::entity::{Entity, Fact, Relation, StateChange};
use domain::ports::WorldRepositoryPort;
use domain::state::{CurrentState, ResourceState};
use domain::world::World;
use std::sync::Arc;
use uuid::Uuid;

/// World Service - 世界管理服务
pub struct WorldService {
    repo: Arc<dyn WorldRepositoryPort>,
}

impl WorldService {
    pub fn new(repo: Arc<dyn WorldRepositoryPort>) -> Self {
        Self { repo }
    }

    // ============================================================
    // World 相关操作
    // ============================================================

    /// 创建世界
    pub async fn create_world(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World> {
        self.repo
            .create_world(project_id, name, description, world_rules, is_main)
            .await
    }

    /// 获取世界
    pub async fn get_world(&self, world_id: Uuid) -> Result<Option<World>> {
        self.repo.get_world(world_id).await
    }

    /// 获取项目的主要世界
    pub async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        self.repo.get_main_world(project_id).await
    }

    /// 确保项目有主要世界
    pub async fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World> {
        self.repo.ensure_main_world(project_id, project_name).await
    }

    /// 获取项目的主要世界；若不存在则自动创建（host 层 get_world 语义）。
    pub async fn get_or_create_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        self.repo.get_or_create_main_world(project_id).await
    }

    /// 更新主要世界的基础字段（name / description / world_rules）。
    pub async fn update_main_world(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        world_rules: Option<&str>,
    ) -> Result<World> {
        self.repo
            .update_main_world(project_id, name, description, world_rules)
            .await
    }

    // ============================================================
    // Entity 相关操作
    // ============================================================

    /// 创建世界实体
    pub async fn create_entity(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Entity> {
        self.repo
            .create_entity(
                project_id,
                world_id,
                entity_type_name,
                name,
                summary,
                description,
                attributes,
            )
            .await
    }

    /// 获取实体 (project-scoped)
    pub async fn get_entity(&self, project_id: Uuid, entity_id: Uuid) -> Result<Option<Entity>> {
        self.repo.get_entity(project_id, entity_id).await
    }

    /// 列出项目中的所有实体
    pub async fn list_entities(&self, project_id: Uuid) -> Result<Vec<Entity>> {
        self.repo.list_entities(project_id).await
    }

    /// 按类型列出实体
    pub async fn list_entities_by_type(
        &self,
        project_id: Uuid,
        entity_type_name: &str,
    ) -> Result<Vec<Entity>> {
        self.repo.list_entities_by_type(project_id, entity_type_name).await
    }

    /// 创建两个实体之间的关系
    pub async fn create_relation(
        &self,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Relation> {
        self.repo
            .create_relation(
                project_id,
                source_entity_id,
                target_entity_id,
                relation_type,
                description,
                attributes,
            )
            .await
    }

    /// 列出实体的所有关系
    pub async fn list_relations(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<Relation>> {
        self.repo.list_relations(project_id, entity_id).await
    }

    // ============================================================
    // Fact 相关操作
    // ============================================================

    /// 创建世界事实
    pub async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact> {
        self.repo
            .create_fact(project_id, content, category, certainty, related_entity_ids)
            .await
    }

    /// 列出项目中的所有事实
    pub async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Fact>> {
        self.repo.list_facts(project_id).await
    }

    // ============================================================
    // State 相关操作
    // ============================================================

    /// 设置实体的当前状态（事务化，port 实现保证原子性）
    pub async fn set_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState> {
        self.repo
            .set_entity_state(project_id, entity_id, state_key, state_value)
            .await
    }

    /// 获取实体的当前状态
    pub async fn get_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        self.repo
            .get_entity_state(project_id, entity_id, state_key)
            .await
    }

    /// 列出实体的所有当前状态
    pub async fn list_entity_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>> {
        self.repo.list_entity_states(project_id, entity_id).await
    }

    // ============================================================
    // Resource 相关操作
    // ============================================================

    /// 创建或更新资源状态
    pub async fn upsert_resource(
        &self,
        project_id: Uuid,
        location_id: Uuid,
        resource_name: &str,
        quantity: Option<f64>,
        production_rate: Option<f64>,
        controlled_by: Option<Uuid>,
    ) -> Result<ResourceState> {
        self.repo
            .upsert_resource(
                project_id,
                location_id,
                resource_name,
                quantity,
                production_rate,
                controlled_by,
            )
            .await
    }

    /// 列出地点的所有资源
    pub async fn list_resources(&self, location_id: Uuid) -> Result<Vec<ResourceState>> {
        self.repo.list_resources(location_id).await
    }

    // ============================================================
    // Event 相关操作
    // ============================================================

    /// 记录事件并应用状态变更（事务化，port 实现保证原子性）
    pub async fn record_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        involved_entity_ids: &[Uuid],
        state_changes: Vec<StateChange>,
    ) -> Result<domain::entity::Event> {
        self.repo
            .record_event(
                project_id,
                name,
                description,
                event_type,
                involved_entity_ids,
                state_changes,
            )
            .await
    }
}
