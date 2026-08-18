//! State Repository

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{CurrentState, ResourceState, StateChangeRecord};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct StateRepo<'a> { db: &'a Database }

impl<'a> StateRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn upsert_state(&self, project_id: Uuid, entity_id: Uuid, state_key: &str, state_value: serde_json::Value) -> Result<CurrentState> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute("UPDATE current_state SET effective_to = ? WHERE entity_id = ? AND state_key = ? AND effective_to IS NULL", [now.to_string(), entity_id.to_string(), state_key.to_string()]).context("Failed to invalidate")?;
        conn.execute("INSERT INTO current_state (id, project_id, entity_id, state_key, state_value, effective_from, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", [id.to_string(), project_id.to_string(), entity_id.to_string(), state_key.to_string(), state_value.to_string(), now.to_string(), now.to_string(), now.to_string()]).context("Failed to insert")?;
        Ok(CurrentState { id, project_id, entity_id, state_key: state_key.to_string(), state_value, effective_from: now, effective_to: None, created_at: now, updated_at: now })
    }

    pub fn get_current_state(&self, entity_id: Uuid, state_key: &str) -> Result<Option<CurrentState>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, created_at, updated_at FROM current_state WHERE entity_id = ? AND state_key = ? AND effective_to IS NULL",
            [entity_id.to_string(), state_key.to_string()],
            |row| {
                let eto: Option<String> = row.get(6)?;
                Ok(CurrentState {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    entity_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    state_key: row.get(3)?,
                    state_value: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    effective_from: get_timestamp(row, 5),
                    effective_to: eto.and_then(|t| t.parse().ok()),
                    created_at: get_timestamp(row, 7),
                    updated_at: get_timestamp(row, 8),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn list_current_states(&self, entity_id: Uuid) -> Result<Vec<CurrentState>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, entity_id, state_key, state_value, effective_from, effective_to, created_at, updated_at FROM current_state WHERE entity_id = ? AND effective_to IS NULL ORDER BY state_key").context("Failed to prepare")?;
        let rows = stmt.query_map([entity_id.to_string()], |row| {
            let eto: Option<String> = row.get(6)?;
            Ok(CurrentState {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                entity_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                state_key: row.get(3)?,
                state_value: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                effective_from: get_timestamp(row, 5),
                effective_to: eto.and_then(|t| t.parse().ok()),
                created_at: get_timestamp(row, 7),
                updated_at: get_timestamp(row, 8),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn record_change(&self, project_id: Uuid, event_id: Option<Uuid>, change_type: &str, target_entity_id: Uuid, state_key: &str, old_value: Option<serde_json::Value>, new_value: serde_json::Value, committed_by: Option<&str>) -> Result<StateChangeRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        
        let old_str = old_value.clone().map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        let new_str = new_value.to_string();
        let committed_by_str = committed_by.unwrap_or("").to_string();
        let now_str = now.to_string();
        
        let sql_str;
        let p0 = id.to_string();
        let p1 = project_id.to_string();
        let p2 = change_type.to_string();
        let p3 = target_entity_id.to_string();
        let p4 = state_key.to_string();
        let p5 = old_str;
        let p6 = new_str;
        let p7 = now_str;
        let p8 = committed_by_str;
        
        if let Some(eid) = event_id {
            sql_str = "INSERT INTO state_change (id, project_id, event_id, change_type, target_entity_id, state_key, old_value, new_value, committed_at, committed_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
            let p_eid = eid.to_string();
            conn.execute(sql_str, [&p0, &p1, &p_eid, &p2, &p3, &p4, &p5, &p6, &p7, &p8]).context("Failed to record")?;
        } else {
            sql_str = "INSERT INTO state_change (id, project_id, change_type, target_entity_id, state_key, old_value, new_value, committed_at, committed_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
            conn.execute(sql_str, [&p0, &p1, &p2, &p3, &p4, &p5, &p6, &p7, &p8]).context("Failed to record")?;
        }
        Ok(StateChangeRecord { id, project_id, event_id, change_type: change_type.to_string(), target_entity_id, state_key: state_key.to_string(), old_value, new_value, committed_at: now, committed_by: committed_by.map(|s| s.to_string()) })
    }

    pub fn upsert_resource(&self, project_id: Uuid, location_id: Uuid, resource_name: &str, quantity: Option<f64>, production_rate: Option<f64>, controlled_by: Option<Uuid>) -> Result<ResourceState> {
        let now = Utc::now();
        let conn = self.db.conn();
        
        let cb_str = controlled_by.map(|c| c.to_string());
        
        // 尝试更新现有记录
        let rows_updated = if let Some(ref cb) = cb_str {
            conn.execute(
                "UPDATE resource_state SET quantity = ?, production_rate = ?, controlled_by_entity_id = ?, updated_at = ? WHERE project_id = ? AND location_id = ? AND resource_name = ?",
                [
                    quantity.map(|q| q.to_string()).unwrap_or_default(),
                    production_rate.map(|p| p.to_string()).unwrap_or_default(),
                    cb.clone(),
                    now.to_string(),
                    project_id.to_string(),
                    location_id.to_string(),
                    resource_name.to_string(),
                ],
            ).context("Failed to update resource")?
        } else {
            conn.execute(
                "UPDATE resource_state SET quantity = ?, production_rate = ?, controlled_by_entity_id = NULL, updated_at = ? WHERE project_id = ? AND location_id = ? AND resource_name = ?",
                [
                    quantity.map(|q| q.to_string()).unwrap_or_default(),
                    production_rate.map(|p| p.to_string()).unwrap_or_default(),
                    now.to_string(),
                    project_id.to_string(),
                    location_id.to_string(),
                    resource_name.to_string(),
                ],
            ).context("Failed to update resource")?
        };
        
        if rows_updated == 0 {
            // 插入新记录
            let id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO resource_state (id, project_id, location_id, resource_name, quantity, production_rate, controlled_by_entity_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    id.to_string(), project_id.to_string(), location_id.to_string(),
                    resource_name.to_string(),
                    quantity.map(|q| q.to_string()).unwrap_or_default(),
                    production_rate.map(|p| p.to_string()).unwrap_or_default(),
                    cb_str.unwrap_or_default(),
                    now.to_string(), now.to_string(),
                ],
            ).context("Failed to insert resource")?;
            
            Ok(ResourceState { id, project_id, location_id, resource_name: resource_name.to_string(), quantity, production_rate, controlled_by_entity_id: controlled_by, created_at: now, updated_at: now })
        } else {
            // 返回更新后的记录
            let result = conn.query_row(
                "SELECT id, project_id, location_id, resource_name, quantity, production_rate, controlled_by_entity_id, created_at, updated_at FROM resource_state WHERE project_id = ? AND location_id = ? AND resource_name = ?",
                [project_id.to_string(), location_id.to_string(), resource_name.to_string()],
                |row| {
                    let cb: Option<String> = row.get(6)?;
                    Ok(ResourceState {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                        project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                        location_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                        resource_name: row.get(3)?,
                        quantity: row.get(4)?,
                        production_rate: row.get(5)?,
                        controlled_by_entity_id: cb.and_then(|c| Uuid::parse_str(&c).ok()),
                        created_at: crate::time_utils::get_timestamp(row, 7),
                        updated_at: crate::time_utils::get_timestamp(row, 8),
                    })
                },
            ).context("Failed to read updated resource")?;
            
            Ok(result)
        }
    }

    pub fn list_resources_by_location(&self, location_id: Uuid) -> Result<Vec<ResourceState>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, location_id, resource_name, quantity, production_rate, controlled_by_entity_id, created_at, updated_at FROM resource_state WHERE location_id = ? ORDER BY resource_name").context("Failed to prepare")?;
        let rows = stmt.query_map([location_id.to_string()], |row| {
            let cb: Option<String> = row.get(6)?;
            Ok(ResourceState {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                location_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                resource_name: row.get(3)?,
                quantity: row.get(4)?,
                production_rate: row.get(5)?,
                controlled_by_entity_id: cb.and_then(|c| Uuid::parse_str(&c).ok()),
                created_at: get_timestamp(row, 7),
                updated_at: get_timestamp(row, 8),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
