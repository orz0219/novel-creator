//! SQLite schema 辅助函数（手工维护）
//!
//! PostgreSQL 版查 information_schema；SQLite 没有该视图，
//! 改用 sqlite_master 与 PRAGMA table_info。

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// 获取所有用户表名（排除 SQLite 内部表与迁移记录表）
pub async fn list_tables(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> '_migrations' \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context("Failed to query tables")?;

    Ok(rows)
}

/// 列信息
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
}

/// 查询指定表的列信息（用 PRAGMA table_info）
pub async fn describe_table(pool: &SqlitePool, table_name: &str) -> Result<Vec<ColumnInfo>> {
    // PRAGMA 不支持参数绑定，表名来自内部调用，这里用格式化拼接
    let sql = format!("PRAGMA table_info(\"{}\")", table_name.replace('"', ""));

    let rows = sqlx::query_as::<_, (i64, String, String, i64, Option<String>, i64)>(&sql)
        .fetch_all(pool)
        .await
        .context("Failed to query columns")?;

    Ok(rows
        .into_iter()
        .map(|(_cid, name, data_type, notnull, default, _pk)| ColumnInfo {
            name,
            data_type,
            is_nullable: notnull == 0,
            column_default: default,
        })
        .collect())
}

/// 验证所有核心表是否存在，返回缺失表名
pub async fn validate_schema(pool: &SqlitePool) -> Result<Vec<String>> {
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
        "entity_arc_stage",
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
        if !existing.iter().any(|t| t == table) {
            missing.push(table.to_string());
        }
    }

    Ok(missing)
}
