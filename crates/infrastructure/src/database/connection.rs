//! Database connection pool for DuckDB

use duckdb::{Connection, DuckdbConnectionManager};
use r2d2::Pool;
use std::sync::Arc;
use tracing::info;

/// Database connection pool wrapper
pub struct DatabasePool {
    pool: Pool<DuckdbConnectionManager>,
}

impl DatabasePool {
    /// Create a new database connection pool
    pub fn new(database_path: &str, max_connections: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = DuckdbConnectionManager::file(database_path)?;
        let pool = Pool::builder()
            .max_size(max_connections)
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

        info!("Database pool created with {} connections to {}", max_connections, database_path);

        Ok(Self { pool })
    }

    /// Create an in-memory database pool (for testing)
    pub fn new_in_memory(max_connections: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = DuckdbConnectionManager::file(":memory:")?;
        let pool = Pool::builder()
            .max_size(max_connections)
            .build(manager)
            .map_err(|e| format!("Failed to create in-memory pool: {}", e))?;

        info!("In-memory database pool created with {} connections", max_connections);

        Ok(Self { pool })
    }

    /// Get a connection from the pool
    pub fn get(&self) -> Result<r2d2::PooledConnection<DuckdbConnectionManager>, Box<dyn std::error::Error>> {
        self.pool.get().map_err(|e| format!("Failed to get connection: {}", e).into())
    }

    /// Get pool statistics
    pub fn statistics(&self) -> PoolStatistics {
        PoolStatistics {
            max_connections: self.pool.max_size(),
            idle_connections: self.pool.idle(),
        }
    }
}

/// Pool statistics
pub struct PoolStatistics {
    pub max_connections: u32,
    pub idle_connections: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_pool() {
        let pool = DatabasePool::new_in_memory(5).unwrap();
        let stats = pool.statistics();
        assert_eq!(stats.max_connections, 5);

        // Get a connection
        let conn = pool.get().unwrap();
        conn.execute_batch("CREATE TABLE test (id INTEGER, name VARCHAR)").unwrap();
        conn.execute_batch("INSERT INTO test VALUES (1, 'hello')").unwrap();
    }

    #[test]
    fn test_multiple_connections() {
        let pool = DatabasePool::new_in_memory(3).unwrap();

        let conn1 = pool.get().unwrap();
        let conn2 = pool.get().unwrap();

        conn1.execute_batch("CREATE TABLE shared (id INTEGER)").unwrap();
        conn2.execute_batch("INSERT INTO shared VALUES (1)").unwrap();

        let stats = pool.statistics();
        assert_eq!(stats.max_connections, 3);
    }
}
