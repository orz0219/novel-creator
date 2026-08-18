//! KnowledgeGap Repository - 知识缺口 CRUD

use anyhow::{Context, Result};
use chrono::Utc;
use domain::state_mgmt::*;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct KnowledgeGapRepo<'a> {
    db: &'a Database,
}

impl<'a> KnowledgeGapRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, gap_type: &str, description: &str, importance: &str, required_by_scene_id: Option<Uuid>, designer_skill_hint: Option<&str>) -> Result<KnowledgeGap> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO knowledge_gap (id, project_id, gap_type, description, importance, required_by_scene_id, status, designer_skill_hint, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                id.to_string(), project_id.to_string(), gap_type.to_string(), description.to_string(), importance.to_string(),
                required_by_scene_id.map(|s| s.to_string()).unwrap_or_default(),
                "Open".to_string(),
                designer_skill_hint.unwrap_or("").to_string(),
                now.to_rfc3339(), now.to_rfc3339(),
            ],
        ).context("Failed to insert knowledge_gap")?;
        Ok(KnowledgeGap { id, project_id, gap_type: gap_type.to_string(), description: description.to_string(), importance: importance.to_string(), required_by_scene_id, status: GapStatus::Open, designer_skill_hint: designer_skill_hint.map(|s| s.to_string()), created_at: now, updated_at: now })
    }

    pub fn update_status(&self, id: Uuid, status: GapStatus) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE knowledge_gap SET status = ?, updated_at = ? WHERE id = ?",
            [status.as_str().to_string(), Utc::now().to_rfc3339(), id.to_string()],
        ).context("Failed to update knowledge_gap status")?;
        Ok(())
    }

    pub fn list_open_by_project(&self, project_id: Uuid) -> Result<Vec<KnowledgeGap>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, gap_type, description, importance, required_by_scene_id, status, designer_skill_hint, created_at, updated_at FROM knowledge_gap WHERE project_id = ? AND status = 'Open' ORDER BY importance DESC").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let id: String = row.get(0)?;
            let project_id: String = row.get(1)?;
            let gap_type: String = row.get(2)?;
            let description: String = row.get(3)?;
            let importance: String = row.get(4)?;
            let required_by_scene_id: Option<String> = row.get(5)?;
            let status: String = row.get(6)?;
            let designer_skill_hint: Option<String> = row.get(7)?;
            Ok(KnowledgeGap {
                id: Uuid::parse_str(&id).unwrap(),
                project_id: Uuid::parse_str(&project_id).unwrap(),
                gap_type,
                description,
                importance,
                required_by_scene_id: required_by_scene_id.and_then(|s| Uuid::parse_str(&s).ok()),
                status: GapStatus::from_str(&status),
                designer_skill_hint,
                created_at: get_timestamp(row, 8),
                updated_at: get_timestamp(row, 9),
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
    fn test_knowledge_gap_crud() {
        let db = Database::open_in_memory().unwrap();
        crate::migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        let repo = KnowledgeGapRepo::new(&db);

        let project_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                [project_id.to_string(), "Test".to_string(), "Test".to_string(), "Active".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }

        let gap = repo.create(project_id, "LOCATION_DETAIL", "黑市入口在哪里", "HIGH", None, Some("location_designer")).unwrap();
        assert_eq!(gap.gap_type, "LOCATION_DETAIL");
        assert_eq!(gap.importance, "HIGH");
        assert_eq!(gap.status, GapStatus::Open);

        let gaps = repo.list_open_by_project(project_id).unwrap();
        assert_eq!(gaps.len(), 1);

        repo.update_status(gap.id, GapStatus::Filled).unwrap();
        let gaps = repo.list_open_by_project(project_id).unwrap();
        assert!(gaps.is_empty());
    }
}
