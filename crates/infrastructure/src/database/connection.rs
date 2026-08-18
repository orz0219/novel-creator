//! Database connection pool for PostgreSQL

use anyhow::{Context, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

/// Database connection pool wrapper for PostgreSQL
#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Create pool from DATABASE_URL environment variable
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
        Self::new(&url, 10, 2).await
    }

    /// Create pool with custom config
    pub async fn new(database_url: &str, max_conn: u32, min_conn: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_conn)
            .min_connections(min_conn)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .context(format!("Failed to connect to PostgreSQL: {}", database_url))?;
        info!("PostgreSQL pool created (max={}, min={})", max_conn, min_conn);
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool { &self.pool }

    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await.context("Health check failed")?;
        Ok(())
    }

    pub async fn close(&self) { self.pool.close().await; }

    pub fn statistics(&self) -> PoolStatistics {
        PoolStatistics { size: self.pool.size(), idle: self.pool.num_idle() }
    }
}

pub struct PoolStatistics {
    pub size: u32,
    pub idle: usize,
}
