//! PlotRepair Repository - CRUD operations for PlotRepair

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{PlotRepair, RepairType, RepairStatus};
use uuid::Uuid;

use crate::connection::Database;

pub struct PlotRepairRepo<'a> {
    db: &'a Database,
}

impl<'a> PlotRepairRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建剧情修复记录
    pub fn create(&self, project_id: Uuid, scene_id: Uuid, issue_description: &str, repair_suggestion: &str, repair_type: RepairType) -> Result<PlotRepair> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let rt_str = match repair_type {
            RepairType::Automatic => "Automatic",
            RepairType::Suggested => "Suggested",
            RepairType::Manual => "Manual",
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO plot_repair (id, project_id, scene_id, issue_description, repair_suggestion, repair_type, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), scene_id.to_string(), issue_description.to_string(), repair_suggestion.to_string(), rt_str.to_string(), "Pending".to_string(), now.to_string()],
        ).context("Failed to create plot repair")?;
        Ok(PlotRepair { id, project_id, scene_id, issue_description: issue_description.to_string(), repair_suggestion: repair_suggestion.to_string(), repair_type, status: RepairStatus::Pending, applied_at: None, created_at: now })
    }

    /// 列出项目中的所有剧情修复
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<PlotRepair>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, scene_id, issue_description, repair_suggestion, repair_type, status, applied_at, created_at FROM plot_repair WHERE project_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(PlotRepair {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                scene_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                issue_description: row.get(3)?,
                repair_suggestion: row.get(4)?,
                repair_type: match row.get::<_, String>(5)?.as_str() {
                    "Automatic" => RepairType::Automatic,
                    "Suggested" => RepairType::Suggested,
                    "Manual" => RepairType::Manual,
                    _ => RepairType::Suggested,
                },
                status: match row.get::<_, String>(6)?.as_str() {
                    "Pending" => RepairStatus::Pending,
                    "Applied" => RepairStatus::Applied,
                    "Rejected" => RepairStatus::Rejected,
                    _ => RepairStatus::Pending,
                },
                applied_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
                created_at: crate::time_utils::get_timestamp(row, 8),
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
    fn test_create_plot_repair() {
        let (db, project_id) = setup_db();
        let repo = PlotRepairRepo::new(&db);
        let scene_id = Uuid::new_v4();
        let pr = repo.create(project_id, scene_id, "时间线矛盾", "调整场景顺序", RepairType::Suggested).unwrap();
        assert_eq!(pr.status, RepairStatus::Pending);
    }
}
