//! Event Repository - CRUD operations for Event

use anyhow::{Context, Result};
use chrono::Utc;
use domain::Event;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct EventRepo<'a> {
    db: &'a Database,
}

impl<'a> EventRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 创建事件
    pub fn create(&self, project_id: Uuid, name: &str, description: &str, event_type: Option<&str>, timestamp: Option<&str>) -> Result<Event> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO event (id, project_id, name, description, event_type, timestamp, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [id.to_string(), project_id.to_string(), name.to_string(), description.to_string(), event_type.unwrap_or("").to_string(), timestamp.unwrap_or("").to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create event")?;
        Ok(Event { id, project_id, name: name.to_string(), description: description.to_string(), event_type: event_type.map(|s| s.to_string()), timestamp: timestamp.map(|s| s.to_string()), event_time: None, duration: None, involved_entity_ids: Vec::new(), state_changes: Vec::new(), created_at: now, updated_at: now })
    }

    /// 按 ID 获取事件
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Event>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, name, description, event_type, timestamp, event_time, duration, created_at, updated_at FROM event WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(Event {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    name: row.get(2)?,
                    description: row.get(3)?,
                    event_type: row.get::<_, Option<String>>(4)?,
                    timestamp: row.get::<_, Option<String>>(5)?,
                    event_time: row.get::<_, Option<String>>(6)?,
                    duration: row.get::<_, Option<String>>(7)?,
                    involved_entity_ids: Vec::new(),
                    state_changes: Vec::new(),
                    created_at: get_timestamp(row, 8),
                    updated_at: get_timestamp(row, 9),
                })
            },
        ).ok();
        Ok(result)
    }

    /// 列出项目中的所有事件
    pub fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Event>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, event_type, timestamp, event_time, duration, created_at, updated_at FROM event WHERE project_id = ? ORDER BY created_at DESC",
        ).context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            Ok(Event {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                name: row.get(2)?,
                description: row.get(3)?,
                event_type: row.get::<_, Option<String>>(4)?,
                timestamp: row.get::<_, Option<String>>(5)?,
                event_time: row.get::<_, Option<String>>(6)?,
                duration: row.get::<_, Option<String>>(7)?,
                involved_entity_ids: Vec::new(),
                state_changes: Vec::new(),
                created_at: get_timestamp(row, 8),
                updated_at: get_timestamp(row, 9),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 删除事件
    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM event_entity WHERE event_id = ?", [id.to_string()]).context("Failed to delete")?;
        conn.execute("DELETE FROM event WHERE id = ?", [id.to_string()]).context("Failed to delete")?;
        Ok(())
    }
}
