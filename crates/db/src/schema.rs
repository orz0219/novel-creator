//! Schema helper functions
//!
//! 提供数据库 schema 的查询和验证功能。

use anyhow::{Context, Result};
use crate::connection::Database;

/// 获取所有表名
pub fn list_tables(db: &Database) -> Result<Vec<String>> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare("SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' AND table_type = 'BASE TABLE' ORDER BY table_name")
        .context("Failed to query tables")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .context("Failed to read tables")?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// 获取指定表的列信息
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
}

pub fn describe_table(db: &Database, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT column_name, data_type, is_nullable, column_default
             FROM information_schema.columns
             WHERE table_schema = 'main' AND table_name = ?
             ORDER BY ordinal_position",
        )
        .context("Failed to query columns")?;
    let rows = stmt
        .query_map([table_name], |row| {
            Ok(ColumnInfo {
                name: row.get(0)?,
                data_type: row.get(1)?,
                is_nullable: row.get::<_, String>(2)? == "YES",
                column_default: row.get(3).ok(),
            })
        })
        .context("Failed to read columns")?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// 验证所有核心表是否存在
pub fn validate_schema(db: &Database) -> Result<Vec<String>> {
    let expected_tables = vec![
        "project",
        "entity_type",
        "entity",
        "relation",
        "fact",
        "fact_entity",
        "event",
        "event_entity",
        "state_change",
        "current_state",
        "resource_state",
        "narrative_node",
        "plot",
        "scene",
        "scene_entity",
        "scene_requirement",
        "character_arc",
        "knowledge_state",
        "revelation",
        "revelation_target",
        "skill",
        "skill_version",
        "generation_task",
        "generation_run",
        "context_snapshot",
        "proposed_change",
        "validation_run",
        "validation_issue",
        "scene_document",
        "timeline_event",
        "storyline",
        "storyline_scene",
        "fact_visibility",
        "approval_record",
        "foreshadowing",
        "causal_relation",
        "reader_knowledge",
        "scene_contract",
        "quality_score",
        "world_branch",
        "narrative_branch",
        "plot_repair",
        // migration 007 new tables
        "character_profile",
        "character_state",
        "character_goal",
        "character_trait",
        "faction_profile",
        "location_profile",
        "location_facility",
        "location_threat",
        "location_secret",
        "location_connection",
        "narrative_budget",
        "novel_state_snapshot",
        "narrative_thread",
        "narrative_thread_participant",
    ];

    let existing = list_tables(db)?;
    let mut missing = Vec::new();

    for table in &expected_tables {
        if !existing.contains(&table.to_string()) {
            missing.push(table.to_string());
        }
    }

    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tables_empty() {
        let db = Database::open_in_memory().unwrap();
        let tables = list_tables(&db).unwrap();
        assert!(tables.is_empty());
    }
}
