//! World Service - 世界管理的业务逻辑层
//!
//! 负责 Entity/State/Event/Relation 的创建、查询、更新。
//! 核心原则：AI 只能通过 ProposedChange 提出变更，不能直接修改世界。

use anyhow::{Context, Result};
use chrono::Utc;
use db::connection::Database;
use db::repos::{entity_repo, state_repo};
use domain::*;
use uuid::Uuid;

/// World Service - 世界管理服务
pub struct WorldService<'a> {
    db: &'a Database,
}

impl<'a> WorldService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    // ============================================================
    // World 相关操作
    // ============================================================

    /// 创建世界
    pub fn create_world(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World> {
        let repo = db::repos::world_repo::WorldRepo::new(self.db);
        let world = repo.create(project_id, name, description, world_rules, is_main)?;
        tracing::info!("Created world: {} (project: {})", name, project_id);
        Ok(world)
    }

    /// 获取世界
    pub fn get_world(&self, world_id: Uuid) -> Result<Option<World>> {
        let repo = db::repos::world_repo::WorldRepo::new(self.db);
        repo.get_by_id(world_id)
    }

    /// 获取项目的主要世界
    pub fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        let repo = db::repos::world_repo::WorldRepo::new(self.db);
        repo.get_main_world(project_id)
    }

    /// 确保项目有主要世界
    pub fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World> {
        let repo = db::repos::world_repo::WorldRepo::new(self.db);
        repo.ensure_main_world(project_id, project_name)
    }

    // ============================================================
    // Entity 相关操作
    // ============================================================

    /// 创建世界实体
    pub fn create_entity(
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
        let type_repo = entity_repo::EntityTypeRepo::new(self.db);
        let entity_type = type_repo.ensure(entity_type_name, None)?;

        // 创建实体
        let entity_repo = entity_repo::EntityRepo::new(self.db);
        let entity = entity_repo.create(project_id, world_id, entity_type.id, name, summary, description, attributes)?;

        tracing::info!("Created entity: {} (type: {}, world: {})", name, entity_type_name, world_id);
        Ok(entity)
    }

    /// 获取实体
    pub fn get_entity(&self, entity_id: Uuid) -> Result<Option<Entity>> {
        let repo = entity_repo::EntityRepo::new(self.db);
        repo.get_by_id(entity_id)
    }

    /// 列出项目中的所有实体
    pub fn list_entities(&self, project_id: Uuid) -> Result<Vec<Entity>> {
        let repo = entity_repo::EntityRepo::new(self.db);
        repo.list_by_project(project_id)
    }

    /// 按类型列出实体
    pub fn list_entities_by_type(
        &self,
        project_id: Uuid,
        entity_type_name: &str,
    ) -> Result<Vec<Entity>> {
        let type_repo = entity_repo::EntityTypeRepo::new(self.db);
        let entity_type = type_repo
            .get_by_name(entity_type_name)?
            .ok_or_else(|| anyhow::anyhow!("Entity type not found: {}", entity_type_name))?;

        let repo = entity_repo::EntityRepo::new(self.db);
        repo.list_by_type(project_id, entity_type.id)
    }

    /// 创建两个实体之间的关系
    pub fn create_relation(
        &self,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Relation> {
        let repo = entity_repo::RelationRepo::new(self.db);
        let relation = repo.create(
            project_id,
            source_entity_id,
            target_entity_id,
            relation_type,
            description,
            attributes,
        )?;

        tracing::info!(
            "Created relation: {} --{}--> {}",
            source_entity_id,
            relation_type,
            target_entity_id
        );
        Ok(relation)
    }

    /// 列出实体的所有关系
    pub fn list_relations(&self, entity_id: Uuid) -> Result<Vec<Relation>> {
        let repo = entity_repo::RelationRepo::new(self.db);
        repo.list_by_entity(entity_id)
    }

    // ============================================================
    // Fact 相关操作
    // ============================================================

    /// 创建世界事实
    pub fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact> {
        let repo = entity_repo::FactRepo::new(self.db);
        let fact = repo.create(project_id, content, category, certainty, related_entity_ids)?;

        tracing::info!("Created fact: {}", content);
        Ok(fact)
    }

    /// 列出项目中的所有事实
    pub fn list_facts(&self, project_id: Uuid) -> Result<Vec<Fact>> {
        let repo = entity_repo::FactRepo::new(self.db);
        repo.list_by_project(project_id)
    }

    // ============================================================
    // State 相关操作
    // ============================================================

    /// 设置实体的当前状态
    pub fn set_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState> {
        let repo = state_repo::StateRepo::new(self.db);
        let state = repo.upsert_state(project_id, entity_id, state_key, state_value.clone())?;

        // 同时记录状态变更历史
        repo.record_change(
            project_id,
            None,
            "SET",
            entity_id,
            state_key,
            None,
            state_value,
            Some("system"),
        )?;

        tracing::info!(
            "Set state: entity={}, key={}, value={}",
            entity_id,
            state_key,
            state.state_value
        );
        Ok(state)
    }

    /// 获取实体的当前状态
    pub fn get_entity_state(
        &self,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>> {
        let repo = state_repo::StateRepo::new(self.db);
        repo.get_current_state(entity_id, state_key)
    }

    /// 列出实体的所有当前状态
    pub fn list_entity_states(&self, entity_id: Uuid) -> Result<Vec<CurrentState>> {
        let repo = state_repo::StateRepo::new(self.db);
        repo.list_current_states(entity_id)
    }

    // ============================================================
    // Resource 相关操作
    // ============================================================

    /// 创建或更新资源状态
    pub fn upsert_resource(
        &self,
        project_id: Uuid,
        location_id: Uuid,
        resource_name: &str,
        quantity: Option<f64>,
        production_rate: Option<f64>,
        controlled_by: Option<Uuid>,
    ) -> Result<ResourceState> {
        let repo = state_repo::StateRepo::new(self.db);
        let resource = repo.upsert_resource(
            project_id,
            location_id,
            resource_name,
            quantity,
            production_rate,
            controlled_by,
        )?;

        tracing::info!(
            "Upserted resource: {} at location {}",
            resource_name,
            location_id
        );
        Ok(resource)
    }

    /// 列出地点的所有资源
    pub fn list_resources(&self, location_id: Uuid) -> Result<Vec<ResourceState>> {
        let repo = state_repo::StateRepo::new(self.db);
        repo.list_resources_by_location(location_id)
    }

    // ============================================================
    // Event 相关操作
    // ============================================================

    /// 记录事件并应用状态变更
    pub fn record_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        involved_entity_ids: &[Uuid],
        state_changes: Vec<StateChange>,
    ) -> Result<Event> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // 插入事件
        {
            let conn = self.db.conn();
            conn.execute(
                "INSERT INTO event (id, project_id, name, description, event_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                [id.to_string(), project_id.to_string(), name.to_string(), description.to_string(), event_type.unwrap_or("").to_string(), now.to_string(), now.to_string()],
            ).context("Failed to insert event")?;

            // 关联实体
            for entity_id in involved_entity_ids {
                conn.execute(
                    "INSERT INTO event_entity (id, event_id, entity_id) VALUES (?, ?, ?)",
                    [Uuid::new_v4().to_string(), id.to_string(), entity_id.to_string()],
                ).context("Failed to insert event_entity")?;
            }
        }

        // 应用状态变更
        let state_repo = state_repo::StateRepo::new(self.db);
        for change in &state_changes {
            state_repo.record_change(
                project_id,
                Some(id),
                &serde_json::to_string(&change.change_type).unwrap_or_default(),
                change.target_entity_id,
                &change.state_key,
                change.old_value.clone(),
                change.new_value.clone(),
                Some("event"),
            )?;

            // 更新 current_state
            state_repo.upsert_state(
                project_id,
                change.target_entity_id,
                &change.state_key,
                change.new_value.clone(),
            )?;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        // 从 db crate 的 manifest 目录找 migrations
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_dir = format!("{}/../db/migrations", manifest_dir);
        db::migration::run_migrations(&db, &migrations_dir).unwrap();
        db
    }

    fn create_test_project(db: &Database) -> Uuid {
        let repo = db::repos::project_repo::ProjectRepo::new(db);
        repo.create("Test Novel", None).unwrap().id
    }

    fn create_test_world(db: &Database, project_id: Uuid) -> Uuid {
        let service = WorldService::new(db);
        service.ensure_main_world(project_id, "Test Novel").unwrap().id
    }

    #[test]
    fn test_create_world() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let service = WorldService::new(&db);

        let world = service.create_world(project_id, "Main World", Some("A fantasy world"), None, true).unwrap();
        assert_eq!(world.name, "Main World");
        assert!(world.is_main);

        let fetched = service.get_world(world.id).unwrap().unwrap();
        assert_eq!(fetched.id, world.id);
    }

    #[test]
    fn test_create_entity() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = WorldService::new(&db);

        let entity = service
            .create_entity(
                project_id,
                world_id,
                "Character",
                "Lin Fan",
                Some("A young cultivator"),
                Some("Cautious and observant"),
                serde_json::json!({"age": 20, "cultivation": "Qi Refining Level 3"}),
            )
            .unwrap();

        assert_eq!(entity.name, "Lin Fan");

        let fetched = service.get_entity(entity.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Lin Fan");
    }

    #[test]
    fn test_create_relation() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = WorldService::new(&db);

        let e1 = service
            .create_entity(project_id, world_id, "Character", "A", None, None, serde_json::json!({}))
            .unwrap();
        let e2 = service
            .create_entity(project_id, world_id, "Character", "B", None, None, serde_json::json!({}))
            .unwrap();

        let rel = service
            .create_relation(project_id, e1.id, e2.id, "friend", Some("old friends"), serde_json::json!({}))
            .unwrap();
        assert_eq!(rel.relation_type, "friend");

        let rels = service.list_relations(e1.id).unwrap();
        assert_eq!(rels.len(), 1);
    }

    #[test]
    fn test_set_entity_state() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = WorldService::new(&db);

        let entity = service
            .create_entity(project_id, world_id, "Character", "Lin Fan", None, None, serde_json::json!({}))
            .unwrap();

        service
            .set_entity_state(project_id, entity.id, "location", serde_json::json!("Black Stone City"))
            .unwrap();

        let state = service.get_entity_state(entity.id, "location").unwrap().unwrap();
        assert_eq!(state.state_value, serde_json::json!("Black Stone City"));
    }

    #[test]
    fn test_record_event_with_state_changes() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let service = WorldService::new(&db);

        let entity = service
            .create_entity(project_id, world_id, "Character", "Lin Fan", None, None, serde_json::json!({}))
            .unwrap();

        let event = service
            .record_event(
                project_id,
                "Lin Fan enters the city",
                "Lin Fan arrives at Black Stone City",
                Some("movement"),
                &[entity.id],
                vec![StateChange {
                    change_type: StateChangeType::LocationChange,
                    target_entity_id: entity.id,
                    state_key: "location".to_string(),
                    old_value: Some(serde_json::json!("outside")),
                    new_value: serde_json::json!("Black Stone City"),
                }],
            )
            .unwrap();

        assert_eq!(event.name, "Lin Fan enters the city");

        // 验证状态已更新
        let state = service.get_entity_state(entity.id, "location").unwrap().unwrap();
        assert_eq!(state.state_value, serde_json::json!("Black Stone City"));
    }
}
