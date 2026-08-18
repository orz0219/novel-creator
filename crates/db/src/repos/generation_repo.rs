//! Generation Repository

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{GenerationTask, Skill, SkillStatus, SkillType, TaskStatus};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::{get_optional_timestamp, get_timestamp};

pub struct SkillRepo<'a> { db: &'a Database }

impl<'a> SkillRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, name: &str, description: Option<&str>, skill_type: SkillType, prompt_template: &str, input_schema: Option<serde_json::Value>, output_schema: Option<serde_json::Value>, default_params: serde_json::Value) -> Result<Skill> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let type_str = crate::ser::skill_type_str(&skill_type);
        let conn = self.db.conn();
        conn.execute("INSERT INTO skill (id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?, 'Draft', ?, ?)", [id.to_string(), name.to_string(), description.unwrap_or("").to_string(), type_str, prompt_template.to_string(), input_schema.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "{}".to_string()), output_schema.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "{}".to_string()), default_params.to_string(), now.to_string(), now.to_string()]).context("Failed to create")?;
        Ok(Skill { id, name: name.to_string(), description: description.map(|s| s.to_string()), skill_type, version: 1, prompt_template: prompt_template.to_string(), input_schema, output_schema, default_params, status: SkillStatus::Draft, created_at: now, updated_at: now })
    }

    pub fn get_by_name(&self, name: &str) -> Result<Option<Skill>> {
        let conn = self.db.conn();
        let result = conn.query_row("SELECT id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at FROM skill WHERE name = ?", [name], |row| {
            Ok(Skill {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                description: row.get(2)?,
                skill_type: crate::ser::parse_skill_type(&row.get::<_, String>(3)?),
                version: row.get(4)?,
                prompt_template: row.get(5)?,
                input_schema: row.get::<_, Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                output_schema: row.get::<_, Option<String>>(7)?.and_then(|s| serde_json::from_str(&s).ok()),
                default_params: row.get::<_, Option<String>>(8)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                status: crate::ser::parse_skill_status(&row.get::<_, String>(9)?),
                created_at: get_timestamp(row, 10),
                updated_at: get_timestamp(row, 11),
            })
        }).ok();
        Ok(result)
    }

    pub fn list_all(&self) -> Result<Vec<Skill>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, name, description, skill_type, version, prompt_template, input_schema, output_schema, default_params, status, created_at, updated_at FROM skill ORDER BY name").context("Failed to prepare")?;
        let rows = stmt.query_map([], |row| {
            Ok(Skill {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                description: row.get(2)?,
                skill_type: crate::ser::parse_skill_type(&row.get::<_, String>(3)?),
                version: row.get(4)?,
                prompt_template: row.get(5)?,
                input_schema: row.get::<_, Option<String>>(6)?.and_then(|s| serde_json::from_str(&s).ok()),
                output_schema: row.get::<_, Option<String>>(7)?.and_then(|s| serde_json::from_str(&s).ok()),
                default_params: row.get::<_, Option<String>>(8)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                status: crate::ser::parse_skill_status(&row.get::<_, String>(9)?),
                created_at: get_timestamp(row, 10),
                updated_at: get_timestamp(row, 11),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

pub struct TaskRepo<'a> { db: &'a Database }

impl<'a> TaskRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create(&self, project_id: Uuid, skill_id: Uuid, scene_id: Option<Uuid>, input: serde_json::Value) -> Result<GenerationTask> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        
        if let Some(sid) = scene_id {
            conn.execute("INSERT INTO generation_task (id, project_id, skill_id, scene_id, input, status, created_at) VALUES (?, ?, ?, ?, ?, 'Pending', ?)", [id.to_string(), project_id.to_string(), skill_id.to_string(), sid.to_string(), input.to_string(), now.to_string()]).context("Failed to create")?;
        } else {
            conn.execute("INSERT INTO generation_task (id, project_id, skill_id, input, status, created_at) VALUES (?, ?, ?, ?, 'Pending', ?)", [id.to_string(), project_id.to_string(), skill_id.to_string(), input.to_string(), now.to_string()]).context("Failed to create")?;
        }
        Ok(GenerationTask { id, project_id, skill_id, scene_id, input, output: None, status: TaskStatus::Pending, token_usage: None, error: None, created_at: now, completed_at: None })
    }

    pub fn update_status(&self, task_id: Uuid, status: TaskStatus, output: Option<serde_json::Value>, error: Option<&str>) -> Result<()> {
        let conn = self.db.conn();
        let status_str = crate::ser::task_status_str(&status);
        let output_str = output.map(|o| o.to_string()).unwrap_or_else(|| "{}".to_string());
        conn.execute("UPDATE generation_task SET status = ?, output = ?, error = ?, completed_at = ? WHERE id = ?", [status_str, output_str, error.unwrap_or("").to_string(), Utc::now().to_string(), task_id.to_string()]).context("Failed to update")?;
        Ok(())
    }

    pub fn get_by_id(&self, task_id: Uuid) -> Result<Option<GenerationTask>> {
        let conn = self.db.conn();
        let result = conn.query_row("SELECT id, project_id, skill_id, scene_id, input, output, status, error, created_at, completed_at FROM generation_task WHERE id = ?", [task_id.to_string()], |row| {
            let scene: Option<String> = row.get(3)?;
            let output: Option<String> = row.get(5)?;
            let error: Option<String> = row.get(7)?;
            Ok(GenerationTask {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                skill_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                scene_id: scene.and_then(|s| Uuid::parse_str(&s).ok()),
                input: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                output: output.and_then(|o| serde_json::from_str(&o).ok()),
                status: TaskStatus::Pending,
                token_usage: None,
                error,
                created_at: get_timestamp(row, 8),
                completed_at: get_optional_timestamp(row, 9),
            })
        }).ok();
        Ok(result)
    }
}
