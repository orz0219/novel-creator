//! WorldVersion Repository - 世界版本检查点（ChatGPT 评审 P2）。
//!
//! 类比 git commit：每次 Canon 前进一个不可变版本，支撑回滚 / diff / 多 Agent 协同。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{WorldVersion, WorldVersionKind};
use sqlx::PgPool;
use uuid::Uuid;

pub struct WorldVersionRepo {
    pool: PgPool,
}

impl WorldVersionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, v: &WorldVersion) -> Result<()> {
        sqlx::query(
            "INSERT INTO world_version (id, world_id, version, kind, trigger_id, summary, parent_version_id, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(v.id)
        .bind(v.world_id)
        .bind(v.version)
        .bind(v.kind.as_str())
        .bind(v.trigger_id)
        .bind(v.summary.clone())
        .bind(v.parent_version_id)
        .bind(v.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to create world version")?;
        Ok(())
    }

    /// 取得某世界当前最新版本号（None 表示尚无版本）
    pub async fn latest_version(&self, world_id: Uuid) -> Result<Option<i32>> {
        let v: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(version) FROM world_version WHERE world_id = $1",
        )
        .bind(world_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query latest world version")?;
        Ok(v)
    }

    /// 列出某世界的版本链（新 -> 旧）
    pub async fn list_by_world(&self, world_id: Uuid) -> Result<Vec<WorldVersion>> {
        let rows = sqlx::query_as::<_, WorldVersionRow>(
            "SELECT id, world_id, version, kind, trigger_id, summary, parent_version_id, created_at \
             FROM world_version WHERE world_id = $1 ORDER BY version DESC",
        )
        .bind(world_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query world versions")?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct WorldVersionRow {
    id: Uuid,
    world_id: Uuid,
    version: i32,
    kind: String,
    trigger_id: Option<Uuid>,
    summary: Option<String>,
    parent_version_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl From<WorldVersionRow> for WorldVersion {
    fn from(r: WorldVersionRow) -> Self {
        let kind = match r.kind.as_str() {
            "user_edit" => WorldVersionKind::UserEdit,
            "ai_proposal" => WorldVersionKind::AiProposal,
            "system" => WorldVersionKind::System,
            _ => WorldVersionKind::Baseline,
        };
        WorldVersion {
            id: r.id,
            world_id: r.world_id,
            version: r.version,
            kind,
            trigger_id: r.trigger_id,
            summary: r.summary,
            parent_version_id: r.parent_version_id,
            created_at: r.created_at,
        }
    }
}
