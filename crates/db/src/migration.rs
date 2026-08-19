//! Database migration runner for PostgreSQL
//!
//! 按顺序执行 SQL migration 文件，确保幂等性。

use anyhow::{Context, Result};
use sqlx::PgPool;

/// 运行所有 migration
pub async fn run_migrations(pool: &PgPool, migrations_dir: &str) -> Result<Vec<String>> {
    let mut executed = Vec::new();

    // 创建 migration 追踪表
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL UNIQUE,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create migrations table")?;

    // 获取已执行的 migration
    let applied: Vec<String> = sqlx::query_scalar("SELECT name FROM _migrations ORDER BY id")
        .fetch_all(pool)
        .await
        .context("Failed to query migrations")?;

    // 读取 migration 文件
    let dir = std::path::Path::new(migrations_dir);
    if !dir.exists() {
        tracing::info!("No migrations directory found at {}", migrations_dir);
        return Ok(executed);
    }

    let mut files: Vec<String> = std::fs::read_dir(dir)
        .context(format!("Failed to read migrations dir: {}", migrations_dir))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "sql")
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    files.sort();

    for file in files {
        if applied.contains(&file) {
            continue;
        }

        let path = dir.join(&file);
        let sql = std::fs::read_to_string(&path)
            .context(format!("Failed to read migration: {}", path.display()))?;

        tracing::info!("Applying migration: {}", file);

        sqlx::raw_sql(&sql)
            .execute(pool)
            .await
            .context(format!("Failed to apply migration: {}", file))?;

        // 记录已执行
        sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
            .bind(&file)
            .execute(pool)
            .await
            .context("Failed to record migration")?;

        executed.push(file);
    }

    Ok(executed)
}

/// 回滚最后一个 migration（简单实现，仅删除记录）
pub async fn rollback_last(pool: &PgPool) -> Result<Option<String>> {
    let last: Option<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM _migrations ORDER BY id DESC LIMIT 1")
            .fetch_optional(pool)
            .await
            .context("Failed to query last migration")?;

    if let Some((id, name)) = last {
        sqlx::query("DELETE FROM _migrations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to delete migration record")?;

        tracing::warn!(
            "Rolled back migration record: {} (tables not dropped)",
            name
        );
        Ok(Some(name))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_dir_exists() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        assert!(
            std::path::Path::new(dir).exists(),
            "Migrations directory should exist at {}",
            dir
        );

        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "sql")
                    .unwrap_or(false)
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        files.sort();
        assert_eq!(files.len(), 1, "Expected 1 canonical migration file");
        assert_eq!(files[0], "001_canonical_schema.sql");
    }
}