//! Storyline Repository - CRUD operations for Storyline

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Storyline, StorylineImportance, StorylineScene, StorylineStatus};
use sqlx::PgPool;
use uuid::Uuid;

pub struct StorylineRepo {
    pool: PgPool,
}

impl StorylineRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建剧情线
    pub async fn create(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: StorylineImportance,
    ) -> Result<Storyline> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let imp_str = match importance {
            StorylineImportance::Main => "Main",
            StorylineImportance::Important => "Important",
            StorylineImportance::Normal => "Normal",
            StorylineImportance::Minor => "Minor",
        };

        sqlx::query(
            "INSERT INTO storyline (id, project_id, name, description, status, importance, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'Active', $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(imp_str)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create storyline")?;

        Ok(Storyline {
            id,
            project_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            status: StorylineStatus::Active,
            importance,
            created_volume_id: None,
            resolved_volume_id: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// 按 ID 获取剧情线
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Storyline>> {
        let row = sqlx::query_as::<_, StorylineRow>(
            "SELECT id, project_id, name, description, status, importance, created_volume_id, resolved_volume_id, created_at, updated_at \
             FROM storyline WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query storyline")?;

        Ok(row.map(|r| r.into()))
    }

    /// 列出项目中的所有剧情线
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Storyline>> {
        let rows = sqlx::query_as::<_, StorylineRow>(
            "SELECT id, project_id, name, description, status, importance, created_volume_id, resolved_volume_id, created_at, updated_at \
             FROM storyline WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query storylines")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 关联剧情线和场景
    pub async fn link_scene(
        &self,
        storyline_id: Uuid,
        scene_id: Uuid,
        significance: Option<&str>,
    ) -> Result<StorylineScene> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO storyline_scene (id, storyline_id, scene_id, significance, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(storyline_id)
        .bind(scene_id)
        .bind(significance.unwrap_or(""))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to link storyline scene")?;

        Ok(StorylineScene {
            id,
            storyline_id,
            scene_id,
            significance: significance.map(|s| s.to_string()),
            created_at: now,
        })
    }
}

#[derive(sqlx::FromRow)]
struct StorylineRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    status: String,
    importance: String,
    created_volume_id: Option<Uuid>,
    resolved_volume_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<StorylineRow> for Storyline {
    fn from(r: StorylineRow) -> Self {
        let status = match r.status.as_str() {
            "Planned" => StorylineStatus::Planned,
            "Active" => StorylineStatus::Active,
            "Resolved" => StorylineStatus::Resolved,
            "Abandoned" => StorylineStatus::Abandoned,
            _ => StorylineStatus::Active,
        };
        let importance = match r.importance.as_str() {
            "Main" => StorylineImportance::Main,
            "Important" => StorylineImportance::Important,
            "Normal" => StorylineImportance::Normal,
            "Minor" => StorylineImportance::Minor,
            _ => StorylineImportance::Normal,
        };

        Storyline {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            status,
            importance,
            created_volume_id: r.created_volume_id,
            resolved_volume_id: r.resolved_volume_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
