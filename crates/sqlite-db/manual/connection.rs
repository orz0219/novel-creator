//! 数据库连接管理（SQLite）（手工维护）
//!
//! 与电脑端 `crates/db/src/connection.rs` 的差异：
//!   - 连接池换成 `SqlitePool`（本地文件库，不需要网络与账号）
//!   - 测试用**真实文件库**而不是 `:memory:`：连接池会开多条连接，
//!     每条内存连接都是各自独立的库，建表与查询会落到不同连接上
//!
//! 之所以整份手工维护：PG 版的单元测试依赖 `DATABASE_URL`（PostgreSQL），
//! 机械转换后在这里必然失败；而 SQLite 需要的测试内容与 PG 完全不同，
//! 无法由转换脚本推导（与 export.rs / import.rs 同一处理方式）。

use anyhow::{Context, Result};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Executor;
use std::time::Duration;

/// 数据库连接管理器
///
/// 单机 SQLite 文件库，连接池用于复用连接、避免每次请求都重新打开文件。
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 从环境变量 DATABASE_URL 创建数据库连接池
    pub async fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable not set")?;
        Self::open(&database_url).await
    }

    /// 使用指定 URL 创建数据库连接池
    pub async fn open(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context("Failed to connect to SQLite")?;

        tracing::info!("SQLite connection pool created");

        Ok(Self { pool })
    }

    /// 使用自定义配置创建数据库连接池
    pub async fn open_with_config(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context("Failed to connect to SQLite")?;

        tracing::info!(
            "SQLite connection pool created (max={}, min={})",
            max_connections,
            min_connections
        );

        Ok(Self { pool })
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 执行 SQL 批量语句（用于 migration）
    pub async fn execute_batch(&self, sql: &str) -> Result<()> {
        self.pool
            .execute(sql)
            .await
            .context("Failed to execute SQL batch")?;
        Ok(())
    }

    /// 执行带参数的 SQL 语句
    pub async fn execute(&self, sql: &str) -> Result<u64> {
        let result = self.pool
            .execute(sql)
            .await
            .context("Failed to execute SQL")?;
        Ok(result.rows_affected())
    }

    /// 检查数据库连接是否健康
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .context("Health check failed: SELECT 1 returned error")?;
        Ok(())
    }

    /// 关闭连接池
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SQLite 下测试连接与健康检查。
    ///
    /// 注意用真实文件库而不是 `:memory:`：连接池会开多条连接，
    /// 每条内存连接都是各自独立的库，建表与查询会落到不同连接上。
    #[tokio::test]
    async fn test_database_health_check() {
        let dir = std::env::temp_dir().join(format!("novel-conn-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("t.db");
        let url = format!("sqlite://{}?mode=rwc", db_path.display());

        let db = Database::open(&url).await.expect("打开数据库失败");
        db.health_check().await.expect("健康检查失败");
        db.close().await;

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 打开不存在的路径应当报错，而不是静默成功
    #[tokio::test]
    async fn test_database_rejects_bad_path() {
        // 目录不存在且不允许创建时，打开应失败
        let url = "sqlite:///nonexistent-dir-xyz/definitely-missing.db?mode=ro";
        let r = Database::open(url).await;
        assert!(r.is_err(), "以只读模式打开不存在的库应当失败");
    }
}