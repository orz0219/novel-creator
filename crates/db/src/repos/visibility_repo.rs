//! Visibility Repository - CRUD operations for FactVisibility

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{FactVisibility, VisibilityLevel, VisibilitySubjectType};
use sqlx::PgPool;
use uuid::Uuid;

pub struct VisibilityRepo {
    pool: PgPool,
}

impl VisibilityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建事实可见性
    pub async fn create(
        &self,
        project_id: Uuid,
        fact_id: Uuid,
        subject_type: VisibilitySubjectType,
        subject_id: Option<Uuid>,
        visibility_level: VisibilityLevel,
    ) -> Result<FactVisibility> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let st_str = match subject_type {
            VisibilitySubjectType::Author => "Author",
            VisibilitySubjectType::NarrativePlanner => "NarrativePlanner",
            VisibilitySubjectType::SceneWriter => "SceneWriter",
            VisibilitySubjectType::Character => "Character",
            VisibilitySubjectType::Reader => "Reader",
        };
        let vl_str = match visibility_level {
            VisibilityLevel::Visible => "Visible",
            VisibilityLevel::ExistsOnly => "ExistsOnly",
            VisibilityLevel::Hidden => "Hidden",
        };

        sqlx::query(
            "INSERT INTO fact_visibility (id, project_id, fact_id, subject_type, subject_id, visibility_level, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(fact_id)
        .bind(st_str)
        .bind(subject_id)
        .bind(vl_str)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create fact visibility")?;

        Ok(FactVisibility {
            id,
            project_id,
            fact_id,
            subject_type,
            subject_id,
            visibility_level,
            created_at: now,
            updated_at: now,
        })
    }

    /// 检查事实对主体的可见性
    pub async fn check_visibility(
        &self,
        fact_id: Uuid,
        subject_type: VisibilitySubjectType,
        subject_id: Option<Uuid>,
    ) -> Result<VisibilityLevel> {
        let st_str = match subject_type {
            VisibilitySubjectType::Author => "Author",
            VisibilitySubjectType::NarrativePlanner => "NarrativePlanner",
            VisibilitySubjectType::SceneWriter => "SceneWriter",
            VisibilitySubjectType::Character => "Character",
            VisibilitySubjectType::Reader => "Reader",
        };

        let row = if let Some(sid) = subject_id {
            sqlx::query_scalar::<_, String>(
                "SELECT visibility_level FROM fact_visibility \
                 WHERE fact_id = $1 AND subject_type = $2 AND subject_id = $3 \
                 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(fact_id)
            .bind(st_str)
            .bind(sid)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to check visibility")?
        } else {
            sqlx::query_scalar::<_, String>(
                "SELECT visibility_level FROM fact_visibility \
                 WHERE fact_id = $1 AND subject_type = $2 AND subject_id IS NULL \
                 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(fact_id)
            .bind(st_str)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to check visibility")?
        };

        match row.as_deref() {
            Some("Visible") => Ok(VisibilityLevel::Visible),
            Some("ExistsOnly") => Ok(VisibilityLevel::ExistsOnly),
            Some("Hidden") => Ok(VisibilityLevel::Hidden),
            _ => Ok(VisibilityLevel::Hidden), // 默认隐藏
        }
    }
}

#[derive(sqlx::FromRow)]
struct FactVisibilityRow {
    id: Uuid,
    project_id: Uuid,
    fact_id: Uuid,
    subject_type: String,
    subject_id: Option<Uuid>,
    visibility_level: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<FactVisibilityRow> for FactVisibility {
    fn from(r: FactVisibilityRow) -> Self {
        let subject_type = match r.subject_type.as_str() {
            "Author" => VisibilitySubjectType::Author,
            "NarrativePlanner" => VisibilitySubjectType::NarrativePlanner,
            "SceneWriter" => VisibilitySubjectType::SceneWriter,
            "Character" => VisibilitySubjectType::Character,
            "Reader" => VisibilitySubjectType::Reader,
            _ => VisibilitySubjectType::Character,
        };
        let visibility_level = match r.visibility_level.as_str() {
            "Visible" => VisibilityLevel::Visible,
            "ExistsOnly" => VisibilityLevel::ExistsOnly,
            "Hidden" => VisibilityLevel::Hidden,
            _ => VisibilityLevel::Hidden,
        };

        FactVisibility {
            id: r.id,
            project_id: r.project_id,
            fact_id: r.fact_id,
            subject_type,
            subject_id: r.subject_id,
            visibility_level,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
