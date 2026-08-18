//! CausalRelation Repository - CRUD operations for CausalRelation

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{CausalRelation, CausalRelationType, CausalStrength};
use uuid::Uuid;

use crate::connection::Database;

pub struct CausalRepo<'a> {
    db: &'a Database,
}

impl<'a> CausalRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建因果关系
    pub fn create(&self, project_id: Uuid, cause_event_id: Uuid, effect_event_id: Uuid, relation_type: CausalRelationType, strength: CausalStrength, description: Option<&str>) -> Result<CausalRelation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let rt_str = match relation_type {
            CausalRelationType::DirectCause => "DirectCause",
            CausalRelationType::IndirectCause => "IndirectCause",
            CausalRelationType::Trigger => "Trigger",
            CausalRelationType::Prerequisite => "Prerequisite",
            CausalRelationType::ContributingFactor => "ContributingFactor",
        };
        let s_str = match strength {
            CausalStrength::Strong => "Strong",
            CausalStrength::Moderate => "Moderate",
            CausalStrength::Weak => "Weak",
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO causal_relation (id, project_id, cause_event_id, effect_event_id, relation_type, strength, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), cause_event_id.to_string(), effect_event_id.to_string(), rt_str.to_string(), s_str.to_string(), description.unwrap_or("").to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create causal relation")?;
        Ok(CausalRelation { id, project_id, cause_event_id, effect_event_id, relation_type, strength, description: description.map(|s| s.to_string()), created_at: now, updated_at: now })
    }

    /// 列出项目中的所有因果关系
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CausalRelation>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, cause_event_id, effect_event_id, relation_type, strength, description, created_at, updated_at FROM causal_relation WHERE project_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(CausalRelation {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                cause_event_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                effect_event_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                relation_type: match row.get::<_, String>(4)?.as_str() {
                    "DirectCause" => CausalRelationType::DirectCause,
                    "IndirectCause" => CausalRelationType::IndirectCause,
                    "Trigger" => CausalRelationType::Trigger,
                    "Prerequisite" => CausalRelationType::Prerequisite,
                    "ContributingFactor" => CausalRelationType::ContributingFactor,
                    _ => CausalRelationType::DirectCause,
                },
                strength: match row.get::<_, String>(5)?.as_str() {
                    "Strong" => CausalStrength::Strong,
                    "Moderate" => CausalStrength::Moderate,
                    "Weak" => CausalStrength::Weak,
                    _ => CausalStrength::Moderate,
                },
                description: row.get::<_, Option<String>>(6)?,
                created_at: crate::time_utils::get_timestamp(row, 7),
                updated_at: crate::time_utils::get_timestamp(row, 8),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> (Database, Uuid) {
        let db = Database::open_in_memory().unwrap();
        migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let project_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test Project".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }
        (db, project_id)
    }

    #[test]
    fn test_create_causal() {
        let (db, project_id) = setup_db();
        let repo = CausalRepo::new(&db);
        let cause = Uuid::new_v4();
        let effect = Uuid::new_v4();
        let c = repo.create(project_id, cause, effect, CausalRelationType::DirectCause, CausalStrength::Strong, Some("资金不足导致扩张")).unwrap();
        assert_eq!(c.relation_type, CausalRelationType::DirectCause);
    }
}
