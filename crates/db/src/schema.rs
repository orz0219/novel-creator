//! Schema helper functions for PostgreSQL
//!
//! 提供数据库 schema 的查询和验证功能。

use anyhow::{Context, Result};
use sqlx::PgPool;

/// 获取所有表名
pub async fn list_tables(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
         ORDER BY table_name",
    )
    .fetch_all(pool)
    .await
    .context("Failed to query tables")?;

    Ok(rows)
}

/// 获取指定表的列信息
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
}

pub async fn describe_table(pool: &PgPool, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT column_name, data_type, is_nullable, column_default \
         FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 \
         ORDER BY ordinal_position",
    )
    .bind(table_name)
    .fetch_all(pool)
    .await
    .context("Failed to query columns")?;

    Ok(rows
        .into_iter()
        .map(|(name, data_type, nullable, default)| ColumnInfo {
            name,
            data_type,
            is_nullable: nullable == "YES",
            column_default: default,
        })
        .collect())
}

/// 验证所有核心表是否存在
pub async fn validate_schema(pool: &PgPool) -> Result<Vec<String>> {
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
        "character_profile",
        "character_state",
        "character_trait",
        "character_drive",
        "character_conflict",
        "character_relationship",
        "character_secret",
        "character_capability",
        "character_arc_potential",
        "character_extension",
        "faction_profile",
        "location_profile",
        "location_facility",
        "location_threat",
        "location_secret",
        "location_connection",
        "narrative_budget",
        "novel_state_snapshot",
        "memories",
        "agent_runs",
        "system_events",
        "narrative_thread",
        "narrative_thread_participant",
        "world_version",
    ];

    let existing = list_tables(pool).await?;
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

    #[tokio::test]
    async fn test_list_tables_requires_connection() {
        // This test requires DATABASE_URL to be set
        if std::env::var("DATABASE_URL").is_err() {
            eprintln!("Skipping test: DATABASE_URL not set");
            return;
        }
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let _tables = list_tables(&pool).await.unwrap();
    }
}