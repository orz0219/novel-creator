//! Unit of Work pattern for PostgreSQL transactions

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::debug;

/// Unit of Work - atomic operations via PostgreSQL transaction
pub struct UnitOfWork {
    tx: Transaction<'static, Postgres>,
    committed: bool,
}

impl UnitOfWork {
    pub async fn begin(pool: &PgPool) -> Result<Self> {
        let tx = pool.begin().await.context("Failed to begin transaction")?;
        debug!("Transaction started");
        Ok(Self { tx, committed: false })
    }

    pub async fn commit(mut self) -> Result<()> {
        self.tx.commit().await.context("Failed to commit")?;
        self.committed = true;
        debug!("Transaction committed");
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        if !self.committed {
            self.tx.rollback().await.context("Failed to rollback")?;
            debug!("Transaction rolled back");
        }
        Ok(())
    }

    pub fn transaction(&mut self) -> &mut Transaction<'static, Postgres> {
        &mut self.tx
    }
}