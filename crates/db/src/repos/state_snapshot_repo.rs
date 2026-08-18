//! StateSnapshot Repository - 状态快照（回滚支持）

use anyhow::{Context, Result};
use chrono::Utc;
use domain::state_mgmt::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct StateSnapshotRepo<'a> {
    db: &'a Database,
}

impl<'a> StateSnapshotRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, scene_id: Uuid, state_before: serde_json::Value, changes: serde_json::Value, state_after: serde_json::Value) -> Result<StateSnapshot> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO state_snapshot (id, project_id, scene_id, state_before, changes, state_after, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), scene_id.to_string(), state_before.to_string(), changes.to_string(), state_after.to_string(), now.to_rfc3339()],
        ).context("Failed to insert state_snapshot")?;
        Ok(StateSnapshot { id, project_id, scene_id, state_before, changes, state_after, created_at: now })
    }

    pub fn get_by_scene(&self, scene_id: Uuid) -> Result<Option<StateSnapshot>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, scene_id, state_before, changes, state_after, created_at FROM state_snapshot WHERE scene_id = ?",
            [scene_id.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let project_id: String = row.get(1)?;
                let scene_id: String = row.get(2)?;
                let state_before_str: String = row.get(3)?;
                let changes_str: String = row.get(4)?;
                let state_after_str: String = row.get(5)?;
                Ok(StateSnapshot {
                    id: Uuid::parse_str(&id).unwrap(),
                    project_id: Uuid::parse_str(&project_id).unwrap(),
                    scene_id: Uuid::parse_str(&scene_id).unwrap(),
                    state_before: serde_json::from_str(&state_before_str).unwrap_or(serde_json::json!({})),
                    changes: serde_json::from_str(&changes_str).unwrap_or(serde_json::json!({})),
                    state_after: serde_json::from_str(&state_after_str).unwrap_or(serde_json::json!({})),
                    created_at: get_timestamp(row, 6),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<StateSnapshot>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, scene_id, created_at FROM state_snapshot WHERE project_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let scene_id: String = row.get(2)?;
            Ok(StateSnapshot {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                scene_id: Uuid::parse_str(&scene_id).unwrap(),
                state_before: serde_json::json!({}),
                changes: serde_json::json!({}),
                state_after: serde_json::json!({}),
                created_at: get_timestamp(row, 3),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 获取指定场景之前的所有快照（用于回滚）
    pub fn list_before_scene(&self, project_id: Uuid, scene_id: Uuid) -> Result<Vec<StateSnapshot>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, scene_id, state_before, changes, state_after, created_at FROM state_snapshot WHERE project_id = ? AND scene_id != ? ORDER BY created_at DESC").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string(), scene_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let scene_id: String = row.get(2)?;
            let state_before_str: String = row.get(3)?;
            let changes_str: String = row.get(4)?;
            let state_after_str: String = row.get(5)?;
            Ok(StateSnapshot {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                scene_id: Uuid::parse_str(&scene_id).unwrap(),
                state_before: serde_json::from_str(&state_before_str).unwrap_or(serde_json::json!({})),
                changes: serde_json::from_str(&changes_str).unwrap_or(serde_json::json!({})),
                state_after: serde_json::from_str(&state_after_str).unwrap_or(serde_json::json!({})),
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Database;

    #[test]
    fn test_state_snapshot_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = StateSnapshotRepo::new(&db);

        let project_id = Uuid::new_v4();
        let scene_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let snapshot = repo.create(project_id, scene_id, serde_json::json!({"gold": 1000}), serde_json::json!({"gold": -300}), serde_json::json!({"gold": 700})).unwrap();
        assert_eq!(snapshot.scene_id, scene_id);

        let found = repo.get_by_scene(scene_id).unwrap();
        assert!(found.is_some());
    }
}
