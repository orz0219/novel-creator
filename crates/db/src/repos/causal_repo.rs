//! CausalRelation Repository - CRUD operations for CausalRelation

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{CausalRelation, CausalRelationType, CausalStrength};
use sqlx::PgPool;
use uuid::Uuid;

pub struct CausalRepo {
    pool: PgPool,
}

impl CausalRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建因果关系
    pub async fn create(
        &self,
        project_id: Uuid,
        cause_event_id: Uuid,
        effect_event_id: Uuid,
        relation_type: CausalRelationType,
        strength: CausalStrength,
        description: Option<&str>,
    ) -> Result<CausalRelation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let rt_str = match relation_type {
            CausalRelationType::DirectCause => "DirectCause",
            CausalRelationType::IndirectCause => "IndirectCause",
            CausalRelationType::Trigger => "Trigger",
            CausalRelationType::Prerequisite => "Prerequisite",
            CausalRelationType::ContributingFactor => "ContributingFactor",
        };
        let s_str = match strength {
            CausalStrength::Strong => "Strong",
            CausalStrength::Moderate => "Moderate",
            CausalStrength::Weak => "Weak",
        };

        sqlx::query(
            "INSERT INTO causal_relation (id, project_id, cause_event_id, effect_event_id, relation_type, strength, description, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(project_id)
        .bind(cause_event_id)
        .bind(effect_event_id)
        .bind(rt_str)
        .bind(s_str)
        .bind(description.unwrap_or(""))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create causal relation")?;

        Ok(CausalRelation {
            id,
            project_id,
            cause_event_id,
            effect_event_id,
            relation_type,
            strength,
            description: description.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    /// 列出项目中的所有因果关系
    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<CausalRelation>> {
        let rows = sqlx::query_as::<_, CausalRelationRow>(
            "SELECT id, project_id, cause_event_id, effect_event_id, relation_type, strength, description, created_at, updated_at \
             FROM causal_relation WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query causal relations")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct CausalRelationRow {
    id: Uuid,
    project_id: Uuid,
    cause_event_id: Uuid,
    effect_event_id: Uuid,
    relation_type: String,
    strength: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CausalRelationRow> for CausalRelation {
    fn from(r: CausalRelationRow) -> Self {
        let relation_type = match r.relation_type.as_str() {
            "DirectCause" => CausalRelationType::DirectCause,
            "IndirectCause" => CausalRelationType::IndirectCause,
            "Trigger" => CausalRelationType::Trigger,
            "Prerequisite" => CausalRelationType::Prerequisite,
            "ContributingFactor" => CausalRelationType::ContributingFactor,
            _ => CausalRelationType::DirectCause,
        };
        let strength = match r.strength.as_str() {
            "Strong" => CausalStrength::Strong,
            "Moderate" => CausalStrength::Moderate,
            "Weak" => CausalStrength::Weak,
            _ => CausalStrength::Moderate,
        };
        CausalRelation {
            id: r.id,
            project_id: r.project_id,
            cause_event_id: r.cause_event_id,
            effect_event_id: r.effect_event_id,
            relation_type,
            strength,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
