//! NarrativeState Repository - 叙事状态 CRUD

use anyhow::{Context, Result};
use chrono::Utc;
use domain::character_mind::{NarrativeState, StateDimension};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct NarrativeStateRepo<'a> {
    db: &'a Database,
}

impl<'a> NarrativeStateRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, dimension: StateDimension, state_key: &str, state_value: serde_json::Value, scene_id: Option<Uuid>) -> Result<NarrativeState> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO narrative_state (id, project_id, state_dimension, state_key, state_value, scene_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), dimension.as_str().to_string(), state_key.to_string(), state_value.to_string(), scene_id.map(|s| s.to_string()).unwrap_or_default(), now.to_rfc3339(), now.to_rfc3339()],
        ).context("Failed to insert narrative_state")?;
        Ok(NarrativeState { id, project_id, state_dimension: dimension, state_key: state_key.to_string(), state_value, scene_id, created_at: now, updated_at: now })
    }

    pub fn list_by_dimension(&self, project_id: Uuid, dimension: StateDimension) -> Result<Vec<NarrativeState>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, state_dimension, state_key, state_value, scene_id, created_at, updated_at FROM narrative_state WHERE project_id = ? AND state_dimension = ?").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string(), dimension.as_str().to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let state_dimension: String = row.get(2)?;
            let state_key: String = row.get(3)?;
            let state_value_str: String = row.get(4)?;
            let scene_id: Option<String> = row.get(5)?;
            Ok(NarrativeState {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                state_dimension: StateDimension::from_str(&state_dimension),
                state_key,
                state_value: serde_json::from_str(&state_value_str).unwrap_or(serde_json::json!(null)),
                scene_id: scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                created_at: get_timestamp(row, 6),
                updated_at: get_timestamp(row, 7),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_by_key(&self, project_id: Uuid, dimension: StateDimension, state_key: &str) -> Result<Option<NarrativeState>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, state_dimension, state_key, state_value, scene_id, created_at, updated_at FROM narrative_state WHERE project_id = ? AND state_dimension = ? AND state_key = ? ORDER BY updated_at DESC LIMIT 1",
            [project_id.to_string(), dimension.as_str().to_string(), state_key.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let project_id: String = row.get(1)?;
                let state_dimension: String = row.get(2)?;
                let state_key: String = row.get(3)?;
                let state_value_str: String = row.get(4)?;
                let scene_id: Option<String> = row.get(5)?;
                Ok(NarrativeState {
                    id: Uuid::parse_str(&id).unwrap(),
                    project_id: Uuid::parse_str(&project_id).unwrap(),
                    state_dimension: StateDimension::from_str(&state_dimension),
                    state_key,
                    state_value: serde_json::from_str(&state_value_str).unwrap_or(serde_json::json!(null)),
                    scene_id: scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    created_at: get_timestamp(row, 6),
                    updated_at: get_timestamp(row, 7),
                })
            },
        ).ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;

    #[test]
    fn test_narrative_state_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = NarrativeStateRepo::new(&db);

        let project_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test Project".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        // 创建叙事状态
        let state = repo.create(project_id, StateDimension::Narrative, "wang_family_destroyed", serde_json::json!(false), None).unwrap();
        assert_eq!(state.state_dimension, StateDimension::Narrative);
        assert_eq!(state.state_key, "wang_family_destroyed");

        // 查询
        let states = repo.list_by_dimension(project_id, StateDimension::Narrative).unwrap();
        assert_eq!(states.len(), 1);

        // 按 key 查询
        let found = repo.get_by_key(project_id, StateDimension::Narrative, "wang_family_destroyed").unwrap();
        assert!(found.is_some());
    }
}
