//! Event Outbox Repository - Outbox pattern for reliable event delivery
//!
//! Events are written to outbox in the same transaction as the mutation.
//! A separate worker reads and delivers events.
//! This prevents:
//! - DB committed but event lost
//! - Event emitted but DB rolled back

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// Outbox event status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    Delivered,
    Failed,
}

impl OutboxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutboxStatus::Pending => "Pending",
            OutboxStatus::Delivered => "Delivered",
            OutboxStatus::Failed => "Failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Pending" => OutboxStatus::Pending,
            "Delivered" => OutboxStatus::Delivered,
            "Failed" => OutboxStatus::Failed,
            _ => OutboxStatus::Pending,
        }
    }
}

/// Outbox event
#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub payload: serde_json::Value,
    pub status: OutboxStatus,
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

pub struct OutboxRepo {
    pool: PgPool,
}

impl OutboxRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 在事务中写入 outbox 事件（必须与 mutation 在同一事务中）
    pub async fn enqueue_tx(
        conn: &mut PgConnection,
        project_id: Uuid,
        event_type: &str,
        aggregate_type: &str,
        aggregate_id: Uuid,
        payload: serde_json::Value,
    ) -> Result<OutboxEvent> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO event_outbox (id, project_id, event_type, aggregate_type, aggregate_id, payload, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'Pending', $7)"
        )
        .bind(id)
        .bind(project_id)
        .bind(event_type)
        .bind(aggregate_type)
        .bind(aggregate_id)
        .bind(&payload)
        .bind(now)
        .execute(&mut *conn)
        .await
        .context("Failed to enqueue outbox event")?;

        Ok(OutboxEvent {
            id,
            project_id,
            event_type: event_type.to_string(),
            aggregate_type: aggregate_type.to_string(),
            aggregate_id,
            payload,
            status: OutboxStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            delivered_at: None,
            error_message: None,
        })
    }

    /// 获取待处理的事件
    pub async fn list_pending(&self, limit: i32) -> Result<Vec<OutboxEvent>> {
        let rows = sqlx::query_as::<_, OutboxRow>(
            "SELECT id, project_id, event_type, aggregate_type, aggregate_id, payload, status, retry_count, max_retries, created_at, delivered_at, error_message \
             FROM event_outbox WHERE status = 'Pending' AND retry_count < max_retries ORDER BY created_at LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list pending outbox events")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 标记事件已投递
    pub async fn mark_delivered(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE event_outbox SET status = 'Delivered', delivered_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to mark outbox event as delivered")?;
        Ok(())
    }

    /// 标记事件投递失败
    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            "UPDATE event_outbox SET status = CASE WHEN retry_count + 1 >= max_retries THEN 'Failed' ELSE 'Pending' END, \
             retry_count = retry_count + 1, error_message = $1 WHERE id = $2"
        )
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to mark outbox event as failed")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct OutboxRow {
    id: Uuid,
    project_id: Uuid,
    event_type: String,
    aggregate_type: String,
    aggregate_id: Uuid,
    payload: serde_json::Value,
    status: String,
    retry_count: i32,
    max_retries: i32,
    created_at: DateTime<Utc>,
    delivered_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
}

impl From<OutboxRow> for OutboxEvent {
    fn from(r: OutboxRow) -> Self {
        OutboxEvent {
            id: r.id,
            project_id: r.project_id,
            event_type: r.event_type,
            aggregate_type: r.aggregate_type,
            aggregate_id: r.aggregate_id,
            payload: r.payload,
            status: OutboxStatus::from_str(&r.status),
            retry_count: r.retry_count,
            max_retries: r.max_retries,
            created_at: r.created_at,
            delivered_at: r.delivered_at,
            error_message: r.error_message,
        }
    }
}
