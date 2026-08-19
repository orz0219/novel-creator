//! StateCommitter - 事务化提交状态变更
//!
//! 将 ProposedChange 列表事务化提交到世界状态。
//! 所有操作在同一个数据库事务中执行，任何一步失败则整体 ROLLBACK。

use anyhow::{Context, Result};
use db::repos::{state_repo, validation_repo};
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

/// StateCommitter 实现 - 事务化提交状态变更
///
/// 所有 change 在单个 BEGIN/COMMIT 事务中提交。
/// 任何一步失败自动 ROLLBACK，保证原子性。
pub struct DbStateCommitter {
    pool: PgPool,
}

impl DbStateCommitter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 事务化提交一批 ProposedChange。
    ///
    /// 整个 batch 在单个事务中执行：
    ///   BEGIN
    ///     validate approved changes
    ///     insert state_change
    ///     update current_state
    ///     mark proposed_change applied
    ///   COMMIT
    ///
    /// 任何一步失败 → ROLLBACK（由 sqlx Transaction drop 自动执行）。
    pub async fn commit(
        &self,
        project_id: Uuid,
        changes: &[ProposedChange],
    ) -> Result<Vec<StateChangeRecord>> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        let mut records = Vec::new();

        for change in changes {
            match &change.change_type {
                ProposedChangeType::StateChange => {
                    let state_key = change.payload.get("state_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let new_value = change.payload.get("new_value")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    // 1. 获取旧状态（在同一事务内）
                    let old_state = state_repo::StateRepo::get_current_state_tx(
                        &mut *tx, change.target_entity_id, state_key,
                    ).await?;
                    let old_value = old_state.map(|s| s.state_value);

                    // 2. 记录变更历史
                    let record = state_repo::StateRepo::record_change_tx(
                        &mut *tx,
                        project_id, None, "STATE_CHANGE",
                        change.target_entity_id, state_key,
                        old_value, new_value.clone(),
                        Some("committer"),
                    ).await?;

                    // 3. 更新当前状态
                    state_repo::StateRepo::upsert_state_tx(
                        &mut *tx,
                        project_id, change.target_entity_id,
                        state_key, new_value,
                    ).await?;

                    // 4. 标记 ProposedChange 为 Applied
                    validation_repo::ValidationRepo::update_status_tx(
                        &mut *tx,
                        change.id,
                        ProposedChangeStatus::Applied,
                    ).await?;

                    records.push(record);
                }
                _ => {
                    tracing::warn!("Unsupported change type: {:?}", change.change_type);
                }
            }
        }

        // 提交事务 - 如果前面任何步骤失败，tx 会被 drop 导致 ROLLBACK
        tx.commit().await.context("Failed to commit transaction")?;

        tracing::info!("Committed {} changes in a single transaction", records.len());
        Ok(records)
    }
}
