//! Visibility Repository - CRUD operations for FactVisibility

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{FactVisibility, VisibilityLevel, VisibilitySubjectType};
use uuid::Uuid;

use crate::connection::Database;

pub struct VisibilityRepo<'a> {
    db: &'a Database,
}

impl<'a> VisibilityRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建事实可见性
    pub fn create(&self, project_id: Uuid, fact_id: Uuid, subject_type: VisibilitySubjectType, subject_id: Option<Uuid>, visibility_level: VisibilityLevel) -> Result<FactVisibility> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let st_str = match subject_type {
            VisibilitySubjectType::Author => "Author",
            VisibilitySubjectType::NarrativePlanner => "NarrativePlanner",
            VisibilitySubjectType::SceneWriter => "SceneWriter",
            VisibilitySubjectType::Character => "Character",
            VisibilitySubjectType::Reader => "Reader",
        };
        let vl_str = match visibility_level {
            VisibilityLevel::Visible => "Visible",
            VisibilityLevel::ExistsOnly => "ExistsOnly",
            VisibilityLevel::Hidden => "Hidden",
        };
        let conn = self.db.conn();
        if let Some(sid) = subject_id {
            conn.execute(
                "INSERT INTO fact_visibility (id, project_id, fact_id, subject_type, subject_id, visibility_level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                [id.to_string(), project_id.to_string(), fact_id.to_string(), st_str.to_string(), sid.to_string(), vl_str.to_string(), now.to_string(), now.to_string()],
            ).context("Failed to create fact visibility")?;
        } else {
            conn.execute(
                "INSERT INTO fact_visibility (id, project_id, fact_id, subject_type, subject_id, visibility_level, created_at, updated_at) VALUES (?, ?, ?, ?, NULL, ?, ?, ?)",
                [id.to_string(), project_id.to_string(), fact_id.to_string(), st_str.to_string(), vl_str.to_string(), now.to_string(), now.to_string()],
            ).context("Failed to create fact visibility")?;
        }
        Ok(FactVisibility { id, project_id, fact_id, subject_type, subject_id, visibility_level, created_at: now, updated_at: now })
    }

    /// 检查事实对主体的可见性
    pub fn check_visibility(&self, fact_id: Uuid, subject_type: VisibilitySubjectType, subject_id: Option<Uuid>) -> Result<VisibilityLevel> {
        let conn = self.db.conn();
        let st_str = match subject_type {
            VisibilitySubjectType::Author => "Author",
            VisibilitySubjectType::NarrativePlanner => "NarrativePlanner",
            VisibilitySubjectType::SceneWriter => "SceneWriter",
            VisibilitySubjectType::Character => "Character",
            VisibilitySubjectType::Reader => "Reader",
        };
        let query = if subject_id.is_some() {
            "SELECT visibility_level FROM fact_visibility WHERE fact_id = ? AND subject_type = ? AND subject_id = ? ORDER BY created_at DESC LIMIT 1"
        } else {
            "SELECT visibility_level FROM fact_visibility WHERE fact_id = ? AND subject_type = ? AND subject_id IS NULL ORDER BY created_at DESC LIMIT 1"
        };
        let result = if let Some(sid) = subject_id {
            conn.query_row(query, [fact_id.to_string(), st_str.to_string(), sid.to_string()], |row| {
                Ok(row.get::<_, String>(0)?)
            }).ok()
        } else {
            conn.query_row(query, [fact_id.to_string(), st_str.to_string()], |row| {
                Ok(row.get::<_, String>(0)?)
            }).ok()
        };
        match result.as_deref() {
            Some("Visible") => Ok(VisibilityLevel::Visible),
            Some("ExistsOnly") => Ok(VisibilityLevel::ExistsOnly),
            Some("Hidden") => Ok(VisibilityLevel::Hidden),
            _ => Ok(VisibilityLevel::Hidden), // 默认隐藏
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn setup_db() -> (Database, Uuid) {
        let db = Database::open_in_memory().unwrap();
        migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/migrations")).unwrap();
        // 先创建 project 和 fact
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
    fn test_create_and_check_visibility() {
        let (db, project_id) = setup_db();
        let repo = VisibilityRepo::new(&db);
        let fact_id = Uuid::new_v4();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO fact (id, project_id, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                [fact_id.to_string(), project_id.to_string(), "幕后黑手是A".to_string(), Utc::now().to_string(), Utc::now().to_string()],
            ).unwrap();
        }
        
        // 创建可见性规则
        repo.create(project_id, fact_id, VisibilitySubjectType::SceneWriter, None, VisibilityLevel::Hidden).unwrap();
        repo.create(project_id, fact_id, VisibilitySubjectType::Author, None, VisibilityLevel::Visible).unwrap();
        
        // 检查可见性
        let level = repo.check_visibility(fact_id, VisibilitySubjectType::SceneWriter, None).unwrap();
        assert_eq!(level, VisibilityLevel::Hidden);
        
        let level = repo.check_visibility(fact_id, VisibilitySubjectType::Author, None).unwrap();
        assert_eq!(level, VisibilityLevel::Visible);
    }
}
