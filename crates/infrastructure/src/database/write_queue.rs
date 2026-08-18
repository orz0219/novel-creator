//! Write Queue for serialized state mutations

use anyhow::Result;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use tracing::{info, error};

/// Write commands that need to be serialized
#[derive(Debug)]
pub enum WriteCommand {
    /// State commit operation
    StateCommit {
        project_id: uuid::Uuid,
        changes: Vec<StateChange>,
    },
    /// Canon commit operation
    CanonCommit {
        project_id: uuid::Uuid,
        rules: Vec<CanonRule>,
    },
    /// Project mutation
    ProjectMutation {
        project_id: uuid::Uuid,
        mutation: ProjectMutation,
    },
    /// Custom write operation
    Custom {
        name: String,
        handler: Box<dyn FnOnce() -> Result<()> + Send>,
    },
}

/// State change for write queue
#[derive(Debug, Clone)]
pub struct StateChange {
    pub entity_id: uuid::Uuid,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

/// Canon rule for write queue
#[derive(Debug, Clone)]
pub struct CanonRule {
    pub rule_type: String,
    pub content: String,
    pub priority: i32,
}

/// Project mutation types
#[derive(Debug)]
pub enum ProjectMutation {
    UpdateName(String),
    UpdateDescription(String),
    UpdateStatus(String),
}

/// Write queue for serialized state mutations
pub struct WriteQueue {
    sender: Sender<WriteCommand>,
    _worker: thread::JoinHandle<()>,
}

impl WriteQueue {
    /// Create a new write queue
    pub fn new(database_path: &str) -> Result<Self> {
        let (sender, receiver): (Sender<WriteCommand>, Receiver<WriteCommand>) = mpsc::channel();

        let worker = thread::spawn(move || {
            info!("Write queue worker started");
            Self::worker_loop(receiver, database_path);
        });

        Ok(Self {
            sender,
            _worker: worker,
        })
    }

    /// Submit a write command to the queue
    pub fn submit(&self, command: WriteCommand) -> Result<()> {
        self.sender.send(command).map_err(|e| {
            error!("Failed to submit write command: {}", e);
            anyhow::anyhow!("Failed to submit write command: {}", e)
        })?;
        Ok(())
    }

    /// Worker loop that processes write commands
    fn worker_loop(receiver: Receiver<WriteCommand>, database_path: &str) {
        // Initialize database connection for worker
        let conn = match duckdb::Connection::open(database_path) {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to open database for worker: {}", e);
                return;
            }
        };

        while let Ok(command) = receiver.recv() {
            match command {
                WriteCommand::StateCommit { project_id, changes } => {
                    if let Err(e) = Self::process_state_commit(&conn, project_id, changes) {
                        error!("Failed to process state commit: {}", e);
                    }
                }
                WriteCommand::CanonCommit { project_id, rules } => {
                    if let Err(e) = Self::process_canon_commit(&conn, project_id, rules) {
                        error!("Failed to process canon commit: {}", e);
                    }
                }
                WriteCommand::ProjectMutation { project_id, mutation } => {
                    if let Err(e) = Self::process_project_mutation(&conn, project_id, mutation) {
                        error!("Failed to process project mutation: {}", e);
                    }
                }
                WriteCommand::Custom { name, handler } => {
                    info!("Processing custom write command: {}", name);
                    if let Err(e) = handler() {
                        error!("Failed to process custom command {}: {}", name, e);
                    }
                }
            }
        }

        info!("Write queue worker stopped");
    }

    /// Process state commit
    fn process_state_commit(conn: &duckdb::Connection, project_id: uuid::Uuid, changes: Vec<StateChange>) -> Result<()> {
        conn.execute_batch("BEGIN TRANSACTION")?;

        for change in changes {
            conn.execute(
                "UPDATE entity SET attributes = json_set(attributes, ?, ?) WHERE id = ? AND project_id = ?",
                [&change.field, &change.new_value, &change.entity_id.to_string(), &project_id.to_string()],
            )?;
        }

        conn.execute_batch("COMMIT")?;
        info!("State commit completed for project {}", project_id);
        Ok(())
    }

    /// Process canon commit
    fn process_canon_commit(conn: &duckdb::Connection, project_id: uuid::Uuid, rules: Vec<CanonRule>) -> Result<()> {
        conn.execute_batch("BEGIN TRANSACTION")?;

        for rule in rules {
            conn.execute(
                "INSERT INTO canon_rules (project_id, rule_type, content, priority) VALUES (?, ?, ?, ?)",
                [&project_id.to_string(), &rule.rule_type, &rule.content, &rule.priority.to_string()],
            )?;
        }

        conn.execute_batch("COMMIT")?;
        info!("Canon commit completed for project {}", project_id);
        Ok(())
    }

    /// Process project mutation
    fn process_project_mutation(conn: &duckdb::Connection, project_id: uuid::Uuid, mutation: ProjectMutation) -> Result<()> {
        conn.execute_batch("BEGIN TRANSACTION")?;

        match mutation {
            ProjectMutation::UpdateName(name) => {
                conn.execute(
                    "UPDATE project SET name = ? WHERE id = ?",
                    [&name, &project_id.to_string()],
                )?;
            }
            ProjectMutation::UpdateDescription(desc) => {
                conn.execute(
                    "UPDATE project SET description = ? WHERE id = ?",
                    [&desc, &project_id.to_string()],
                )?;
            }
            ProjectMutation::UpdateStatus(status) => {
                conn.execute(
                    "UPDATE project SET status = ? WHERE id = ?",
                    [&status, &project_id.to_string()],
                )?;
            }
        }

        conn.execute_batch("COMMIT")?;
        info!("Project mutation completed for project {}", project_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_write_queue_submit() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let queue = WriteQueue::new(":memory:").unwrap();

        // Create test table
        {
            let conn = pool.get().unwrap();
            conn.execute_batch("CREATE TABLE test (id INTEGER, name VARCHAR)").unwrap();
        }

        // Submit a write command
        let result = queue.submit(WriteCommand::Custom {
            name: "test".to_string(),
            handler: Box::new(|| {
                info!("Test command executed");
                Ok(())
            }),
        });

        assert!(result.is_ok());
        thread::sleep(std::time::Duration::from_millis(100)); // Wait for worker
    }

    #[test]
    fn test_write_queue_state_commit() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let queue = WriteQueue::new(":memory:").unwrap();

        // Create test table
        {
            let conn = pool.get().unwrap();
            conn.execute_batch("CREATE TABLE entity (id VARCHAR, attributes JSON)").unwrap();
            conn.execute_batch("INSERT INTO entity VALUES ('123', '{\"name\": \"test\"}')").unwrap();
        }

        // Submit state commit
        let result = queue.submit(WriteCommand::StateCommit {
            project_id: uuid::Uuid::new_v4(),
            changes: vec![StateChange {
                entity_id: uuid::Uuid::parse_str("123").unwrap(),
                field: "name".to_string(),
                old_value: Some("test".to_string()),
                new_value: "updated".to_string(),
            }],
        });

        assert!(result.is_ok());
        thread::sleep(std::time::Duration::from_millis(100));
    }
}
