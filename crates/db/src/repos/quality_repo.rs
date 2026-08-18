//! QualityScore Repository - CRUD operations for QualityScore

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{QualityScore, QualityIssue};
use uuid::Uuid;

use crate::connection::Database;

pub struct QualityScoreRepo<'a> {
    db: &'a Database,
}

impl<'a> QualityScoreRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建质量评分
    pub fn create(&self, project_id: Uuid, scene_id: Uuid, continuity: Option<i32>, character: Option<i32>, plot: Option<i32>, knowledge: Option<i32>, world: Option<i32>, style: Option<i32>, issues: Vec<QualityIssue>) -> Result<QualityScore> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let overall = match (continuity, character, plot, knowledge, world, style) {
            (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) => Some((a + b + c + d + e + f) / 6),
            _ => None,
        };
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO quality_score (id, project_id, scene_id, continuity_score, character_score, plot_score, knowledge_score, world_score, style_score, overall_score, issues, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                id.to_string(), project_id.to_string(), scene_id.to_string(),
                continuity.map(|v| v.to_string()).unwrap_or_default(),
                character.map(|v| v.to_string()).unwrap_or_default(),
                plot.map(|v| v.to_string()).unwrap_or_default(),
                knowledge.map(|v| v.to_string()).unwrap_or_default(),
                world.map(|v| v.to_string()).unwrap_or_default(),
                style.map(|v| v.to_string()).unwrap_or_default(),
                overall.map(|v| v.to_string()).unwrap_or_default(),
                serde_json::to_string(&issues).unwrap_or_default(),
                now.to_string(),
            ],
        ).context("Failed to create quality score")?;
        Ok(QualityScore { id, project_id, scene_id, run_id: None, continuity_score: continuity, character_score: character, plot_score: plot, knowledge_score: knowledge, world_score: world, style_score: style, overall_score: overall, issues, created_at: now })
    }

    /// 按场景获取质量评分列表
    pub fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<QualityScore>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, scene_id, continuity_score, character_score, plot_score, knowledge_score, world_score, style_score, overall_score, issues, created_at FROM quality_score WHERE scene_id = ? ORDER BY created_at DESC",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([scene_id.to_string()], |row| {
            Ok(QualityScore {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                scene_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                run_id: None,
                continuity_score: row.get::<_, Option<i32>>(3)?,
                character_score: row.get::<_, Option<i32>>(4)?,
                plot_score: row.get::<_, Option<i32>>(5)?,
                knowledge_score: row.get::<_, Option<i32>>(6)?,
                world_score: row.get::<_, Option<i32>>(7)?,
                style_score: row.get::<_, Option<i32>>(8)?,
                overall_score: row.get::<_, Option<i32>>(9)?,
                issues: row.get::<_, Option<String>>(10)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                created_at: crate::time_utils::get_timestamp(row, 11),
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
    fn test_create_quality_score() {
        let (db, project_id) = setup_db();
        let repo = QualityScoreRepo::new(&db);
        let scene_id = Uuid::new_v4();
        let issues = vec![QualityIssue { dimension: "style".into(), severity: "Warning".into(), description: "节奏偏慢".into(), suggestion: Some("加快节奏".into()) }];
        let qs = repo.create(project_id, scene_id, Some(96), Some(91), Some(100), Some(100), Some(97), Some(87), issues).unwrap();
        assert_eq!(qs.overall_score, Some(95));
    }
}
