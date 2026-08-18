//! SceneLedger Repository - 场景账本 CRUD

use anyhow::{Context, Result};
use chrono::Utc;
use domain::ledger::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct SceneLedgerRepo<'a> {
    db: &'a Database,
}

impl<'a> SceneLedgerRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, scene_id: Uuid, ledger: &SceneLedger) -> Result<()> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO scene_ledger (id, project_id, scene_id, events, gains, losses, relationship_changes, knowledge_changes, world_changes, foreshadowing_mentions, storyline_progress, character_growth, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                id.to_string(), project_id.to_string(), scene_id.to_string(),
                serde_json::to_string(&ledger.events).unwrap_or_default(),
                serde_json::to_string(&ledger.gains).unwrap_or_default(),
                serde_json::to_string(&ledger.losses).unwrap_or_default(),
                serde_json::to_string(&ledger.relationship_changes).unwrap_or_default(),
                serde_json::to_string(&ledger.knowledge_changes).unwrap_or_default(),
                serde_json::to_string(&ledger.world_changes).unwrap_or_default(),
                serde_json::to_string(&ledger.foreshadowing_mentions).unwrap_or_default(),
                serde_json::to_string(&ledger.storyline_progress).unwrap_or_default(),
                serde_json::to_string(&ledger.character_growth).unwrap_or_default(),
                now.to_rfc3339(),
            ],
        ).context("Failed to insert scene_ledger")?;
        Ok(())
    }

    pub fn get_by_scene(&self, scene_id: Uuid) -> Result<Option<SceneLedger>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, scene_id, events, gains, losses, relationship_changes, knowledge_changes, world_changes, foreshadowing_mentions, storyline_progress, character_growth, created_at FROM scene_ledger WHERE scene_id = ?",
            [scene_id.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let project_id: String = row.get(1)?;
                let scene_id: String = row.get(2)?;
                let parse_json = |s: Option<String>| -> Vec<serde_json::Value> {
                    s.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
                };
                Ok(SceneLedger {
                    id: Uuid::parse_str(&id).unwrap(),
                    project_id: Uuid::parse_str(&project_id).unwrap(),
                    scene_id: Uuid::parse_str(&scene_id).unwrap(),
                    events: Vec::new(), // Simplified for now
                    gains: Vec::new(),
                    losses: Vec::new(),
                    relationship_changes: Vec::new(),
                    knowledge_changes: Vec::new(),
                    world_changes: Vec::new(),
                    foreshadowing_mentions: Vec::new(),
                    storyline_progress: Vec::new(),
                    character_growth: Vec::new(),
                    created_at: get_timestamp(row, 12),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<SceneLedger>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, scene_id, created_at FROM scene_ledger WHERE project_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let scene_id: String = row.get(2)?;
            Ok(SceneLedger {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                scene_id: Uuid::parse_str(&scene_id).unwrap(),
                events: Vec::new(),
                gains: Vec::new(),
                losses: Vec::new(),
                relationship_changes: Vec::new(),
                knowledge_changes: Vec::new(),
                world_changes: Vec::new(),
                foreshadowing_mentions: Vec::new(),
                storyline_progress: Vec::new(),
                character_growth: Vec::new(),
                created_at: get_timestamp(row, 3),
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
    fn test_scene_ledger_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = SceneLedgerRepo::new(&db);

        let project_id = Uuid::new_v4();
        let scene_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let ledger = SceneLedger {
            id: Uuid::new_v4(),
            project_id,
            scene_id,
            events: vec![LedgerEvent { description: "林凡进入黑市".to_string(), event_type: None, involved_entity_ids: vec![] }],
            gains: vec![LedgerItem { item_name: "黑市通行证".to_string(), item_type: None, entity_id: None, quantity: None }],
            losses: vec![LedgerItem { item_name: "300灵石".to_string(), item_type: None, entity_id: None, quantity: Some(300.0) }],
            relationship_changes: vec![],
            knowledge_changes: vec![],
            world_changes: vec![],
            foreshadowing_mentions: vec![],
            storyline_progress: vec![],
            character_growth: vec![],
            created_at: Utc::now(),
        };

        repo.create(project_id, scene_id, &ledger).unwrap();

        let found = repo.get_by_scene(scene_id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().scene_id, scene_id);
    }
}
