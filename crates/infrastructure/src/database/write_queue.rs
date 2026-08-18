//! Write Queue for serialized state mutations via PostgreSQL

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

#[derive(Debug)]
pub enum WriteCommand {
    StateCommit { project_id: uuid::Uuid, changes: Vec<StateChange> },
    CanonCommit { project_id: uuid::Uuid, rules: Vec<CanonRule> },
    ProjectMutation { project_id: uuid::Uuid, mutation: ProjectMutation },
}

#[derive(Debug, Clone)]
pub struct StateChange {
    pub entity_id: uuid::Uuid,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: String,
}

#[derive(Debug, Clone)]
pub struct CanonRule {
    pub rule_type: String,
    pub content: String,
    pub priority: i32,
}

#[derive(Debug)]
pub enum ProjectMutation {
    UpdateName(String),
    UpdateDescription(String),
    UpdateStatus(String),
}

/// Write queue backed by PostgreSQL
pub struct WriteQueue {
    pool: PgPool,
}

impl WriteQueue {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn process_state_commit(&self, project_id: uuid::Uuid, changes: Vec<StateChange>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for change in &changes {
            sqlx::query(
                "UPDATE entity SET attributes = jsonb_set(COALESCE(attributes, '{}'::jsonb), $1, $2::jsonb) WHERE id = $3 AND project_id = $4"
            )
            .bind(format!("{{{}}}", change.field))
            .bind(&change.new_value)
            .bind(change.entity_id)
            .bind(project_id)
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        info!("State commit completed for project {}", project_id);
        Ok(())
    }

    pub async fn process_canon_commit(&self, project_id: uuid::Uuid, rules: Vec<CanonRule>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for rule in &rules {
            sqlx::query(
                "INSERT INTO canon_rule (id, project_id, rule_type, content, priority, created_at) VALUES ($1, $2, $3, $4, $5, NOW())"
            )
            .bind(uuid::Uuid::new_v4())
            .bind(project_id)
            .bind(&rule.rule_type)
            .bind(&rule.content)
            .bind(rule.priority)
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        info!("Canon commit completed for project {}", project_id);
        Ok(())
    }

    pub async fn process_project_mutation(&self, project_id: uuid::Uuid, mutation: ProjectMutation) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        match mutation {
            ProjectMutation::UpdateName(n) => {
                sqlx::query("UPDATE project SET name=$1, updated_at=NOW() WHERE id=$2").bind(&n).bind(project_id).execute(&mut *tx).await?;
            }
            ProjectMutation::UpdateDescription(d) => {
                sqlx::query("UPDATE project SET description=$1, updated_at=NOW() WHERE id=$2").bind(&d).bind(project_id).execute(&mut *tx).await?;
            }
            ProjectMutation::UpdateStatus(s) => {
                sqlx::query("UPDATE project SET status=$1, updated_at=NOW() WHERE id=$2").bind(&s).bind(project_id).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        info!("Project mutation completed for project {}", project_id);
        Ok(())
    }
}