//! Revelation Repository - CRUD operations for Revelation

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{Revelation, RevelationTarget};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct RevelationRepo<'a> {
    db: &'a Database,
}

impl<'a> RevelationRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建揭示
    pub fn create(&self, project_id: Uuid, fact_id: Uuid, scene_id: Uuid, revealed_to: &[RevelationTarget], method: Option<&str>, significance: Option<&str>) -> Result<Revelation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO revelation (id, project_id, fact_id, scene_id, revelation_method, narrative_significance, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), fact_id.to_string(), scene_id.to_string(), method.unwrap_or("").to_string(), significance.unwrap_or("").to_string(), now.to_string()],
        ).context("Failed to create revelation")?;

        for target in revealed_to {
            let st_str = crate::ser::knowledge_subject_type_str(&target.subject_type);
            let kl_str = crate::ser::knowledge_level_str(&target.knowledge_level);
            conn.execute(
                "INSERT INTO revelation_target (id, revelation_id, subject_type, subject_id, knowledge_level) VALUES (?, ?, ?, ?, ?)",
                [Uuid::new_v4().to_string(), id.to_string(), st_str, target.subject_id.map(|s| s.to_string()).unwrap_or_default(), kl_str],
            ).context("Failed to create revelation target")?;
        }

        Ok(Revelation { id, project_id, fact_id, scene_id, revealed_to: revealed_to.to_vec(), revelation_method: method.map(|s| s.to_string()), narrative_significance: significance.map(|s| s.to_string()), created_at: now })
    }

    /// 按场景获取揭示列表
    pub fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<Revelation>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, fact_id, scene_id, revelation_method, narrative_significance, created_at FROM revelation WHERE scene_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([scene_id.to_string()], |row| {
            Ok(Revelation {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                fact_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                scene_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                revealed_to: Vec::new(),
                revelation_method: row.get::<_, Option<String>>(4)?,
                narrative_significance: row.get::<_, Option<String>>(5)?,
                created_at: get_timestamp(row, 6),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 按事实获取揭示列表
    pub fn list_by_fact(&self, fact_id: Uuid) -> Result<Vec<Revelation>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, fact_id, scene_id, revelation_method, narrative_significance, created_at FROM revelation WHERE fact_id = ? ORDER BY created_at",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([fact_id.to_string()], |row| {
            Ok(Revelation {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                fact_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                scene_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                revealed_to: Vec::new(),
                revelation_method: row.get::<_, Option<String>>(4)?,
                narrative_significance: row.get::<_, Option<String>>(5)?,
                created_at: get_timestamp(row, 6),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
