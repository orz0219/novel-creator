//! ⚠️ 本文件由 tmp/gen_sqlite_backend.py 自动生成，请勿手工编辑。
//! 如需修改逻辑，请改 PG 侧的对应文件后重新生成。

//! 阶段弧线（entity_arc_stage）仓库 —— 人物 / 势力 / 地点共用。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::*;
use sqlx::SqlitePool;
use uuid::Uuid;

/// 实体阶段弧线（外部时间线：何时上场、演什么、戏份多大、当时什么状态）。
///
/// 与各实体的 profile（静态身份）正交，单独一张表，整组替换语义
/// （与 conflicts / secrets 一致：模型读到旧列表、改完再整份写回）。
pub struct EntityArcStageRepo {
    pool: SqlitePool,
}

impl EntityArcStageRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_by_entity(&self, entity_id: Uuid) -> Result<Vec<EntityArcStage>> {
        let rows = sqlx::query_as::<_, EntityArcStageRow>(
            "SELECT id, entity_id, stage, order_index, role, screen_weight, goal, \"function\", entry_trigger, status, created_at, updated_at \
             FROM entity_arc_stage WHERE entity_id = $1 ORDER BY order_index, created_at",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query entity_arc_stage")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn replace_by_entity(
        &self,
        entity_id: Uuid,
        stages: &[EntityArcStage],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin arc stage tx")?;
        sqlx::query("DELETE FROM entity_arc_stage WHERE entity_id = $1")
            .bind(entity_id)
            .execute(&mut *tx)
            .await
            .context("clear entity_arc_stage")?;

        let now = Utc::now();
        for s in stages {
            sqlx::query(
                "INSERT INTO entity_arc_stage (id, entity_id, stage, order_index, role, screen_weight, goal, \"function\", entry_trigger, status, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .bind(s.id)
            .bind(entity_id)
            .bind(&s.stage)
            .bind(s.order)
            .bind(&s.role)
            .bind(s.screen_weight.map(|w| w.as_str()))
            .bind(&s.goal)
            .bind(&s.function)
            .bind(&s.entry_trigger)
            .bind(&s.status)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("insert entity_arc_stage")?;
        }
        tx.commit().await.context("commit arc stage tx")?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)] // 字段保留以便后续扩展 / SQL 完整性
struct EntityArcStageRow {
    id: Uuid,
    entity_id: Uuid,
    stage: String,
    order_index: i32,
    role: Option<String>,
    screen_weight: Option<String>,
    goal: Option<String>,
    function: Option<String>,
    entry_trigger: Option<String>,
    status: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<EntityArcStageRow> for EntityArcStage {
    fn from(r: EntityArcStageRow) -> Self {
        let filter_empty = |s: Option<String>| s.filter(|s| !s.trim().is_empty());
        EntityArcStage {
            id: r.id,
            entity_id: r.entity_id,
            stage: r.stage,
            order: r.order_index,
            role: filter_empty(r.role),
            screen_weight: r.screen_weight.as_deref().and_then(ScreenWeight::parse),
            goal: filter_empty(r.goal),
            function: filter_empty(r.function),
            entry_trigger: filter_empty(r.entry_trigger),
            status: filter_empty(r.status),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
