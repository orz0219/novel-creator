//! Unit of Work pattern for atomic transactions

use anyhow::Result;
use duckdb::Connection;
use tracing::{debug, error};

/// Unit of Work - ensures atomic operations across multiple repositories
pub struct UnitOfWork<'a> {
    conn: &'a Connection,
    committed: bool,
}

impl<'a> UnitOfWork<'a> {
    /// Begin a new unit of work (transaction)
    pub fn begin(conn: &'a Connection) -> Result<Self> {
        conn.execute_batch("BEGIN TRANSACTION")?;
        debug!("Transaction started");
        Ok(Self {
            conn,
            committed: false,
        })
    }

    /// Commit the transaction
    pub fn commit(mut self) -> Result<()> {
        self.conn.execute_batch("COMMIT")?;
        self.committed = true;
        debug!("Transaction committed");
        Ok(())
    }

    /// Rollback the transaction
    pub fn rollback(mut self) -> Result<()> {
        if !self.committed {
            self.conn.execute_batch("ROLLBACK")?;
            debug!("Transaction rolled back");
        }
        Ok(())
    }

    /// Execute a closure within the transaction
    pub fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        f(self.conn)
    }

    /// Execute a batch of SQL statements
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.conn.execute_batch(sql).map_err(|e| {
            error!("Failed to execute batch: {}", e);
            anyhow::anyhow!("Batch execution failed: {}", e)
        })
    }

    /// Execute a single SQL statement
    pub fn execute(&self, sql: &str, params: &[&dyn duckdb::types::ToSql]) -> Result<usize> {
        self.conn.execute(sql, params).map_err(|e| {
            error!("Failed to execute: {}", e);
            anyhow::anyhow!("Execution failed: {}", e)
        })
    }

    /// Get a reference to the underlying connection (for reading)
    pub fn connection(&self) -> &Connection {
        self.conn
    }
}

impl<'a> Drop for UnitOfWork<'a> {
    fn drop(&mut self) {
        if !self.committed {
            if let Err(e) = self.conn.execute_batch("ROLLBACK") {
                error!("Failed to rollback on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_unit_of_work_commit() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        conn.execute_batch("CREATE TABLE test (id INTEGER, name VARCHAR)").unwrap();

        {
            let uow = UnitOfWork::begin(&conn).unwrap();
            uow.execute_batch("INSERT INTO test VALUES (1, 'hello')").unwrap();
            uow.commit().unwrap();
        }

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_unit_of_work_rollback() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        conn.execute_batch("CREATE TABLE test (id INTEGER, name VARCHAR)").unwrap();

        {
            let uow = UnitOfWork::begin(&conn).unwrap();
            uow.execute_batch("INSERT INTO test VALUES (1, 'hello')").unwrap();
            uow.rollback().unwrap();
        }

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_unit_of_work_automatic_rollback() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        conn.execute_batch("CREATE TABLE test (id INTEGER, name VARCHAR)").unwrap();

        {
            let _uow = UnitOfWork::begin(&conn).unwrap();
            conn.execute_batch("INSERT INTO test VALUES (1, 'hello')").unwrap();
            // Dropped without commit - should auto-rollback
        }

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM test", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
