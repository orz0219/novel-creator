//! Knowledge Repository

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{KnowledgeLevel, KnowledgeState, KnowledgeSubjectType, Revelation, RevelationTarget};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct KnowledgeRepo<'a> { db: &'a Database }

impl<'a> KnowledgeRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create_state(&self, project_id: Uuid, fact_id: Uuid, subject_type: KnowledgeSubjectType, subject_id: Option<Uuid>, knows: bool, knowledge_level: KnowledgeLevel, source: Option<&str>) -> Result<KnowledgeState> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let st_str = crate::ser::knowledge_subject_type_str(&subject_type);
        let kl_str = crate::ser::knowledge_level_str(&knowledge_level);
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO knowledge_state (id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source, effective_from, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), fact_id.to_string(), st_str, subject_id.map(|s| s.to_string()).unwrap_or_default(), knows.to_string(), kl_str, source.unwrap_or("").to_string(), now.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create")?;
        Ok(KnowledgeState { id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source: source.map(|s| s.to_string()), effective_from: now, effective_to: None, created_at: now, updated_at: now })
    }

    pub fn get_state(&self, fact_id: Uuid, subject_type: &KnowledgeSubjectType, subject_id: Option<Uuid>) -> Result<Option<KnowledgeState>> {
        let conn = self.db.conn();
        let st_str = crate::ser::knowledge_subject_type_str(subject_type);
        let sid_str = subject_id.map(|s| s.to_string()).unwrap_or_default();
        let result = conn.query_row(
            "SELECT id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source, effective_from, effective_to, created_at, updated_at FROM knowledge_state WHERE fact_id = ? AND subject_type = ? AND subject_id = ? ORDER BY created_at DESC LIMIT 1",
            [fact_id.to_string(), st_str, sid_str],
            |row| {
                let sid: Option<String> = row.get(4)?;
                Ok(KnowledgeState {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    fact_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    subject_type: crate::ser::parse_knowledge_subject_type(&row.get::<_, String>(3)?),
                    subject_id: sid.and_then(|s| Uuid::parse_str(&s).ok()),
                    knows: row.get(5)?,
                    knowledge_level: crate::ser::parse_knowledge_level(&row.get::<_, String>(6)?),
                    source: row.get(7)?,
                    effective_from: get_timestamp(row, 8),
                    effective_to: None,
                    created_at: get_timestamp(row, 10),
                    updated_at: get_timestamp(row, 11),
                })
            },
        ).ok();
        Ok(result)
    }

    /// Get all knowledge states for a character
    pub fn get_character_knowledge(&self, character_id: Uuid, project_id: Uuid) -> Result<Vec<KnowledgeState>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source, effective_from, effective_to, created_at, updated_at FROM knowledge_state WHERE project_id = ? AND subject_type = 'Character' AND subject_id = ? AND knows = TRUE ORDER BY created_at DESC"
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string(), character_id.to_string()], |row| {
            let sid: Option<String> = row.get(4)?;
            Ok(KnowledgeState {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                fact_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                subject_type: crate::ser::parse_knowledge_subject_type(&row.get::<_, String>(3)?),
                subject_id: sid.and_then(|s| Uuid::parse_str(&s).ok()),
                knows: row.get(5)?,
                knowledge_level: crate::ser::parse_knowledge_level(&row.get::<_, String>(6)?),
                source: row.get(7)?,
                effective_from: get_timestamp(row, 8),
                effective_to: None,
                created_at: get_timestamp(row, 10),
                updated_at: get_timestamp(row, 11),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn create_revelation(&self, project_id: Uuid, fact_id: Uuid, scene_id: Uuid, revealed_to: &[RevelationTarget], method: Option<&str>, significance: Option<&str>) -> Result<Revelation> {
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
            ).context("Failed to create target")?;
        }
        Ok(Revelation { id, project_id, fact_id, scene_id, revealed_to: revealed_to.to_vec(), revelation_method: method.map(|s| s.to_string()), narrative_significance: significance.map(|s| s.to_string()), created_at: now })
    }
}
