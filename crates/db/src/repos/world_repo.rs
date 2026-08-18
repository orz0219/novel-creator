//! World Repository - CRUD operations for World

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::World;
use sqlx::PgPool;
use uuid::Uuid;

pub struct WorldRepo {
    pool: PgPool,
}

impl WorldRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let config = serde_json::json!({});

        sqlx::query(
            "INSERT INTO world (id, project_id, name, description, world_rules, config, is_main, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(world_rules.unwrap_or(""))
        .bind(&config)
        .bind(is_main)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create world")?;

        Ok(World {
            id,
            project_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            world_rules: world_rules.map(|s| s.to_string()),
            config,
            is_main,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<World>> {
        let row = sqlx::query_as::<_, WorldRow>(
            "SELECT id, project_id, name, description, world_rules, config, is_main, created_at, updated_at \
             FROM world WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query world")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>> {
        let row = sqlx::query_as::<_, WorldRow>(
            "SELECT id, project_id, name, description, world_rules, config, is_main, created_at, updated_at \
             FROM world WHERE project_id = $1 AND is_main = true LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query main world")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<World>> {
        let rows = sqlx::query_as::<_, WorldRow>(
            "SELECT id, project_id, name, description, world_rules, config, is_main, created_at, updated_at \
             FROM world WHERE project_id = $1 ORDER BY is_main DESC, name",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query worlds")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn update(&self, world: &World) -> Result<()> {
        sqlx::query(
            "UPDATE world SET name = $1, description = $2, world_rules = $3, config = $4, is_main = $5, updated_at = $6 \
             WHERE id = $7",
        )
        .bind(&world.name)
        .bind(&world.description)
        .bind(&world.world_rules)
        .bind(&world.config)
        .bind(world.is_main)
        .bind(Utc::now())
        .bind(world.id)
        .execute(&self.pool)
        .await
        .context("Failed to update world")?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM world WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete world")?;
        Ok(())
    }

    /// 确保项目有至少一个世界
    pub async fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World> {
        if let Some(world) = self.get_main_world(project_id).await? {
            return Ok(world);
        }
        self.create(
            project_id,
            &format!("{} - Main World", project_name),
            None,
            None,
            true,
        )
        .await
    }
}

#[derive(sqlx::FromRow)]
struct WorldRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    world_rules: Option<String>,
    config: Option<serde_json::Value>,
    is_main: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<WorldRow> for World {
    fn from(r: WorldRow) -> Self {
        World {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            world_rules: r.world_rules,
            config: r.config.unwrap_or_default(),
            is_main: r.is_main,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
