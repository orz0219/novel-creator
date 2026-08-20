//! StateCommitter - transactional commit of state changes.
//!
//! The runtime component is a thin orchestration wrapper: the actual database
//! transaction lives behind the StateCommitterPort trait, implemented by the
//! db crate. This keeps the runtime free of any PostgreSQL / sqlx dependency.

use anyhow::Result;
use domain::ports::StateCommitterPort;
use domain::validation::CommitResponse;
use std::sync::Arc;
use uuid::Uuid;

/// Runtime-side StateCommitter. Delegates the canonical write boundary to the
/// injected port implementation.
pub struct DbStateCommitter {
    port: Arc<dyn StateCommitterPort>,
}

impl DbStateCommitter {
    pub fn new(port: Arc<dyn StateCommitterPort>) -> Self {
        Self { port }
    }

    /// Commit a batch of approved ProposedChanges atomically.
    pub async fn commit(
        &self,
        project_id: Uuid,
        change_ids: &[Uuid],
    ) -> Result<CommitResponse> {
        self.port.commit(project_id, change_ids).await
    }
}
