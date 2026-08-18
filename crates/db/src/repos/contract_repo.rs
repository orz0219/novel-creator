//! Contract Repository - CRUD operations for SceneContract

use anyhow::{Context, Result};
use chrono::Utc;
use domain::SceneContract;
use uuid::Uuid;

use crate::connection::Database;

pub struct ContractRepo<'a> {
    db: &'a Database,
}

impl<'a> ContractRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建场景契约
    pub fn create(&self, scene_id: Uuid, required_events: Vec<String>, forbidden_events: Vec<String>, required_characters: Vec<Uuid>, required_facts: Vec<String>, reader_learns: Vec<String>, protagonist_learns: Vec<String>, world_changes: Vec<String>) -> Result<SceneContract> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO scene_contract (id, scene_id, required_events, forbidden_events, required_characters, required_facts, reader_learns, protagonist_learns, world_changes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                id.to_string(), scene_id.to_string(),
                serde_json::to_string(&required_events).unwrap_or_default(),
                serde_json::to_string(&forbidden_events).unwrap_or_default(),
                serde_json::to_string(&required_characters).unwrap_or_default(),
                serde_json::to_string(&required_facts).unwrap_or_default(),
                serde_json::to_string(&reader_learns).unwrap_or_default(),
                serde_json::to_string(&protagonist_learns).unwrap_or_default(),
                serde_json::to_string(&world_changes).unwrap_or_default(),
                now.to_string(), now.to_string(),
            ],
        ).context("Failed to create scene contract")?;
        Ok(SceneContract { id, scene_id, required_events, forbidden_events, required_characters, required_facts, reader_learns, protagonist_learns, world_changes, created_at: now, updated_at: now })
    }

    /// 按场景获取契约
    pub fn get_by_scene(&self, scene_id: Uuid) -> Result<Option<SceneContract>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, scene_id, required_events, forbidden_events, required_characters, required_facts, reader_learns, protagonist_learns, world_changes, created_at, updated_at FROM scene_contract WHERE scene_id = ?",
            [scene_id.to_string()],
            |row| {
                let parse_vec = |s: Option<String>| -> Vec<String> {
                    s.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
                };
                let parse_uuid_vec = |s: Option<String>| -> Vec<Uuid> {
                    s.and_then(|s| serde_json::from_str::<Vec<Uuid>>(&s).ok()).unwrap_or_default()
                };
                Ok(SceneContract {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    scene_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    required_events: parse_vec(row.get(2)?),
                    forbidden_events: parse_vec(row.get(3)?),
                    required_characters: parse_uuid_vec(row.get(4)?),
                    required_facts: parse_vec(row.get(5)?),
                    reader_learns: parse_vec(row.get(6)?),
                    protagonist_learns: parse_vec(row.get(7)?),
                    world_changes: parse_vec(row.get(8)?),
                    created_at: crate::time_utils::get_timestamp(row, 9),
                    updated_at: crate::time_utils::get_timestamp(row, 10),
                })
            },
        ).ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        db
    }

    #[test]
    fn test_create_contract() {
        let db = setup_db();
        let repo = ContractRepo::new(&db);
        let scene_id = Uuid::new_v4();
        let contract = repo.create(scene_id, vec!["进入黑市".into()], vec!["发现遗迹".into()], vec![], vec![], vec!["黑市存在王家眼线".into()], vec!["王家正在调查自己".into()], vec!["获得通行资格".into()]).unwrap();
        assert_eq!(contract.scene_id, scene_id);
        assert_eq!(contract.required_events.len(), 1);
        assert_eq!(contract.forbidden_events.len(), 1);
    }
}
