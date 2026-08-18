//! Project Repository - CRUD operations for Project

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Project, ProjectStatus};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ser;

pub struct ProjectRepo {
    pool: PgPool,
}

impl ProjectRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, description: Option<&str>) -> Result<Project> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let config = serde_json::json!({});

        sqlx::query(
            "INSERT INTO project (id, name, description, config, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'Concept', $5, $6)",
        )
        .bind(id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(&config)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
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
            status: ProjectStatus::Concept,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, name, description, language, world_setting, system_setting, \
             default_model, default_style, default_params, config, status, created_at, updated_at \
             FROM project WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query project")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_all(&self) -> Result<Vec<Project>> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, name, description, language, world_setting, system_setting, \
             default_model, default_style, default_params, config, status, created_at, updated_at \
             FROM project ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query projects")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(&self, project: &Project) -> Result<()> {
        sqlx::query(
            "UPDATE project SET name = $1, description = $2, world_setting = $3, \
             system_setting = $4, config = $5, status = $6, updated_at = $7 WHERE id = $8",
        )
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.world_setting)
        .bind(&project.system_setting)
        .bind(&project.config)
        .bind(ser::project_status_str(&project.status))
        .bind(Utc::now())
        .bind(project.id)
        .execute(&self.pool)
        .await
        .context("Failed to update project")?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM project WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete project")?;
        Ok(())
    }
}

/// sqlx row mapping for Project
#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    language: Option<String>,
    world_setting: Option<String>,
    system_setting: Option<String>,
    default_model: Option<String>,
    default_style: Option<String>,
    default_params: Option<serde_json::Value>,
    config: Option<serde_json::Value>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProjectRow> for Project {
    fn from(r: ProjectRow) -> Self {
        Project {
            id: r.id,
            name: r.name,
            description: r.description,
            language: r.language,
            world_setting: r.world_setting,
            system_setting: r.system_setting,
            default_model: r.default_model,
            default_style: r.default_style,
            default_params: r.default_params.unwrap_or_default(),
            config: r.config.unwrap_or_default(),
            status: ser::parse_project_status(&r.status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests require DATABASE_URL to be set
    // Run with: cargo test --package db -- --ignored
}
