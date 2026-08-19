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
use db::repos::{entity_repo, state_repo, validation_repo};
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
    ///   - P1-2: commit 前重新从 DB 加载 proposal，不依赖传入的快照
    ///
    /// 任何一步失败 → ROLLBACK。
    pub async fn commit(
        &self,
        project_id: Uuid,
        change_ids: &[Uuid],
    ) -> Result<Vec<StateChangeRecord>> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        let mut records = Vec::new();

        for change_id in change_ids {
            // ============================================================
            // P1-2: 从数据库重新加载 proposal 的权威版本
            // 不依赖调用者传入的快照，防止并发修改导致的数据不一致
            // ============================================================
            let change = validation_repo::ValidationRepo::get_proposed_change_by_id_tx(
                &mut *tx, *change_id
            ).await?
            .ok_or_else(|| anyhow::anyhow!(
                "ProposedChange {} not found in database", change_id
            ))?;

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

            // ============================================================
            // P2-5: 验证 target_entity_id 存在且属于当前 project
            // 不只依赖 Validator 的检查，commit 本身也要验证
            // ============================================================
            let entity = entity_repo::EntityRepo::get_by_id_with_project(
                &entity_repo::EntityRepo::new(self.pool.clone()),
                project_id,
                change.target_entity_id,
            ).await?;

            if entity.is_none() {
                return Err(anyhow::anyhow!(
                    "Cannot commit ProposedChange {}: target entity {} not found in project {}",
                    change.id, change.target_entity_id, project_id
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
                    // P1-1: Unsupported change type 必须失败并 rollback
                    // 不能静默跳过，否则会造成"部分提交"的错误语义
                    return Err(anyhow::anyhow!(
                        "Unsupported change type: {:?}.                          StateCommitter only supports StateChange type.                          All changes in a batch must be supported, otherwise the entire transaction rolls back.",
                        change.change_type
                    ));
                }
            }
        }

        // 提交事务 - 如果前面任何步骤失败，tx 会被 drop 导致 ROLLBACK
        tx.commit().await.context("Failed to commit transaction")?;

        tracing::info!("Committed {} changes in a single transaction", records.len());
        Ok(records)
    }
}
