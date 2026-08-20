//! Character Mind Repos - Belief, Memory, EmotionState CRUD

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ai::character_mind::*;
use sqlx::PgPool;
use uuid::Uuid;

// ============= BeliefRepo =============

pub struct BeliefRepo {
    pool: PgPool,
}

impl BeliefRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        character_id: Uuid,
        belief_content: &str,
        confidence: f64,
        source: Option<&str>,
    ) -> Result<Belief> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO belief (id, project_id, character_id, belief_content, confidence, source, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, true, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(character_id)
        .bind(belief_content)
        .bind(confidence)
        .bind(source.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert belief")?;

        Ok(Belief {
            id,
            project_id,
            character_id,
            belief_content: belief_content.to_string(),
            confidence,
            source: source.map(|s| s.to_string()),
            source_scene_id: None,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_by_character(&self, character_id: Uuid) -> Result<Vec<Belief>> {
        let rows = sqlx::query_as::<_, BeliefRow>(
            "SELECT id, project_id, character_id, belief_content, confidence, source, source_scene_id, is_active, created_at, updated_at \
             FROM belief WHERE character_id = $1 AND is_active = true",
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query beliefs")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct BeliefRow {
    id: Uuid,
    project_id: Uuid,
    character_id: Uuid,
    belief_content: String,
    confidence: f64,
    source: Option<String>,
    source_scene_id: Option<Uuid>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<BeliefRow> for Belief {
    fn from(r: BeliefRow) -> Self {
        Belief {
            id: r.id,
            project_id: r.project_id,
            character_id: r.character_id,
            belief_content: r.belief_content,
            confidence: r.confidence,
            source: r.source,
            source_scene_id: r.source_scene_id,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= MemoryRepo =============

pub struct MemoryRepo {
    pool: PgPool,
}

impl MemoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        character_id: Uuid,
        memory_content: &str,
        emotional_impact: Option<&str>,
        importance: i32,
    ) -> Result<Memory> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO character_memory (id, project_id, character_id, memory_content, emotional_impact, importance, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, true, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(character_id)
        .bind(memory_content)
        .bind(emotional_impact.unwrap_or(""))
        .bind(importance)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert memory")?;

        Ok(Memory {
            id,
            project_id,
            character_id,
            memory_content: memory_content.to_string(),
            emotional_impact: emotional_impact.map(|s| s.to_string()),
            scene_id: None,
            importance,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_by_character(&self, character_id: Uuid) -> Result<Vec<Memory>> {
        let rows = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, project_id, character_id, memory_content, emotional_impact, scene_id, importance, is_active, created_at, updated_at \
             FROM character_memory WHERE character_id = $1 AND is_active = true ORDER BY importance DESC",
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query memories")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct MemoryRow {
    id: Uuid,
    project_id: Uuid,
    character_id: Uuid,
    memory_content: String,
    emotional_impact: Option<String>,
    scene_id: Option<Uuid>,
    importance: i32,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<MemoryRow> for Memory {
    fn from(r: MemoryRow) -> Self {
        Memory {
            id: r.id,
            project_id: r.project_id,
            character_id: r.character_id,
            memory_content: r.memory_content,
            emotional_impact: r.emotional_impact,
            scene_id: r.scene_id,
            importance: r.importance,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ============= EmotionRepo =============

pub struct EmotionRepo {
    pool: PgPool,
}

impl EmotionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        character_id: Uuid,
        emotion_type: &str,
        intensity: i32,
        decay_rate: f64,
        trigger_description: Option<&str>,
    ) -> Result<EmotionState> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO emotion_state (id, project_id, character_id, emotion_type, intensity, decay_rate, trigger_description, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(project_id)
        .bind(character_id)
        .bind(emotion_type)
        .bind(intensity)
        .bind(decay_rate)
        .bind(trigger_description.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert emotion")?;

        Ok(EmotionState {
            id,
            project_id,
            character_id,
            emotion_type: emotion_type.to_string(),
            intensity,
            decay_rate,
            trigger_scene_id: None,
            trigger_description: trigger_description.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    /// 更新情绪强度（考虑衰减）
    pub async fn update_intensity(&self, id: Uuid, new_intensity: i32) -> Result<()> {
        sqlx::query("UPDATE emotion_state SET intensity = $1, updated_at = $2 WHERE id = $3")
            .bind(new_intensity)
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update emotion intensity")?;
        Ok(())
    }

    /// 应用衰减：每个 Scene 结束后调用
    pub async fn apply_decay(&self, project_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE emotion_state SET intensity = CASE \
               WHEN intensity - CAST(intensity * decay_rate AS INTEGER) < 0 THEN 0 \
               ELSE intensity - CAST(intensity * decay_rate AS INTEGER) \
             END, updated_at = $1 \
             WHERE project_id = $2 AND intensity > 0",
        )
        .bind(Utc::now())
        .bind(project_id)
        .execute(&self.pool)
        .await
        .context("Failed to apply emotion decay")?;
        Ok(())
    }

    pub async fn list_by_character(&self, character_id: Uuid) -> Result<Vec<EmotionState>> {
        let rows = sqlx::query_as::<_, EmotionStateRow>(
            "SELECT id, project_id, character_id, emotion_type, intensity, decay_rate, trigger_scene_id, trigger_description, created_at, updated_at \
             FROM emotion_state WHERE character_id = $1 AND intensity > 0 ORDER BY intensity DESC",
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query emotions")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct EmotionStateRow {
    id: Uuid,
    project_id: Uuid,
    character_id: Uuid,
    emotion_type: String,
    intensity: i32,
    decay_rate: f64,
    trigger_scene_id: Option<Uuid>,
    trigger_description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EmotionStateRow> for EmotionState {
    fn from(r: EmotionStateRow) -> Self {
        EmotionState {
            id: r.id,
            project_id: r.project_id,
            character_id: r.character_id,
            emotion_type: r.emotion_type,
            intensity: r.intensity,
            decay_rate: r.decay_rate,
            trigger_scene_id: r.trigger_scene_id,
            trigger_description: r.trigger_description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
