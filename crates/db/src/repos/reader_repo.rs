//! ReaderKnowledge Repository - CRUD operations for ReaderKnowledge

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{ReaderKnowledge, ReaderKnowledgeLevel, ReaderConfidence};
use uuid::Uuid;

use crate::connection::Database;

pub struct ReaderKnowledgeRepo<'a> {
    db: &'a Database,
}

impl<'a> ReaderKnowledgeRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建读者知识
    pub fn create(&self, project_id: Uuid, fact_id: Uuid, knowledge_level: ReaderKnowledgeLevel, confidence: ReaderConfidence) -> Result<ReaderKnowledge> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let kl_str = match knowledge_level {
            ReaderKnowledgeLevel::Unknown => "Unknown",
            ReaderKnowledgeLevel::Hearsay => "Hearsay",
            ReaderKnowledgeLevel::Suspected => "Suspected",
            ReaderKnowledgeLevel::Partial => "Partial",
            ReaderKnowledgeLevel::Complete => "Complete",
            ReaderKnowledgeLevel::Misunderstood => "Misunderstood",
        };
        let c_str = match confidence {
            ReaderConfidence::Certain => "Certain",
            ReaderConfidence::Likely => "Likely",
            ReaderConfidence::Uncertain => "Uncertain",
            ReaderConfidence::Speculative => "Speculative",
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO reader_knowledge (id, project_id, fact_id, knowledge_level, confidence, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), fact_id.to_string(), kl_str.to_string(), c_str.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create reader knowledge")?;
        Ok(ReaderKnowledge { id, project_id, fact_id, knowledge_level, source_scene_id: None, confidence, created_at: now, updated_at: now })
    }

    /// 列出项目中的所有读者知识
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<ReaderKnowledge>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, fact_id, knowledge_level, source_scene_id, confidence, created_at, updated_at FROM reader_knowledge WHERE project_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(ReaderKnowledge {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                fact_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                knowledge_level: match row.get::<_, String>(3)?.as_str() {
                    "Unknown" => ReaderKnowledgeLevel::Unknown,
                    "Hearsay" => ReaderKnowledgeLevel::Hearsay,
                    "Suspected" => ReaderKnowledgeLevel::Suspected,
                    "Partial" => ReaderKnowledgeLevel::Partial,
                    "Complete" => ReaderKnowledgeLevel::Complete,
                    "Misunderstood" => ReaderKnowledgeLevel::Misunderstood,
                    _ => ReaderKnowledgeLevel::Unknown,
                },
                source_scene_id: row.get::<_, Option<String>>(4)?.and_then(|s| Uuid::parse_str(&s).ok()),
                confidence: match row.get::<_, String>(5)?.as_str() {
                    "Certain" => ReaderConfidence::Certain,
                    "Likely" => ReaderConfidence::Likely,
                    "Uncertain" => ReaderConfidence::Uncertain,
                    "Speculative" => ReaderConfidence::Speculative,
                    _ => ReaderConfidence::Certain,
                },
                created_at: crate::time_utils::get_timestamp(row, 6),
                updated_at: crate::time_utils::get_timestamp(row, 7),
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
    fn test_create_reader_knowledge() {
        let (db, project_id) = setup_db();
        let repo = ReaderKnowledgeRepo::new(&db);
        let fact_id = Uuid::new_v4();
        let rk = repo.create(project_id, fact_id, ReaderKnowledgeLevel::Suspected, ReaderConfidence::Speculative).unwrap();
        assert_eq!(rk.knowledge_level, ReaderKnowledgeLevel::Suspected);
    }
}
