//! StateCommitter - 事务化提交状态变更
//!
//! 将 ProposedChange 列表事务化提交到世界状态。
//! 所有操作在同一个数据库事务中执行，任何一步失败则整体 ROLLBACK。
//!
//! 核心不变量：
//! - 只有 Approved 的 ProposedChange 才能进入 StateCommitter
//! - 所有 State 查询必须具备 Project Isolation
//! - 使用 version CAS 防止并发覆盖

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
    /// 核心不变量：
    ///   - 只接受 status == Approved 的 change
    ///   - 使用 project_id + entity_id + state_key 做 project isolation
    ///   - 使用 version CAS 防止并发覆盖
    ///
    /// 任何一步失败 → ROLLBACK。
    pub async fn commit(
        &self,
        project_id: Uuid,
        changes: &[ProposedChange],
    ) -> Result<Vec<StateChangeRecord>> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        let mut records = Vec::new();

        for change in changes {
            // ============================================================
            // 不变量 1: 只有 Approved 的 ProposedChange 才能进入
            // ============================================================
            if change.status != ProposedChangeStatus::Approved {
                return Err(anyhow::anyhow!(
                    "Cannot commit ProposedChange {}: status is {:?}, expected Approved",
                    change.id, change.status
                ));
            }

            // ============================================================
            // 不变量 2: Project isolation - change 必须属于当前 project
            // ============================================================
            if change.project_id != project_id {
                return Err(anyhow::anyhow!(
                    "Cannot commit ProposedChange {}: project_id {} does not match expected {}",
                    change.id, change.project_id, project_id
                ));
            }

            match &change.change_type {
                ProposedChangeType::StateChange => {
                    let state_key = change.payload.get("state_key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let new_value = change.payload.get("new_value")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    // ============================================================
                    // 不变量 3: Project-scoped 查询
                    // ============================================================
                    let old_state = state_repo::StateRepo::get_current_state_tx(
                        &mut *tx, project_id, change.target_entity_id, state_key,
                    ).await?;
                    let old_value = old_state.as_ref().map(|s| s.state_value.clone());
                    let expected_version = old_state.as_ref().map(|s| s.version);

                    // 记录变更历史（source of truth）
                    let record = state_repo::StateRepo::record_change_tx(
                        &mut *tx,
                        project_id, None, "STATE_CHANGE",
                        change.target_entity_id, state_key,
                        old_value, new_value.clone(),
                        Some("committer"),
                    ).await?;

                    // ============================================================
                    // 不变量 4: CAS - 使用 version 做 compare-and-swap
                    // ============================================================
                    state_repo::StateRepo::upsert_state_tx(
                        &mut *tx,
                        project_id, change.target_entity_id,
                        state_key, new_value,
                        expected_version,
                    ).await?;

                    // 标记 ProposedChange 为 Applied
                    // 使用 CAS: 只有从 Approved 才能转为 Applied
                    let rows_affected = validation_repo::ValidationRepo::update_status_with_guard_tx(
                        &mut *tx,
                        change.id,
                        ProposedChangeStatus::Applied,
                        ProposedChangeStatus::Approved,
                    ).await?;

                    if rows_affected == 0 {
                        return Err(anyhow::anyhow!(
                            "Cannot commit ProposedChange {}: status transition from Approved to Applied failed (concurrent modification or invalid state)",
                            change.id
                        ));
                    }

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
