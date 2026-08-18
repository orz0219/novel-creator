//! ReaderKnowledge Repository - CRUD operations for ReaderKnowledge

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{ReaderConfidence, ReaderKnowledge, ReaderKnowledgeLevel};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ReaderKnowledgeRepo {
    pool: PgPool,
}

impl ReaderKnowledgeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建读者知识
    pub async fn create(
        &self,
        project_id: Uuid,
        fact_id: Uuid,
        knowledge_level: ReaderKnowledgeLevel,
        confidence: ReaderConfidence,
    ) -> Result<ReaderKnowledge> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let kl_str = match knowledge_level {
            ReaderKnowledgeLevel::Unknown => "Unknown",
            ReaderKnowledgeLevel::Hearsay => "Hearsay",
            ReaderKnowledgeLevel::Suspected => "Suspected",
            ReaderKnowledgeLevel::Partial => "Partial",
            ReaderKnowledgeLevel::Complete => "Complete",
            ReaderKnowledgeLevel::Misunderstood => "Misunderstood",
        };
        let c_str = match confidence {
            ReaderConfidence::Certain => "Certain",
            ReaderConfidence::Likely => "Likely",
            ReaderConfidence::Uncertain => "Uncertain",
            ReaderConfidence::Speculative => "Speculative",
        };

        sqlx::query(
            "INSERT INTO reader_knowledge (id, project_id, fact_id, knowledge_level, confidence, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(fact_id)
        .bind(kl_str)
        .bind(c_str)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create reader knowledge")?;

        Ok(ReaderKnowledge {
            id,
            project_id,
            fact_id,
            knowledge_level,
            source_scene_id: None,
            confidence,
            created_at: now,
            updated_at: now,
        })
    }

    /// 列出项目中的所有读者知识
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<ReaderKnowledge>> {
        let rows = sqlx::query_as::<_, ReaderKnowledgeRow>(
            "SELECT id, project_id, fact_id, knowledge_level, source_scene_id, confidence, created_at, updated_at \
             FROM reader_knowledge WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query reader knowledge")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ReaderKnowledgeRow {
    id: Uuid,
    project_id: Uuid,
    fact_id: Uuid,
    knowledge_level: String,
    source_scene_id: Option<Uuid>,
    confidence: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ReaderKnowledgeRow> for ReaderKnowledge {
    fn from(r: ReaderKnowledgeRow) -> Self {
        let knowledge_level = match r.knowledge_level.as_str() {
            "Unknown" => ReaderKnowledgeLevel::Unknown,
            "Hearsay" => ReaderKnowledgeLevel::Hearsay,
            "Suspected" => ReaderKnowledgeLevel::Suspected,
            "Partial" => ReaderKnowledgeLevel::Partial,
            "Complete" => ReaderKnowledgeLevel::Complete,
            "Misunderstood" => ReaderKnowledgeLevel::Misunderstood,
            _ => ReaderKnowledgeLevel::Unknown,
        };
        let confidence = match r.confidence.as_str() {
            "Certain" => ReaderConfidence::Certain,
            "Likely" => ReaderConfidence::Likely,
            "Uncertain" => ReaderConfidence::Uncertain,
            "Speculative" => ReaderConfidence::Speculative,
            _ => ReaderConfidence::Certain,
        };
        ReaderKnowledge {
            id: r.id,
            project_id: r.project_id,
            fact_id: r.fact_id,
            knowledge_level,
            source_scene_id: r.source_scene_id,
            confidence,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
