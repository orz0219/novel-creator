//! ContextSnapshot Repository - 上下文快照持久化

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{ContextLayer, ContextPackage};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ContextSnapshotRepo {
    pool: PgPool,
}

impl ContextSnapshotRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 保存上下文快照
    pub async fn save(&self, package: &ContextPackage) -> Result<()> {
        sqlx::query(
            "INSERT INTO context_snapshot (id, project_id, scene_id, token_budget, \
             l0_essential, l1_scene_relevant, l2_recent_history, l3_narrative_context, \
             l4_character_knowledge, l5_world_background, l6_optional_supplement, \
             actual_tokens, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(package.id)
        .bind(package.project_id)
        .bind(package.scene_id)
        .bind(package.token_budget)
        .bind(serde_json::to_value(&package.l0_essential).unwrap_or_default())
        .bind(serde_json::to_value(&package.l1_scene_relevant).unwrap_or_default())
        .bind(serde_json::to_value(&package.l2_recent_history).unwrap_or_default())
        .bind(serde_json::to_value(&package.l3_narrative_context).unwrap_or_default())
        .bind(serde_json::to_value(&package.l4_character_knowledge).unwrap_or_default())
        .bind(serde_json::to_value(&package.l5_world_background).unwrap_or_default())
        .bind(serde_json::to_value(&package.l6_optional_supplement).unwrap_or_default())
        .bind(package.actual_tokens)
        .bind(package.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to save context snapshot")?;
        Ok(())
    }

    /// 按 ID 获取快照
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<ContextPackage>> {
        let row = sqlx::query_as::<_, ContextSnapshotRow>(
            "SELECT id, project_id, scene_id, token_budget, \
             l0_essential, l1_scene_relevant, l2_recent_history, l3_narrative_context, \
             l4_character_knowledge, l5_world_background, l6_optional_supplement, \
             actual_tokens, created_at \
             FROM context_snapshot WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query context snapshot")?;

        Ok(row.map(|r| r.into()))
    }

    /// 按场景获取快照列表
    pub async fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<ContextPackage>> {
        let rows = sqlx::query_as::<_, ContextSnapshotRow>(
            "SELECT id, project_id, scene_id, token_budget, \
             l0_essential, l1_scene_relevant, l2_recent_history, l3_narrative_context, \
             l4_character_knowledge, l5_world_background, l6_optional_supplement, \
             actual_tokens, created_at \
             FROM context_snapshot WHERE scene_id = $1 ORDER BY created_at DESC",
        )
        .bind(scene_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query context snapshots")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ContextSnapshotRow {
    id: Uuid,
    project_id: Uuid,
    scene_id: Uuid,
    token_budget: i32,
    l0_essential: Option<serde_json::Value>,
    l1_scene_relevant: Option<serde_json::Value>,
    l2_recent_history: Option<serde_json::Value>,
    l3_narrative_context: Option<serde_json::Value>,
    l4_character_knowledge: Option<serde_json::Value>,
    l5_world_background: Option<serde_json::Value>,
    l6_optional_supplement: Option<serde_json::Value>,
    actual_tokens: Option<i32>,
    created_at: DateTime<Utc>,
}

impl From<ContextSnapshotRow> for ContextPackage {
    fn from(r: ContextSnapshotRow) -> Self {
        let parse_layer = |v: Option<serde_json::Value>| -> ContextLayer {
            v.and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or(ContextLayer {
                    content: String::new(),
                    token_estimate: 0,
                    included: false,
                })
        };

        ContextPackage {
            id: r.id,
            project_id: r.project_id,
            scene_id: r.scene_id,
            token_budget: r.token_budget,
            l0_essential: parse_layer(r.l0_essential),
            l1_scene_relevant: parse_layer(r.l1_scene_relevant),
            l2_recent_history: parse_layer(r.l2_recent_history),
            l3_narrative_context: parse_layer(r.l3_narrative_context),
            l4_character_knowledge: parse_layer(r.l4_character_knowledge),
            l5_world_background: parse_layer(r.l5_world_background),
            l6_optional_supplement: parse_layer(r.l6_optional_supplement),
            actual_tokens: r.actual_tokens.unwrap_or(0),
            created_at: r.created_at,
        }
    }
}
