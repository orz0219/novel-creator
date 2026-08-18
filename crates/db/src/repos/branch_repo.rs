//! Branch Repository - CRUD operations for WorldBranch/NarrativeBranch

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{BranchStatus, NarrativeBranch, WorldBranch};
use sqlx::PgPool;
use uuid::Uuid;

pub struct BranchRepo {
    pool: PgPool,
}

impl BranchRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建世界分支
    pub async fn create_world_branch(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        is_main: bool,
    ) -> Result<WorldBranch> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO world_branch (id, project_id, name, description, is_main, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, 'Active', $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(is_main)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create world branch")?;

        Ok(WorldBranch {
            id,
            project_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            parent_branch_id: None,
            is_main,
            status: BranchStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    /// 创建叙事分支
    pub async fn create_narrative_branch(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        is_main: bool,
    ) -> Result<NarrativeBranch> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO narrative_branch (id, project_id, name, description, is_main, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, 'Active', $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(name)
        .bind(description.unwrap_or(""))
        .bind(is_main)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create narrative branch")?;

        Ok(NarrativeBranch {
            id,
            project_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            parent_branch_id: None,
            fork_point_scene_id: None,
            is_main,
            status: BranchStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    /// 列出项目中的所有世界分支
    pub async fn list_world_branches(&self, project_id: Uuid) -> Result<Vec<WorldBranch>> {
        let rows = sqlx::query_as::<_, WorldBranchRow>(
            "SELECT id, project_id, name, description, parent_branch_id, is_main, status, created_at, updated_at \
             FROM world_branch WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query world branches")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct WorldBranchRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    parent_branch_id: Option<Uuid>,
    is_main: bool,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<WorldBranchRow> for WorldBranch {
    fn from(r: WorldBranchRow) -> Self {
        let status = match r.status.as_str() {
            "Active" => BranchStatus::Active,
            "Merged" => BranchStatus::Merged,
            "Abandoned" => BranchStatus::Abandoned,
            _ => BranchStatus::Active,
        };
        WorldBranch {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            parent_branch_id: r.parent_branch_id,
            is_main: r.is_main,
            status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
