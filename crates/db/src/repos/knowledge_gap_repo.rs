//! KnowledgeGap Repository - 知识缺口 CRUD

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ai::state_mgmt::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct KnowledgeGapRepo {
    pool: PgPool,
}

impl KnowledgeGapRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        gap_type: &str,
        description: &str,
        importance: &str,
        required_by_scene_id: Option<Uuid>,
        designer_skill_hint: Option<&str>,
    ) -> Result<KnowledgeGap> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO knowledge_gap (id, project_id, gap_type, description, importance, required_by_scene_id, status, designer_skill_hint, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'Open', $7, $8, $9)",
        )
        .bind(id)
        .bind(project_id)
        .bind(gap_type)
        .bind(description)
        .bind(importance)
        .bind(required_by_scene_id)
        .bind(designer_skill_hint.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert knowledge_gap")?;

        Ok(KnowledgeGap {
            id,
            project_id,
            gap_type: gap_type.to_string(),
            description: description.to_string(),
            importance: importance.to_string(),
            required_by_scene_id,
            status: GapStatus::Open,
            designer_skill_hint: designer_skill_hint.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn update_status(&self, id: Uuid, status: GapStatus) -> Result<()> {
        sqlx::query("UPDATE knowledge_gap SET status = $1, updated_at = $2 WHERE id = $3")
            .bind(status.as_str())
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update knowledge_gap status")?;
        Ok(())
    }

    pub async fn list_open_by_project(&self, project_id: Uuid) -> Result<Vec<KnowledgeGap>> {
        let rows = sqlx::query_as::<_, KnowledgeGapRow>(
            "SELECT id, project_id, gap_type, description, importance, required_by_scene_id, status, designer_skill_hint, created_at, updated_at \
             FROM knowledge_gap WHERE project_id = $1 AND status = 'Open' ORDER BY importance DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query knowledge gaps")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct KnowledgeGapRow {
    id: Uuid,
    project_id: Uuid,
    gap_type: String,
    description: String,
    importance: String,
    required_by_scene_id: Option<Uuid>,
    status: String,
    designer_skill_hint: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<KnowledgeGapRow> for KnowledgeGap {
    fn from(r: KnowledgeGapRow) -> Self {
        KnowledgeGap {
            id: r.id,
            project_id: r.project_id,
            gap_type: r.gap_type,
            description: r.description,
            importance: r.importance,
            required_by_scene_id: r.required_by_scene_id,
            status: GapStatus::from_str(&r.status),
            designer_skill_hint: r.designer_skill_hint,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
