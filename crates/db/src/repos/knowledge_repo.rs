//! Knowledge Repository

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{
    KnowledgeLevel, KnowledgeState, KnowledgeSubjectType, Revelation, RevelationTarget,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ser;

pub struct KnowledgeRepo {
    pool: PgPool,
}

impl KnowledgeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_state(
        &self,
        project_id: Uuid,
        fact_id: Uuid,
        subject_type: KnowledgeSubjectType,
        subject_id: Option<Uuid>,
        knows: bool,
        knowledge_level: KnowledgeLevel,
        source: Option<&str>,
    ) -> Result<KnowledgeState> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let st_str = ser::knowledge_subject_type_str(&subject_type);
        let kl_str = ser::knowledge_level_str(&knowledge_level);

        sqlx::query(
            "INSERT INTO knowledge_state (id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source, effective_from, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(project_id)
        .bind(fact_id)
        .bind(&st_str)
        .bind(subject_id)
        .bind(knows)
        .bind(&kl_str)
        .bind(source.unwrap_or(""))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create knowledge state")?;

        Ok(KnowledgeState {
            id,
            project_id,
            fact_id,
            subject_type,
            subject_id,
            knows,
            knowledge_level,
            source: source.map(|s| s.to_string()),
            effective_from: now,
            effective_to: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_state(
        &self,
        fact_id: Uuid,
        subject_type: &KnowledgeSubjectType,
        subject_id: Option<Uuid>,
    ) -> Result<Option<KnowledgeState>> {
        let st_str = ser::knowledge_subject_type_str(subject_type);

        let row = sqlx::query_as::<_, KnowledgeStateRow>(
            "SELECT id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source, effective_from, effective_to, created_at, updated_at \
             FROM knowledge_state WHERE fact_id = $1 AND subject_type = $2 AND subject_id = $3 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(fact_id)
        .bind(&st_str)
        .bind(subject_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query knowledge state")?;

        Ok(row.map(|r| r.into()))
    }

    /// Get all knowledge states for a character
    pub async fn get_character_knowledge(
        &self,
        character_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<KnowledgeState>> {
        let rows = sqlx::query_as::<_, KnowledgeStateRow>(
            "SELECT id, project_id, fact_id, subject_type, subject_id, knows, knowledge_level, source, effective_from, effective_to, created_at, updated_at \
             FROM knowledge_state \
             WHERE project_id = $1 AND subject_type = 'Character' AND subject_id = $2 AND knows = TRUE \
             ORDER BY created_at DESC",
        )
        .bind(project_id)
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query character knowledge")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get all facts a character actually knows, joined with fact content.
    pub async fn get_character_known_facts(
        &self,
        character_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<domain::knowledge::CharacterKnowledgeItem>> {
        let rows = sqlx::query_as::<_, KnownFactRow>(
            "SELECT f.content, f.category, COALESCE(f.certainty, 'CANON'), ks.knowledge_level, ks.source \
             FROM knowledge_state ks \
             JOIN fact f ON f.id = ks.fact_id \
             WHERE ks.project_id = $1 \
               AND ks.subject_type = 'Character' \
               AND ks.subject_id = $2 \
               AND ks.knows = TRUE \
             ORDER BY ks.created_at DESC",
        )
        .bind(project_id)
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query known facts")?;

        Ok(rows
            .into_iter()
            .map(|r| domain::knowledge::CharacterKnowledgeItem {
                fact_content: r.content,
                fact_category: r.category,
                fact_certainty: r.certainty,
                knowledge_level: ser::parse_knowledge_level(&r.knowledge_level),
                source: r.source,
            })
            .collect())
    }

    pub async fn create_revelation(
        &self,
        project_id: Uuid,
        fact_id: Uuid,
        scene_id: Uuid,
        revealed_to: &[RevelationTarget],
        method: Option<&str>,
        significance: Option<&str>,
    ) -> Result<Revelation> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO revelation (id, project_id, fact_id, scene_id, revelation_method, narrative_significance, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(fact_id)
        .bind(scene_id)
        .bind(method.unwrap_or(""))
        .bind(significance.unwrap_or(""))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create revelation")?;

        for target in revealed_to {
            let st_str = ser::knowledge_subject_type_str(&target.subject_type);
            let kl_str = ser::knowledge_level_str(&target.knowledge_level);
            sqlx::query(
                "INSERT INTO revelation_target (id, revelation_id, subject_type, subject_id, knowledge_level) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(&st_str)
            .bind(target.subject_id)
            .bind(&kl_str)
            .execute(&self.pool)
            .await
            .context("Failed to create revelation target")?;
        }

        Ok(Revelation {
            id,
            project_id,
            fact_id,
            scene_id,
            revealed_to: revealed_to.to_vec(),
            revelation_method: method.map(|s| s.to_string()),
            narrative_significance: significance.map(|s| s.to_string()),
            created_at: now,
        })
    }
}

#[derive(sqlx::FromRow)]
struct KnowledgeStateRow {
    id: Uuid,
    project_id: Uuid,
    fact_id: Uuid,
    subject_type: String,
    subject_id: Option<Uuid>,
    knows: bool,
    knowledge_level: String,
    source: Option<String>,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<KnowledgeStateRow> for KnowledgeState {
    fn from(r: KnowledgeStateRow) -> Self {
        KnowledgeState {
            id: r.id,
            project_id: r.project_id,
            fact_id: r.fact_id,
            subject_type: ser::parse_knowledge_subject_type(&r.subject_type),
            subject_id: r.subject_id,
            knows: r.knows,
            knowledge_level: ser::parse_knowledge_level(&r.knowledge_level),
            source: r.source,
            effective_from: r.effective_from,
            effective_to: r.effective_to,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct KnownFactRow {
    content: String,
    category: Option<String>,
    certainty: String,
    knowledge_level: String,
    source: Option<String>,
}


