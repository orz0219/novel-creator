//! Database connection management for PostgreSQL
//!
//! 使用 sqlx::PgPool 管理数据库连接池，支持多进程并发访问。

use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Executor, Row};
use std::time::Duration;

/// 数据库连接管理器
///
/// PostgreSQL 通过连接池支持多写入者并发，不再需要 Mutex 串行化。
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
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
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context(format!("Failed to connect to PostgreSQL: {}", database_url))?;

        tracing::info!("PostgreSQL connection pool created");

        Ok(Self { pool })
    }

    /// 使用自定义配置创建数据库连接池
    pub async fn open_with_config(
        database_url: &str,
        max_connections: u32,
        min_connections: u32,
    ) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context(format!("Failed to connect to PostgreSQL: {}", database_url))?;

        tracing::info!(
            "PostgreSQL connection pool created (max={}, min={})",
            max_connections,
            min_connections
        );

        Ok(Self { pool })
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
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
        self.pool
            .acquire()
            .await
            .context("Failed to acquire connection for health check")?;
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

    #[tokio::test]
    async fn test_database_from_env() {
        // 仅在设置了 DATABASE_URL 时运行
        if std::env::var("DATABASE_URL").is_err() {
            eprintln!("Skipping test_database_from_env: DATABASE_URL not set");
            return;
        }
        let db = Database::from_env().await.unwrap();
        db.health_check().await.unwrap();
        db.close().await;
    }
}
