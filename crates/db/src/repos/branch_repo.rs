//! Branch Repository - CRUD operations for WorldBranch/NarrativeBranch

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{WorldBranch, NarrativeBranch, BranchStatus};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct BranchRepo<'a> {
    db: &'a Database,
}

impl<'a> BranchRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建世界分支
    pub fn create_world_branch(&self, project_id: Uuid, name: &str, description: Option<&str>, is_main: bool) -> Result<WorldBranch> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO world_branch (id, project_id, name, description, is_main, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), name.to_string(), description.unwrap_or("").to_string(), is_main.to_string(), "Active".to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create world branch")?;
        Ok(WorldBranch { id, project_id, name: name.to_string(), description: description.map(|s| s.to_string()), parent_branch_id: None, is_main, status: BranchStatus::Active, created_at: now, updated_at: now })
    }

    /// 创建叙事分支
    pub fn create_narrative_branch(&self, project_id: Uuid, name: &str, description: Option<&str>, is_main: bool) -> Result<NarrativeBranch> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO narrative_branch (id, project_id, name, description, is_main, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), name.to_string(), description.unwrap_or("").to_string(), is_main.to_string(), "Active".to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create narrative branch")?;
        Ok(NarrativeBranch { id, project_id, name: name.to_string(), description: description.map(|s| s.to_string()), parent_branch_id: None, fork_point_scene_id: None, is_main, status: BranchStatus::Active, created_at: now, updated_at: now })
    }

    /// 列出项目中的所有世界分支
    pub fn list_world_branches(&self, project_id: Uuid) -> Result<Vec<WorldBranch>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, parent_branch_id, is_main, status, created_at, updated_at FROM world_branch WHERE project_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(WorldBranch {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                name: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?,
                parent_branch_id: row.get::<_, Option<String>>(4)?.and_then(|s| Uuid::parse_str(&s).ok()),
                is_main: row.get::<_, bool>(5)?,
                status: match row.get::<_, String>(6)?.as_str() {
                    "Active" => BranchStatus::Active,
                    "Merged" => BranchStatus::Merged,
                    "Abandoned" => BranchStatus::Abandoned,
                    _ => BranchStatus::Active,
                },
                created_at: get_timestamp(row, 7),
                updated_at: get_timestamp(row, 8),
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
    fn test_create_branches() {
        let (db, project_id) = setup_db();
        let repo = BranchRepo::new(&db);
        let wb = repo.create_world_branch(project_id, "main", Some("主线世界"), true).unwrap();
        assert_eq!(wb.name, "main");
        assert!(wb.is_main);
        let nb = repo.create_narrative_branch(project_id, "chapter-20-rewrite", Some("第20章重写"), false).unwrap();
        assert_eq!(nb.name, "chapter-20-rewrite");
        assert!(!nb.is_main);
    }
}
