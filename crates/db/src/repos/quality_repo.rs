//! QualityScore Repository - CRUD operations for QualityScore

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{QualityIssue, QualityScore};
use sqlx::PgPool;
use uuid::Uuid;

pub struct QualityScoreRepo {
    pool: PgPool,
}

impl QualityScoreRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        scene_id: Uuid,
        continuity: Option<i32>,
        character: Option<i32>,
        plot: Option<i32>,
        knowledge: Option<i32>,
        world: Option<i32>,
        style: Option<i32>,
        issues: Vec<QualityIssue>,
    ) -> Result<QualityScore> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let overall = match (continuity, character, plot, knowledge, world, style) {
            (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) => Some((a + b + c + d + e + f) / 6),
            _ => None,
        };

        sqlx::query(
            "INSERT INTO quality_score (id, project_id, scene_id, continuity_score, character_score, plot_score, knowledge_score, world_score, style_score, overall_score, issues, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(project_id)
        .bind(scene_id)
        .bind(continuity)
        .bind(character)
        .bind(plot)
        .bind(knowledge)
        .bind(world)
        .bind(style)
        .bind(overall)
        .bind(serde_json::to_value(&issues).unwrap_or_default())
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create quality score")?;

        Ok(QualityScore {
            id,
            project_id,
            scene_id,
            run_id: None,
            continuity_score: continuity,
            character_score: character,
            plot_score: plot,
            knowledge_score: knowledge,
            world_score: world,
            style_score: style,
            overall_score: overall,
            issues,
            created_at: now,
        })
    }

    pub async fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<QualityScore>> {
        let rows = sqlx::query_as::<_, QualityScoreRow>(
            "SELECT id, project_id, scene_id, continuity_score, character_score, plot_score, knowledge_score, world_score, style_score, overall_score, issues, created_at \
             FROM quality_score WHERE scene_id = $1 ORDER BY created_at DESC",
        )
        .bind(scene_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query quality scores")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct QualityScoreRow {
    id: Uuid,
    project_id: Uuid,
    scene_id: Uuid,
    continuity_score: Option<i32>,
    character_score: Option<i32>,
    plot_score: Option<i32>,
    knowledge_score: Option<i32>,
    world_score: Option<i32>,
    style_score: Option<i32>,
    overall_score: Option<i32>,
    issues: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<QualityScoreRow> for QualityScore {
    fn from(r: QualityScoreRow) -> Self {
        QualityScore {
            id: r.id,
            project_id: r.project_id,
            scene_id: r.scene_id,
            run_id: None,
            continuity_score: r.continuity_score,
            character_score: r.character_score,
            plot_score: r.plot_score,
            knowledge_score: r.knowledge_score,
            world_score: r.world_score,
            style_score: r.style_score,
            overall_score: r.overall_score,
            issues: r.issues.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            created_at: r.created_at,
        }
    }
}
