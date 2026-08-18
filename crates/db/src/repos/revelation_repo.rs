//! Revelation Repository - CRUD operations for Revelation

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{Revelation, RevelationTarget};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ser;

pub struct RevelationRepo {
    pool: PgPool,
}

impl RevelationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建揭示
    pub async fn create(
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

    /// 按场景获取揭示列表
    pub async fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<Revelation>> {
        let rows = sqlx::query_as::<_, RevelationRow>(
            "SELECT id, project_id, fact_id, scene_id, revelation_method, narrative_significance, created_at \
             FROM revelation WHERE scene_id = $1 ORDER BY created_at",
        )
        .bind(scene_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query revelations")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 按事实获取揭示列表
    pub async fn list_by_fact(&self, fact_id: Uuid) -> Result<Vec<Revelation>> {
        let rows = sqlx::query_as::<_, RevelationRow>(
            "SELECT id, project_id, fact_id, scene_id, revelation_method, narrative_significance, created_at \
             FROM revelation WHERE fact_id = $1 ORDER BY created_at",
        )
        .bind(fact_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query revelations")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct RevelationRow {
    id: Uuid,
    project_id: Uuid,
    fact_id: Uuid,
    scene_id: Uuid,
    revelation_method: Option<String>,
    narrative_significance: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<RevelationRow> for Revelation {
    fn from(r: RevelationRow) -> Self {
        Revelation {
            id: r.id,
            project_id: r.project_id,
            fact_id: r.fact_id,
            scene_id: r.scene_id,
            revealed_to: Vec::new(),
            revelation_method: r.revelation_method,
            narrative_significance: r.narrative_significance,
            created_at: r.created_at,
        }
    }
}
