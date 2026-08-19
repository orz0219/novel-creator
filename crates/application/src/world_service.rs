//! World Service - 世界管理的业务逻辑层
//!
//! 负责 Entity/State/Event/Relation 的创建、查询、更新。
//! 核心原则：AI 只能通过 ProposedChange 提出变更，不能直接修改世界。

use anyhow::{Context, Result};
use chrono::Utc;
use db::repos::{entity_repo, state_repo};
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

/// World Service - 世界管理服务
pub struct WorldService {
    pool: PgPool,
}

impl WorldService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        let repo = db::repos::world_repo::WorldRepo::new(self.pool.clone());
        let world = repo.create(project_id, name, description, world_rules, is_main).await?;
        tracing::info!("Created world: {} (project: {})", name, project_id);
        Ok(world)
    }

    /// 获取世界
    pub async fn get_world(&self, world_id: Uuid) -> Result<Option<World>> {
        let repo = db::repos::world_repo::WorldRepo::new(self.pool.clone());
        repo.get_by_id(world_id).await
    }

    /// 获取项目的主要世界
    pub async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        let repo = db::repos::world_repo::WorldRepo::new(self.pool.clone());
        repo.get_main_world(project_id).await
    }

    /// 确保项目有主要世界
    pub async fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World> {
        let repo = db::repos::world_repo::WorldRepo::new(self.pool.clone());
        repo.ensure_main_world(project_id, project_name).await
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
        // 确保 entity_type 存在
        let type_repo = entity_repo::EntityTypeRepo::new(self.pool.clone());
        let entity_type = type_repo.ensure(entity_type_name, None).await?;

        // 创建实体
        let entity_repo = entity_repo::EntityRepo::new(self.pool.clone());
        let entity = entity_repo
            .create(project_id, world_id, entity_type.id, name, summary, description, attributes)
            .await?;

        tracing::info!("Created entity: {} (type: {}, world: {})", name, entity_type_name, world_id);
        Ok(entity)
    }

    /// 获取实体 (project-scoped)
    pub async fn get_entity(&self, project_id: Uuid, entity_id: Uuid) -> Result<Option<Entity>> {
        let repo = entity_repo::EntityRepo::new(self.pool.clone());
        repo.get_by_id_with_project(project_id, entity_id).await
    }

    /// 列出项目中的所有实体
    pub async fn list_entities(&self, project_id: Uuid) -> Result<Vec<Entity>> {
        let repo = entity_repo::EntityRepo::new(self.pool.clone());
        repo.list_by_project(project_id).await
    }

    /// 按类型列出实体
    pub async fn list_entities_by_type(
        &self,
        project_id: Uuid,
        entity_type_name: &str,
    ) -> Result<Vec<Entity>> {
        let type_repo = entity_repo::EntityTypeRepo::new(self.pool.clone());
        let entity_type = type_repo
            .get_by_name(entity_type_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Entity type not found: {}", entity_type_name))?;

        let repo = entity_repo::EntityRepo::new(self.pool.clone());
        repo.list_by_type(project_id, entity_type.id).await
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
        let repo = entity_repo::RelationRepo::new(self.pool.clone());
        let relation = repo
            .create(
                project_id,
                source_entity_id,
                target_entity_id,
                relation_type,
                description,
                attributes,
            )
            .await?;

        tracing::info!(
            "Created relation: {} --{}--> {}",
            source_entity_id,
            relation_type,
            target_entity_id
        );
        Ok(relation)
    }

    /// 列出实体的所有关系
    pub async fn list_relations(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<Relation>> {
        let repo = entity_repo::RelationRepo::new(self.pool.clone());
        repo.list_by_entity(project_id, entity_id).await
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
        let repo = entity_repo::FactRepo::new(self.pool.clone());
        let fact = repo.create(project_id, content, category, certainty, related_entity_ids).await?;

        tracing::info!("Created fact: {}", content);
        Ok(fact)
    }

    /// 列出项目中的所有事实
    pub async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Fact>> {
        let repo = entity_repo::FactRepo::new(self.pool.clone());
        repo.list_by_project(project_id).await
    }

    // ============================================================
    // State 相关操作
    // ============================================================

    /// 设置实体的当前状态
    ///
    /// P0-2: 使用事务保证 state mutation 和 change record 的原子性。
    /// 所有 Canonical State 写入必须经过事务化路径。
    pub async fn set_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;

        // 先获取当前版本号用于 CAS
        let current = state_repo::StateRepo::get_current_state_tx(
            &mut *tx, project_id, entity_id, state_key
        ).await?;
        let expected_version = current.as_ref().map(|s| s.version);
        let old_value = current.map(|s| s.state_value);

        // 记录状态变更历史（在同一事务中）
        state_repo::StateRepo::record_change_tx(
            &mut *tx,
            project_id,
            None,
            "SET",
            entity_id,
            state_key,
            old_value,
            state_value.clone(),
            Some("system"),
        ).await?;

        // 更新 current_state（在同一事务中）
        let state = state_repo::StateRepo::upsert_state_tx(
            &mut *tx,
            project_id, entity_id, state_key, state_value, expected_version,
        ).await?;

        tx.commit().await.context("Failed to commit transaction")?;

        tracing::info!(
            "Set state: entity={}, key={}, value={}",
            entity_id,
            state_key,
            state.state_value
        );
        Ok(state)
    }

    /// 获取实体的当前状态
    pub async fn get_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        let repo = state_repo::StateRepo::new(self.pool.clone());
        repo.get_current_state(project_id, entity_id, state_key).await
    }

    /// 列出实体的所有当前状态
    pub async fn list_entity_states(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<CurrentState>> {
        let repo = state_repo::StateRepo::new(self.pool.clone());
        repo.list_current_states(project_id, entity_id).await
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
        let repo = state_repo::StateRepo::new(self.pool.clone());
        let resource = repo
            .upsert_resource(
                project_id,
                location_id,
                resource_name,
                quantity,
                production_rate,
                controlled_by,
            )
            .await?;

        tracing::info!(
            "Upserted resource: {} at location {}",
            resource_name,
            location_id
        );
        Ok(resource)
    }

    /// 列出地点的所有资源
    pub async fn list_resources(&self, location_id: Uuid) -> Result<Vec<ResourceState>> {
        let repo = state_repo::StateRepo::new(self.pool.clone());
        repo.list_resources_by_location(location_id).await
    }

    // ============================================================
    // Event 相关操作
    // ============================================================

    /// 记录事件并应用状态变更
    ///
    /// P0-2: 使用事务保证 event + entity relation + state change 的原子性。
    pub async fn record_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        involved_entity_ids: &[Uuid],
        state_changes: Vec<StateChange>,
    ) -> Result<Event> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        let id = Uuid::new_v4();
        let now = Utc::now();

        // 插入事件
        sqlx::query(
            "INSERT INTO event (id, project_id, name, description, event_type, created_at, updated_at)              VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(event_type)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .context("Failed to insert event")?;

        // 关联实体
        for entity_id in involved_entity_ids {
            sqlx::query("INSERT INTO event_entity (id, event_id, entity_id) VALUES ($1, $2, $3)")
                .bind(Uuid::new_v4())
                .bind(id)
                .bind(entity_id)
                .execute(&mut *tx)
                .await
                .context("Failed to insert event_entity")?;
        }

        // 应用状态变更（在同一事务中）
        for change in &state_changes {
            // 获取当前状态
            let current = state_repo::StateRepo::get_current_state_tx(
                &mut *tx, project_id, change.target_entity_id, &change.state_key
            ).await?;
            let expected_version = current.as_ref().map(|s| s.version);
            let old_value = current.map(|s| s.state_value);

            // 记录变更历史
            state_repo::StateRepo::record_change_tx(
                &mut *tx,
                project_id,
                Some(id),
                "EVENT",
                change.target_entity_id,
                &change.state_key,
                old_value,
                change.new_value.clone(),
                Some("event"),
            ).await?;

            // 更新 current_state (with CAS)
            state_repo::StateRepo::upsert_state_tx(
                &mut *tx,
                project_id,
                change.target_entity_id,
                &change.state_key,
                change.new_value.clone(),
                expected_version,
            ).await?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        tracing::info!("Recorded event: {} (changes: {})", name, state_changes.len());
        Ok(Event {
            id,
            project_id,
            name: name.to_string(),
            description: description.to_string(),
            event_type: event_type.map(|s| s.to_string()),
            timestamp: None,
            event_time: None,
            duration: None,
            involved_entity_ids: involved_entity_ids.to_vec(),
            state_changes,
            created_at: now,
            updated_at: now,
        })
    }
}
