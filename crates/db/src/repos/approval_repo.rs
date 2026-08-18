//! Approval Repository - CRUD operations for ApprovalRecord

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{ApprovalRecord, ApprovalStatus, ApprovalTargetType};
use uuid::Uuid;

use crate::connection::Database;

pub struct ApprovalRepo<'a> {
    db: &'a Database,
}

impl<'a> ApprovalRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建审批记录
    pub fn create(&self, project_id: Uuid, target_type: ApprovalTargetType, target_id: Uuid, proposed_by: &str, proposal_content: serde_json::Value) -> Result<ApprovalRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let tt_str = match &target_type {
            ApprovalTargetType::World => "World",
            ApprovalTargetType::Entity => "Entity",
            ApprovalTargetType::Volume => "Volume",
            ApprovalTargetType::Arc => "Arc",
            ApprovalTargetType::Scene => "Scene",
            ApprovalTargetType::Storyline => "Storyline",
            ApprovalTargetType::Fact => "Fact",
            ApprovalTargetType::Custom(s) => s,
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO approval_record (id, project_id, target_type, target_id, proposed_by, proposal_content, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), tt_str.to_string(), target_id.to_string(), proposed_by.to_string(), proposal_content.to_string(), "Pending".to_string(), now.to_string()],
        ).context("Failed to create approval record")?;
        Ok(ApprovalRecord { id, project_id, target_type, target_id, proposed_by: proposed_by.to_string(), proposal_content, status: ApprovalStatus::Pending, reviewer_id: None, reviewer_comment: None, created_at: now, reviewed_at: None })
    }

    /// 审批记录
    pub fn approve(&self, id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE approval_record SET status = 'Approved', reviewer_id = ?, reviewer_comment = ?, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
            [reviewer_id.to_string(), comment.unwrap_or("").to_string(), id.to_string()],
        ).context("Failed to approve record")?;
        Ok(())
    }

    /// 拒绝记录
    pub fn reject(&self, id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE approval_record SET status = 'Rejected', reviewer_id = ?, reviewer_comment = ?, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
            [reviewer_id.to_string(), comment.unwrap_or("").to_string(), id.to_string()],
        ).context("Failed to reject record")?;
        Ok(())
    }

    /// 获取待审批记录
    pub fn list_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, target_type, target_id, proposed_by, CAST(proposal_content AS VARCHAR), status, reviewer_id, reviewer_comment, created_at, reviewed_at FROM approval_record WHERE project_id = ? AND status = 'Pending' ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(ApprovalRecord {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                target_type: match row.get::<_, String>(2)?.as_str() {
                    "World" => ApprovalTargetType::World,
                    "Entity" => ApprovalTargetType::Entity,
                    "Volume" => ApprovalTargetType::Volume,
                    "Arc" => ApprovalTargetType::Arc,
                    "Scene" => ApprovalTargetType::Scene,
                    "Storyline" => ApprovalTargetType::Storyline,
                    "Fact" => ApprovalTargetType::Fact,
                    s => ApprovalTargetType::Custom(s.to_string()),
                },
                target_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                proposed_by: row.get(4)?,
                proposal_content: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                status: ApprovalStatus::Pending,
                reviewer_id: row.get::<_, Option<String>>(7)?,
                reviewer_comment: row.get::<_, Option<String>>(8)?,
                created_at: crate::time_utils::get_timestamp(row, 9),
                reviewed_at: row.get::<_, Option<String>>(10)?.and_then(|s| s.parse().ok()),
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
        // 先创建 project
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
    fn test_create_and_approve() {
        let (db, project_id) = setup_db();
        let repo = ApprovalRepo::new(&db);
        let target_id = Uuid::new_v4();
        let record = repo.create(project_id, ApprovalTargetType::Entity, target_id, "ai", serde_json::json!({"name": "地下赌场"})).unwrap();
        assert_eq!(record.status, ApprovalStatus::Pending);
        repo.approve(record.id, "user1", Some("看起来不错")).unwrap();
        let pending = repo.list_pending(project_id).unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_reject() {
        let (db, project_id) = setup_db();
        let repo = ApprovalRepo::new(&db);
        let target_id = Uuid::new_v4();
        let record = repo.create(project_id, ApprovalTargetType::Scene, target_id, "ai", serde_json::json!({"objective": "test"})).unwrap();
        repo.reject(record.id, "user1", Some("不符合设定")).unwrap();
        let pending = repo.list_pending(project_id).unwrap();
        assert_eq!(pending.len(), 0);
    }
}
