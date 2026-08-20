//! Summary Repos - Chapter/Arc/Volume Summary + Global Story State

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ai::state_mgmt::*;
use sqlx::PgPool;
use uuid::Uuid;

// ============= ChapterSummaryRepo =============

pub struct ChapterSummaryRepo {
    pool: PgPool,
}

impl ChapterSummaryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        chapter_id: Uuid,
        summary: &str,
        key_events: Vec<String>,
        involved_characters: Vec<Uuid>,
    ) -> Result<ChapterSummary> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO chapter_summary (id, project_id, chapter_id, summary, key_events, involved_characters, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(chapter_id)
        .bind(summary)
        .bind(serde_json::to_value(&key_events).unwrap_or_default())
        .bind(serde_json::to_value(&involved_characters).unwrap_or_default())
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert chapter_summary")?;

        Ok(ChapterSummary {
            id,
            project_id,
            chapter_id,
            summary: summary.to_string(),
            key_events,
            involved_characters,
            created_at: now,
        })
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<ChapterSummary>> {
        let rows = sqlx::query_as::<_, ChapterSummaryRow>(
            "SELECT id, project_id, chapter_id, summary, key_events, involved_characters, created_at \
             FROM chapter_summary WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query chapter summaries")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ChapterSummaryRow {
    id: Uuid,
    project_id: Uuid,
    chapter_id: Uuid,
    summary: String,
    key_events: Option<serde_json::Value>,
    involved_characters: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<ChapterSummaryRow> for ChapterSummary {
    fn from(r: ChapterSummaryRow) -> Self {
        ChapterSummary {
            id: r.id,
            project_id: r.project_id,
            chapter_id: r.chapter_id,
            summary: r.summary,
            key_events: r.key_events.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            involved_characters: r.involved_characters.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            created_at: r.created_at,
        }
    }
}

// ============= ArcSummaryRepo =============

pub struct ArcSummaryRepo {
    pool: PgPool,
}

impl ArcSummaryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        arc_id: Uuid,
        summary: &str,
        key_turning_points: Vec<String>,
        status: &str,
    ) -> Result<ArcSummary> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO arc_summary (id, project_id, arc_id, summary, key_turning_points, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(arc_id)
        .bind(summary)
        .bind(serde_json::to_value(&key_turning_points).unwrap_or_default())
        .bind(status)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert arc_summary")?;

        Ok(ArcSummary {
            id,
            project_id,
            arc_id,
            summary: summary.to_string(),
            key_turning_points,
            status: status.to_string(),
            created_at: now,
        })
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<ArcSummary>> {
        let rows = sqlx::query_as::<_, ArcSummaryRow>(
            "SELECT id, project_id, arc_id, summary, key_turning_points, status, created_at \
             FROM arc_summary WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query arc summaries")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ArcSummaryRow {
    id: Uuid,
    project_id: Uuid,
    arc_id: Uuid,
    summary: String,
    key_turning_points: Option<serde_json::Value>,
    status: String,
    created_at: DateTime<Utc>,
}

impl From<ArcSummaryRow> for ArcSummary {
    fn from(r: ArcSummaryRow) -> Self {
        ArcSummary {
            id: r.id,
            project_id: r.project_id,
            arc_id: r.arc_id,
            summary: r.summary,
            key_turning_points: r.key_turning_points.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            status: r.status,
            created_at: r.created_at,
        }
    }
}

// ============= VolumeSummaryRepo =============

pub struct VolumeSummaryRepo {
    pool: PgPool,
}

impl VolumeSummaryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        volume_id: Uuid,
        summary: &str,
        character_changes: Vec<String>,
        world_changes: Vec<String>,
        foreshadowing_progress: Vec<String>,
    ) -> Result<VolumeSummary> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO volume_summary (id, project_id, volume_id, summary, character_changes, world_changes, foreshadowing_progress, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(volume_id)
        .bind(summary)
        .bind(serde_json::to_value(&character_changes).unwrap_or_default())
        .bind(serde_json::to_value(&world_changes).unwrap_or_default())
        .bind(serde_json::to_value(&foreshadowing_progress).unwrap_or_default())
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert volume_summary")?;

        Ok(VolumeSummary {
            id,
            project_id,
            volume_id,
            summary: summary.to_string(),
            character_changes,
            world_changes,
            foreshadowing_progress,
            created_at: now,
        })
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<VolumeSummary>> {
        let rows = sqlx::query_as::<_, VolumeSummaryRow>(
            "SELECT id, project_id, volume_id, summary, character_changes, world_changes, foreshadowing_progress, created_at \
             FROM volume_summary WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query volume summaries")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct VolumeSummaryRow {
    id: Uuid,
    project_id: Uuid,
    volume_id: Uuid,
    summary: String,
    character_changes: Option<serde_json::Value>,
    world_changes: Option<serde_json::Value>,
    foreshadowing_progress: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<VolumeSummaryRow> for VolumeSummary {
    fn from(r: VolumeSummaryRow) -> Self {
        VolumeSummary {
            id: r.id,
            project_id: r.project_id,
            volume_id: r.volume_id,
            summary: r.summary,
            character_changes: r.character_changes.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            world_changes: r.world_changes.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            foreshadowing_progress: r.foreshadowing_progress.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            created_at: r.created_at,
        }
    }
}

// ============= GlobalStoryStateRepo =============

pub struct GlobalStoryStateRepo {
    pool: PgPool,
}

impl GlobalStoryStateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(
        &self,
        project_id: Uuid,
        current_progress: &str,
        open_foreshadowing: Vec<String>,
        open_storylines: Vec<String>,
        world_state_summary: &str,
        character_state_summary: &str,
    ) -> Result<GlobalStoryState> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Delete existing
        sqlx::query("DELETE FROM global_story_state WHERE project_id = $1")
            .bind(project_id)
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query(
            "INSERT INTO global_story_state (id, project_id, current_progress, open_foreshadowing, open_storylines, world_state_summary, character_state_summary, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(current_progress)
        .bind(serde_json::to_value(&open_foreshadowing).unwrap_or_default())
        .bind(serde_json::to_value(&open_storylines).unwrap_or_default())
        .bind(world_state_summary)
        .bind(character_state_summary)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert global_story_state")?;

        Ok(GlobalStoryState {
            id,
            project_id,
            current_progress: current_progress.to_string(),
            open_foreshadowing,
            open_storylines,
            world_state_summary: world_state_summary.to_string(),
            character_state_summary: character_state_summary.to_string(),
            updated_at: now,
        })
    }

    pub async fn get(&self, project_id: Uuid) -> Result<Option<GlobalStoryState>> {
        let row = sqlx::query_as::<_, GlobalStoryStateRow>(
            "SELECT id, project_id, current_progress, open_foreshadowing, open_storylines, world_state_summary, character_state_summary, updated_at \
             FROM global_story_state WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query global story state")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct GlobalStoryStateRow {
    id: Uuid,
    project_id: Uuid,
    current_progress: String,
    open_foreshadowing: Option<serde_json::Value>,
    open_storylines: Option<serde_json::Value>,
    world_state_summary: String,
    character_state_summary: String,
    updated_at: DateTime<Utc>,
}

impl From<GlobalStoryStateRow> for GlobalStoryState {
    fn from(r: GlobalStoryStateRow) -> Self {
        GlobalStoryState {
            id: r.id,
            project_id: r.project_id,
            current_progress: r.current_progress,
            open_foreshadowing: r.open_foreshadowing.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            open_storylines: r.open_storylines.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            world_state_summary: r.world_state_summary,
            character_state_summary: r.character_state_summary,
            updated_at: r.updated_at,
        }
    }
}
