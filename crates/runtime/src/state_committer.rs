//! StateCommitter - 事务化提交状态变更
//!
//! 将 ProposedChange 列表事务化提交到世界状态。
//! 所有操作在同一个数据库事务中执行，任何一步失败则整体 ROLLBACK。
//!
//! 核心不变量：
//! - 只有 Approved 的 ProposedChange 才能进入 StateCommitter
//! - 所有 State 查询必须具备 Project Isolation
//! - 使用 version CAS 防止并发覆盖
//! - P1-2: commit 前重新从 DB 加载 proposal，不依赖传入的快照
//! - P1-3: 创建真实 DomainEvent，绑定到 StateChangeRecord
//! - P1-4: 使用 commit_state_change_tx 原子操作
//! - P2-4: 使用类型化的 ChangePayload
//! - P2-5: 验证 target_entity_id 存在

use anyhow::{Context, Result};
use chrono::Utc;
use db::repos::{entity_repo, state_repo, validation_repo};
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

/// StateCommitter 实现 - 事务化提交状态变更
///
/// 所有 change 在单个 BEGIN/COMMIT 事务中提交。
/// 任何一步失败自动 ROLLBACK，保证原子性。
///
/// INVARIANT: self.pool is ONLY used for begin().
/// All repo operations MUST use _tx methods with &mut *tx.
/// FORBIDDEN: self.pool.clone() inside commit() for creating repos.
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
    ///   - P1-2: commit 前重新从 DB 加载 proposal
    ///   - P1-3: 创建真实 DomainEvent
    ///   - P1-4: 使用原子操作 commit_state_change_tx
    ///   - P2-4: 使用类型化 ChangePayload
    ///   - P2-5: 验证 entity 存在性
    ///
    /// 任何一步失败 → ROLLBACK。
    pub async fn commit(
        &self,
        project_id: Uuid,
        change_ids: &[Uuid],
    ) -> Result<CommitResponse> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;
        let mut results = Vec::new();
        let mut event_ids = Vec::new();

        for change_id in change_ids {
            // ============================================================
            // P1-2: 从数据库重新加载 proposal 的权威版本
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
            // 不变量 2: Project isolation
            // ============================================================
            if change.project_id != project_id {
                return Err(anyhow::anyhow!(
                    "Cannot commit ProposedChange {}: project_id {} does not match expected {}",
                    change.id, change.project_id, project_id
                ));
            }

            // ============================================================
            // P2-5: 验证 target_entity_id 存在 (within transaction)
            // ============================================================
            let entity = entity_repo::EntityRepo::get_by_id_with_project_tx(
                &mut *tx,
                project_id,
                change.target_entity_id,
            ).await?;

            if entity.is_none() {
                return Err(anyhow::anyhow!(
                    "Cannot commit ProposedChange {}: target entity {} not found in project {}",
                    change.id, change.target_entity_id, project_id
                ));
            }

            // ============================================================
            // P1-3: 创建 DomainEvent
            // ============================================================
            let event = DomainEvent::new(
                DomainEventType::ProposalCommitted,
                project_id,
                Some(change.target_entity_id),
                serde_json::json!({
                    "proposed_change_id": change.id,
                    "change_type": format!("{:?}", change.change_type),
                    "payload": change.payload,
                }),
            );

            // 持久化 event 到数据库
            sqlx::query(
                "INSERT INTO system_event (id, event_type, project_id, entity_id, data, source, created_at)                  VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(event.id)
            .bind(format!("{:?}", event.event_type))
            .bind(event.project_id)
            .bind(event.entity_id)
            .bind(&event.data)
            .bind(&event.metadata.source)
            .bind(event.created_at)
            .execute(&mut *tx)
            .await
            .context("Failed to persist DomainEvent")?;

            event_ids.push(event.id);

            // ============================================================
            // P2-4: 使用类型化 ChangePayload
            // ============================================================
            let payload: ChangePayload = serde_json::from_value(change.payload.clone())
                .unwrap_or_else(|_| ChangePayload::Custom(change.payload.clone()));

            match payload {
                ChangePayload::StateChange { state_key, new_value } => {
                    // ============================================================
                    // P1-4: 使用原子操作 commit_state_change_tx
                    // ============================================================
                    let (record, new_version) = state_repo::StateRepo::commit_state_change_tx(
                        &mut *tx,
                        project_id,
                        Some(event.id),
                        "STATE_CHANGE",
                        change.target_entity_id,
                        &state_key,
                        new_value,
                        Some("committer"),
                    ).await?;

                    // 标记 ProposedChange 为 Applied
                    let rows_affected = validation_repo::ValidationRepo::update_status_with_guard_tx(
                        &mut *tx,
                        change.id,
                        ProposedChangeStatus::Applied,
                        ProposedChangeStatus::Approved,
                    ).await?;

                    if rows_affected == 0 {
                        return Err(anyhow::anyhow!(
                            "Cannot commit ProposedChange {}: status transition failed",
                            change.id
                        ));
                    }

                    results.push(CommitResult::StateChange {
                        record,
                        new_version,
                    });
                }
                ChangePayload::EntityCreate { entity_type, name, attributes } => {
                    // P2-9: 支持实体创建 (within transaction)
                    let entity_type_obj = entity_repo::EntityTypeRepo::ensure_tx(
                        &mut *tx,
                        &entity_type,
                        None,
                    ).await?;

                    let world_id = sqlx::query_scalar::<_, Uuid>(
                        "SELECT id FROM world WHERE project_id = $1 AND is_main = TRUE LIMIT 1"
                    )
                    .bind(project_id)
                    .fetch_one(&mut *tx)
                    .await
                    .context("No main world found for project")?;

                    let entity = entity_repo::EntityRepo::create_tx(
                        &mut *tx,
                        project_id,
                        world_id,
                        entity_type_obj.id,
                        &name,
                        None,
                        None,
                        attributes,
                    ).await?;

                    // 标记 ProposedChange 为 Applied
                    validation_repo::ValidationRepo::update_status_with_guard_tx(
                        &mut *tx,
                        change.id,
                        ProposedChangeStatus::Applied,
                        ProposedChangeStatus::Approved,
                    ).await?;

                    results.push(CommitResult::EntityCreated {
                        entity_id: entity.id,
                        entity_name: entity.name,
                    });
                }
                ChangePayload::RelationCreate { target_entity_id, relation_type, attributes } => {
                    // P2-9: 支持关系创建 (within transaction)
                    let relation = entity_repo::RelationRepo::create_tx(
                        &mut *tx,
                        project_id,
                        change.target_entity_id,
                        target_entity_id,
                        &relation_type,
                        None,
                        attributes,
                    ).await?;

                    // 标记 ProposedChange 为 Applied
                    validation_repo::ValidationRepo::update_status_with_guard_tx(
                        &mut *tx,
                        change.id,
                        ProposedChangeStatus::Applied,
                        ProposedChangeStatus::Approved,
                    ).await?;

                    results.push(CommitResult::RelationCreated {
                        relation_id: relation.id,
                        source_entity_id: relation.source_entity_id,
                        target_entity_id: relation.target_entity_id,
                        relation_type: relation.relation_type,
                    });
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unsupported change payload type for ProposedChange {}",
                        change.id
                    ));
                }
            }
        }

        // 提交事务
        tx.commit().await.context("Failed to commit transaction")?;

        tracing::info!(
            "Committed {} changes with {} events in a single transaction",
            results.len(),
            event_ids.len()
        );

        Ok(CommitResponse {
            project_id,
            results,
            events: event_ids,
            committed_at: Utc::now(),
        })
    }
}