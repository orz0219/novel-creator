//! Database connection management for DuckDB

use anyhow::{Context, Result};
use duckdb::Connection;
use std::path::Path;
use std::sync::Mutex;

/// 数据库连接管理器
///
/// DuckDB 在嵌入式模式下不支持多写入者，使用 Mutex 串行化写操作。
pub struct Database {
    conn: Mutex<Connection>,
    path: String,
}

impl Database {
    /// 创建或打开数据库
    pub fn open(path: &str) -> Result<Self> {
        // 确保目录存在
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        let conn = Connection::open(path)
            .context(format!("Failed to open DuckDB at {}", path))?;

        // 启用 WAL 模式以支持更好的并发读取
        conn.execute_batch("PRAGMA enable_progress_bar")
            .context("Failed to set pragmas")?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_string(),
        })
    }

    /// 创建内存数据库（用于测试）
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .context("Failed to open in-memory DuckDB")?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: ":memory:".to_string(),
        })
    }

    /// 获取连接的只读引用
    ///
    /// 用于查询操作，不需要独占锁。
    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("Database lock poisoned")
    }

    /// 获取可写的连接引用
    ///
    /// 用于写入操作，需要独占锁。
    pub fn conn_mut(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("Database lock poisoned")
    }

    /// 执行 SQL 批量语句
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock().expect("Database lock poisoned");
        conn.execute_batch(sql)
            .context("Failed to execute SQL batch")
    }

    /// 获取数据库路径
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.path(), ":memory:");
    }

    #[test]
    fn test_execute_batch() {
        let db = Database::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE test (id INTEGER, name VARCHAR)")
            .unwrap();
    }
}
