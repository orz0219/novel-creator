//! Database migration runner
//!
//! 按顺序执行 SQL migration 文件，确保幂等性。

use anyhow::{Context, Result};
use crate::connection::Database;
use std::fs;
use std::path::Path;

/// 运行所有 migration
pub fn run_migrations(db: &Database, migrations_dir: &str) -> Result<Vec<String>> {
    let mut executed = Vec::new();

    // 创建 migration 追踪表
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            name VARCHAR NOT NULL,
            applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("Failed to create migrations table")?;

    // 获取已执行的 migration
    let applied: Vec<String> = {
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT name FROM _migrations ORDER BY id")
            .context("Failed to query migrations")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))
            .context("Failed to read migrations")?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // 读取 migration 文件
    let dir = Path::new(migrations_dir);
    if !dir.exists() {
        tracing::info!("No migrations directory found at {}", migrations_dir);
        return Ok(executed);
    }

    let mut files: Vec<String> = fs::read_dir(dir)
        .context(format!("Failed to read migrations dir: {}", migrations_dir))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
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
        let sql = fs::read_to_string(&path)
            .context(format!("Failed to read migration: {}", path.display()))?;

        tracing::info!("Applying migration: {}", file);

        db.execute_batch(&sql)
            .context(format!("Failed to apply migration: {}", file))?;

        // 记录已执行
        {
            let conn = db.conn();
            let next_id = applied.len() as i32 + executed.len() as i32 + 1;
            conn.execute(
                "INSERT INTO _migrations (id, name) VALUES (?, ?)",
                [next_id.to_string(), file.clone()],
            )
            .context("Failed to record migration")?;
        }

        executed.push(file);
    }

    Ok(executed)
}

/// 回滚最后一个 migration（简单实现，仅删除表）
pub fn rollback_last(db: &Database) -> Result<Option<String>> {
    let conn = db.conn();

    let last: Option<(i32, String)> = conn
        .prepare("SELECT id, name FROM _migrations ORDER BY id DESC LIMIT 1")
        .context("Failed to query last migration")?
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
        .ok();

    if let Some((id, name)) = last {
        // 简单回滚：删除 migration 记录
        // 注意：不删除已创建的表（需要更复杂的回滚逻辑）
        conn.execute("DELETE FROM _migrations WHERE id = ?", [id])
            .context("Failed to delete migration record")?;

        tracing::warn!("Rolled back migration record: {} (tables not dropped)", name);
        Ok(Some(name))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_migrations_empty() {
        let db = Database::open_in_memory().unwrap();
        let dir = "/tmp/test_migrations_empty";
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(dir).unwrap();

        let result = run_migrations(&db, dir).unwrap();
        assert!(result.is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_run_migrations_with_file() {
        let db = Database::open_in_memory().unwrap();
        let dir = "/tmp/test_migrations_with_file";
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(dir).unwrap();

        fs::write(
            format!("{}/001_test.sql", dir),
            "CREATE TABLE test_mig (id INTEGER PRIMARY KEY, name VARCHAR);",
        )
        .unwrap();

        let result = run_migrations(&db, dir).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "001_test.sql");

        // 再次运行，应该跳过已执行的
        let result2 = run_migrations(&db, dir).unwrap();
        assert!(result2.is_empty());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_run_full_schema_migration() {
        let db = Database::open_in_memory().unwrap();
        // 使用项目根目录下的 migration 文件
        let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        let result = run_migrations(&db, migrations_dir).unwrap();
        assert_eq!(result.len(), 7);
        assert_eq!(result[0], "001_initial_schema.sql");
        assert_eq!(result[1], "002_design3_phase1.sql");
        assert_eq!(result[2], "003_design3_phase2.sql");
        assert_eq!(result[3], "004_design3_phase3.sql");
        assert_eq!(result[4], "005_design4_v2.sql");
        assert_eq!(result[5], "006_design6_phase1.sql");
        assert_eq!(result[6], "007_schema_alignment.sql");

        // 验证核心表存在
        let missing = crate::schema::validate_schema(&db).unwrap();
        assert!(missing.is_empty(), "Missing tables: {:?}", missing);
    }
}
