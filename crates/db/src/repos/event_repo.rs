//! Event Repository - CRUD operations for Event

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::Event;
use sqlx::PgPool;
use uuid::Uuid;

pub struct EventRepo {
    pool: PgPool,
}

impl EventRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建事件
    pub async fn create(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<Event> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO event (id, project_id, name, description, event_type, timestamp, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(event_type.unwrap_or(""))
        .bind(timestamp.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create event")?;

        Ok(Event {
            id,
            project_id,
            name: name.to_string(),
            description: description.to_string(),
            event_type: event_type.map(|s| s.to_string()),
            timestamp: timestamp.map(|s| s.to_string()),
            event_time: None,
            duration: None,
            involved_entity_ids: Vec::new(),
            state_changes: Vec::new(),
            created_at: now,
            updated_at: now,
        })
    }

    /// 按 ID 获取事件
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Event>> {
        let row = sqlx::query_as::<_, EventRow>(
            "SELECT id, project_id, name, description, event_type, timestamp, event_time, duration, created_at, updated_at \
             FROM event WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query event")?;

        Ok(row.map(|r| r.into()))
    }

    /// 事务内创建世界事件（不可变：只 INSERT，不 UPDATE/DELETE）。
    pub async fn create_tx<'c>(
        executor: impl sqlx::Executor<'c, Database = sqlx::Postgres>,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<Event> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO event (id, project_id, name, description, event_type, timestamp, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(event_type.unwrap_or(""))
        .bind(timestamp.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(executor)
        .await
        .context("Failed to create event")?;

        Ok(Event {
            id,
            project_id,
            name: name.to_string(),
            description: description.to_string(),
            event_type: event_type.map(|s| s.to_string()),
            timestamp: timestamp.map(|s| s.to_string()),
            event_time: None,
            duration: None,
            involved_entity_ids: Vec::new(),
            state_changes: Vec::new(),
            created_at: now,
            updated_at: now,
        })
    }

    /// 列出项目中的所有事件
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Event>> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT id, project_id, name, description, event_type, timestamp, event_time, duration, created_at, updated_at \
             FROM event WHERE project_id = $1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query events")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 删除事件
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM event_entity WHERE event_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete event_entity")?;
        sqlx::query("DELETE FROM event WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete event")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: String,
    event_type: Option<String>,
    timestamp: Option<String>,
    event_time: Option<String>,
    duration: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EventRow> for Event {
    fn from(r: EventRow) -> Self {
        Event {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            event_type: r.event_type,
            timestamp: r.timestamp,
            event_time: r.event_time,
            duration: r.duration,
            involved_entity_ids: Vec::new(),
            state_changes: Vec::new(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
