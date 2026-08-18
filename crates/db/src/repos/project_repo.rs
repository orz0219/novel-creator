//! Project Repository - CRUD operations for Project

use anyhow::{Context, Result};
use chrono::Utc;
use domain::Project;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct ProjectRepo<'a> {
    db: &'a Database,
}

impl<'a> ProjectRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(&self, name: &str, description: Option<&str>) -> Result<Project> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let config = serde_json::json!({});

        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO project (id, name, description, config, status, created_at, updated_at) VALUES (?, ?, ?, ?, 'Concept', ?, ?)",
            [
                id.to_string(),
                name.to_string(),
                description.unwrap_or("").to_string(),
                config.to_string(),
                now.to_string(),
                now.to_string(),
            ],
        )
        .context("Failed to create project")?;

        Ok(Project {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            language: None,
            world_setting: None,
            system_setting: None,
            default_model: None,
            default_style: None,
            default_params: serde_json::json!({}),
            config,
            status: domain::ProjectStatus::Concept,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, language, world_setting, system_setting, default_model, default_style, default_params, config, status, created_at, updated_at
                 FROM project WHERE id = ?",
            )
            .context("Failed to prepare query")?;

        let result = stmt
            .query_row([id.to_string()], |row| {
                Ok(Project {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    language: row.get::<_, Option<String>>(3)?,
                    world_setting: row.get::<_, Option<String>>(4)?,
                    system_setting: row.get::<_, Option<String>>(5)?,
                    default_model: row.get::<_, Option<String>>(6)?,
                    default_style: row.get::<_, Option<String>>(7)?,
                    default_params: row.get::<_, Option<String>>(8)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                    config: row.get::<_, Option<String>>(9)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                    status: crate::ser::parse_project_status(&row.get::<_, String>(10)?),
                    created_at: get_timestamp(row, 11),
                    updated_at: get_timestamp(row, 12),
                })
            })
            .ok();

        Ok(result)
    }

    pub fn list_all(&self) -> Result<Vec<Project>> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, language, world_setting, system_setting, default_model, default_style, default_params, config, status, created_at, updated_at
                 FROM project ORDER BY created_at DESC",
            )
            .context("Failed to prepare query")?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    language: row.get::<_, Option<String>>(3)?,
                    world_setting: row.get::<_, Option<String>>(4)?,
                    system_setting: row.get::<_, Option<String>>(5)?,
                    default_model: row.get::<_, Option<String>>(6)?,
                    default_style: row.get::<_, Option<String>>(7)?,
                    default_params: row.get::<_, Option<String>>(8)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                    config: row.get::<_, Option<String>>(9)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
                    status: crate::ser::parse_project_status(&row.get::<_, String>(10)?),
                    created_at: get_timestamp(row, 11),
                    updated_at: get_timestamp(row, 12),
                })
            })
            .context("Failed to query projects")?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update(&self, project: &Project) -> Result<()> {
        let conn = self.db.conn();
        conn.execute(
            "UPDATE project SET name = ?, description = ?, world_setting = ?, system_setting = ?,
             config = ?, status = ?, updated_at = ? WHERE id = ?",
            [
                project.name.clone(),
                project.description.clone().unwrap_or_default(),
                project.world_setting.clone().unwrap_or_default(),
                project.system_setting.clone().unwrap_or_default(),
                project.config.to_string(),
                crate::ser::project_status_str(&project.status),
                Utc::now().to_string(),
                project.id.to_string(),
            ],
        )
        .context("Failed to update project")?;
        Ok(())
    }

    pub fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.db.conn();
        conn.execute("DELETE FROM project WHERE id = ?", [id.to_string()])
            .context("Failed to delete project")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        crate::migration::run_migrations(&db, migrations_dir).unwrap();
        db
    }

    #[test]
    fn test_create_and_get_project() {
        let db = setup_db();
        let repo = ProjectRepo::new(&db);
        let project = repo.create("Test Novel", Some("A test novel")).unwrap();
        assert_eq!(project.name, "Test Novel");
        let fetched = repo.get_by_id(project.id).unwrap().unwrap();
        assert_eq!(fetched.id, project.id);
    }

    #[test]
    fn test_list_projects() {
        let db = setup_db();
        let repo = ProjectRepo::new(&db);
        repo.create("Novel 1", None).unwrap();
        repo.create("Novel 2", None).unwrap();
        assert_eq!(repo.list_all().unwrap().len(), 2);
    }

    #[test]
    fn test_delete_project() {
        let db = setup_db();
        let repo = ProjectRepo::new(&db);
        let project = repo.create("To Delete", None).unwrap();
        repo.delete(project.id).unwrap();
        assert!(repo.get_by_id(project.id).unwrap().is_none());
    }
}
