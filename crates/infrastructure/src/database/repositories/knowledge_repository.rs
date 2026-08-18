//! DuckDB implementation of Knowledge Repository

use anyhow::Result;
use domain::knowledge::{KnowledgeState, KnowledgeChange};
use duckdb::Connection;
use uuid::Uuid;

/// DuckDB implementation of knowledge repository
pub struct DuckDbKnowledgeRepository {
    conn: Connection,
}

impl DuckDbKnowledgeRepository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get knowledge state for a character
    pub fn get_character_knowledge(&self, character_id: Uuid, project_id: Uuid) -> Result<Vec<KnowledgeState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, character_id, entity_id, knowledge_level, source, confidence, created_at, updated_at FROM knowledge_state WHERE character_id = ? AND project_id = ?",
        )?;

        let rows = stmt.query_map([character_id.to_string(), project_id.to_string()], |row| {
            Ok(KnowledgeState {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                character_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                knowledge_level: row.get::<_, String>(4)?,
                source: row.get::<_, Option<String>>(5)?,
                confidence: row.get::<_, f64>(6)?,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })?;

        let mut states = Vec::new();
        for row in rows {
            states.push(row?);
        }

        Ok(states)
    }

    /// Record a knowledge change
    pub fn record_change(&self, change: &KnowledgeChange) -> Result<()> {
        self.conn.execute(
            "INSERT INTO knowledge_change (id, project_id, character_id, entity_id, change_type, source, confidence, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [
                change.id.to_string(),
                change.project_id.to_string(),
                change.character_id.to_string(),
                change.entity_id.to_string(),
                change.change_type.clone(),
                change.source.clone().unwrap_or_default(),
                change.confidence.to_string(),
                change.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get knowledge changes for a character
    pub fn get_changes_by_character(&self, character_id: Uuid) -> Result<Vec<KnowledgeChange>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, character_id, entity_id, change_type, source, confidence, created_at FROM knowledge_change WHERE character_id = ? ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([character_id.to_string()], |row| {
            Ok(KnowledgeChange {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                character_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                change_type: row.get::<_, String>(4)?,
                source: row.get::<_, Option<String>>(5)?,
                confidence: row.get::<_, f64>(6)?,
                created_at: chrono::Utc::now(),
            })
        })?;

        let mut changes = Vec::new();
        for row in rows {
            changes.push(row?);
        }

        Ok(changes)
    }

    /// Get knowledge changes for a project
    pub fn get_changes_by_project(&self, project_id: Uuid) -> Result<Vec<KnowledgeChange>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, character_id, entity_id, change_type, source, confidence, created_at FROM knowledge_change WHERE project_id = ? ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(KnowledgeChange {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                character_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                entity_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap(),
                change_type: row.get::<_, String>(4)?,
                source: row.get::<_, Option<String>>(5)?,
                confidence: row.get::<_, f64>(6)?,
                created_at: chrono::Utc::now(),
            })
        })?;

        let mut changes = Vec::new();
        for row in rows {
            changes.push(row?);
        }

        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_knowledge_crud() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        // Create tables
        conn.execute_batch(
            "CREATE TABLE knowledge_state (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                character_id VARCHAR NOT NULL,
                entity_id VARCHAR NOT NULL,
                knowledge_level VARCHAR NOT NULL,
                source VARCHAR,
                confidence DOUBLE DEFAULT 1.0,
                created_at TIMESTAMP,
                updated_at TIMESTAMP
            )"
        ).unwrap();

        conn.execute_batch(
            "CREATE TABLE knowledge_change (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                character_id VARCHAR NOT NULL,
                entity_id VARCHAR NOT NULL,
                change_type VARCHAR NOT NULL,
                source VARCHAR,
                confidence DOUBLE DEFAULT 1.0,
                created_at TIMESTAMP
            )"
        ).unwrap();

        let repo = DuckDbKnowledgeRepository::new(conn);

        // Record knowledge change
        let change = KnowledgeChange {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            character_id: Uuid::new_v4(),
            entity_id: Uuid::new_v4(),
            change_type: "learned".to_string(),
            source: Some("observation".to_string()),
            confidence: 0.9,
            created_at: chrono::Utc::now(),
        };

        repo.record_change(&change).unwrap();

        // Get changes
        let changes = repo.get_changes_by_character(change.character_id).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, "learned");
    }
}
