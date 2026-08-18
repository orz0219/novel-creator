//! DecisionTrace Repository - AI 决策追踪 CRUD

use anyhow::{Context, Result};
use chrono::Utc;
use domain::ledger::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct DecisionTraceRepo<'a> {
    db: &'a Database,
}

impl<'a> DecisionTraceRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, scene_id: Uuid, character_id: Uuid, decision: &str, factors: Vec<DecisionFactor>) -> Result<DecisionTrace> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO decision_trace (id, project_id, scene_id, character_id, decision, factors, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), scene_id.to_string(), character_id.to_string(), decision.to_string(), serde_json::to_string(&factors).unwrap_or_default(), now.to_rfc3339()],
        ).context("Failed to insert decision_trace")?;
        Ok(DecisionTrace { id, project_id, scene_id, character_id, decision: decision.to_string(), factors, created_at: now })
    }

    pub fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<DecisionTrace>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, scene_id, character_id, decision, factors, created_at FROM decision_trace WHERE scene_id = ?").context("Failed to prepare")?;
        let rows = stmt.query_map([scene_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let scene_id: String = row.get(2)?;
            let character_id: String = row.get(3)?;
            let decision: String = row.get(4)?;
            let factors_str: String = row.get(5)?;
            Ok(DecisionTrace {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                scene_id: Uuid::parse_str(&scene_id).unwrap(),
                character_id: Uuid::parse_str(&character_id).unwrap(),
                decision,
                factors: serde_json::from_str(&factors_str).unwrap_or_default(),
                created_at: get_timestamp(row, 6),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_by_character(&self, character_id: Uuid) -> Result<Vec<DecisionTrace>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, scene_id, character_id, decision, factors, created_at FROM decision_trace WHERE character_id = ? ORDER BY created_at DESC").context("Failed to prepare")?;
        let rows = stmt.query_map([character_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let scene_id: String = row.get(2)?;
            let character_id: String = row.get(3)?;
            let decision: String = row.get(4)?;
            let factors_str: String = row.get(5)?;
            Ok(DecisionTrace {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                scene_id: Uuid::parse_str(&scene_id).unwrap(),
                character_id: Uuid::parse_str(&character_id).unwrap(),
                decision,
                factors: serde_json::from_str(&factors_str).unwrap_or_default(),
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
    fn test_decision_trace_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = DecisionTraceRepo::new(&db);

        let project_id = Uuid::new_v4();
        let scene_id = Uuid::new_v4();
        let character_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let trace = repo.create(project_id, scene_id, character_id, "逃跑", vec![
            DecisionFactor { factor_type: "emotion".to_string(), description: "恐惧85".to_string(), influence: 0.8, related_id: None },
            DecisionFactor { factor_type: "personality".to_string(), description: "谨慎".to_string(), influence: 0.6, related_id: None },
        ]).unwrap();

        assert_eq!(trace.decision, "逃跑");
        assert_eq!(trace.factors.len(), 2);

        let traces = repo.list_by_scene(scene_id).unwrap();
        assert_eq!(traces.len(), 1);
    }
}
