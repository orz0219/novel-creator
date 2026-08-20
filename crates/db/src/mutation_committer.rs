//! DbMutationCommitter - `MutationCommitterPort` 的 PostgreSQL 实现
//!
//! 这是 World Canon 唯一允许修改 canonical 状态的地方（经端口）。
//! 它复用各 repo 的 `_tx` 方法，在单一事务内完成：
//! CAS 校验 → 投影更新 → StateChange → DomainEvent(system_event) → 历史写入。
//!
//! 设计要点（提案 二十 / 二十一）：
//! - 一次 Canon mutation 必须在一个 transaction 内完成，任一步失败整体 ROLLBACK。
//! - 业务层禁止物理 DELETE：关系结束用 `valid_until`，事件只 INSERT 不 UPDATE/DELETE。
//! - 幂等：同一 `command_id` 重复提交返回同一结果（提案 二十七）。

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use domain::events::{DomainEvent, DomainEventType};
use domain::mutation::*;
use domain::{WorldVersion, WorldVersionKind};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::repos::world_version_repo::WorldVersionRepo;

pub struct DbMutationCommitter {
    pool: PgPool,
    world_version: WorldVersionRepo,
}

impl DbMutationCommitter {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            world_version: WorldVersionRepo::new(pool),
        }
    }
}

#[async_trait]
impl MutationCommitterPort for DbMutationCommitter {
    async fn commit(
        &self,
        cmd: MutationCommand,
    ) -> Result<MutationCommitResult, MutationError> {
        let batch = MutationBatch {
            commands: vec![cmd.clone()],
            affected_worlds: vec![],
            source: cmd.source,
            plan_id: None,
        };
        let mut results = self.commit_batch(batch).await?;
        Ok(results.remove(0))
    }

    async fn commit_batch(
        &self,
        batch: MutationBatch,
    ) -> Result<Vec<MutationCommitResult>, MutationError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin mutation transaction")?;
        let mut out = Vec::with_capacity(batch.commands.len());
        // 在移动 batch.commands 之前先记录长度与首命令 id，供下方 world_version 元数据使用。
        let n_cmds = batch.commands.len();
        let first_cmd_id = batch.commands.first().map(|c| c.command_id);

        for cmd in batch.commands {
            // 幂等：mutation_ledger(command_id) 唯一约束
            if let Some(cached) = ledger_try_insert(&mut *tx, &cmd).await? {
                out.push(cached);
                continue;
            }

            let result = apply(&mut tx, cmd).await?;
            ledger_mark_done(&mut *tx, &result).await?;
            out.push(result);
        }

        // 同一事务内为每个受影响世界产出 world_version（git-commit 式），
        // 保证「Canon commit == world version commit」是同一原子事实。
        // 不在此处从 command 推断 world_id——由调用方在 batch.affected_worlds 显式给出。
        let mut worlds = batch.affected_worlds.clone();
        worlds.sort_unstable();
        worlds.dedup();
        for world_id in worlds {
            let parent = self.world_version.latest_tx(&mut *tx, world_id).await?;
            let new_version = parent.as_ref().map(|p| p.version).unwrap_or(0) + 1;
            let parent_id = parent.as_ref().map(|p| p.id);
            let kind = match batch.source {
                MutationSource::User => WorldVersionKind::UserEdit,
                MutationSource::AI => WorldVersionKind::AiProposal,
                MutationSource::System => WorldVersionKind::System,
            };
            let trigger = batch.plan_id.or(first_cmd_id);
            let v = WorldVersion {
                id: Uuid::new_v4(),
                world_id,
                version: new_version,
                kind,
                trigger_id: trigger,
                summary: Some(format!(
                    "MutationCommit: {} commands (source={})",
                    n_cmds,
                    batch.source.as_str()
                )),
                parent_version_id: parent_id,
                created_at: Utc::now(),
            };
            self.world_version.create_tx(&mut *tx, &v).await?;
        }

        tx.commit()
            .await
            .context("Failed to commit mutation transaction")?;
        Ok(out)
    }
}

/// 尝试登记 command_id；若已存在则取回缓存结果（幂等）。
async fn ledger_try_insert(
    executor: &mut PgConnection,
    cmd: &MutationCommand,
) -> Result<Option<MutationCommitResult>, MutationError> {
    let res = sqlx::query(
        "INSERT INTO mutation_ledger (command_id, project_id, status, created_at) \
         VALUES ($1, $2, 'committed', NOW()) ON CONFLICT (command_id) DO NOTHING",
    )
    .bind(cmd.command_id)
    .bind(cmd.project_id)
    .execute(&mut *executor)
    .await
    .context("Failed to insert mutation ledger")?;

    if res.rows_affected() == 0 {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT result FROM mutation_ledger WHERE command_id = $1",
        )
        .bind(cmd.command_id)
        .fetch_optional(&mut *executor)
        .await
        .context("Failed to read cached mutation result")?;
        if let Some((val,)) = row {
            return Ok(Some(
                serde_json::from_value(val).unwrap_or_else(|_| MutationCommitResult::new(cmd.command_id)),
            ));
        }
    }
    Ok(None)
}

async fn ledger_mark_done(
    executor: &mut PgConnection,
    result: &MutationCommitResult,
) -> Result<(), MutationError> {
    sqlx::query("UPDATE mutation_ledger SET result = $2 WHERE command_id = $1")
        .bind(result.command_id)
        .bind(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
        .execute(executor)
        .await
        .context("Failed to update mutation ledger")
        .map_err(MutationError::from)?;
    Ok(())
}

/// 在事务内落实单条命令。
async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cmd: MutationCommand,
) -> Result<MutationCommitResult, MutationError> {
    let project_id = cmd.project_id;
    let source = cmd.source;
    let mut result = MutationCommitResult::new(cmd.command_id);

    match cmd.payload {
        MutationPayload::CreateEntity {
            world_id,
            entity_type,
            name,
            summary,
            description,
            attributes,
        } => {
            let et = crate::repos::entity_repo::EntityTypeRepo::ensure_tx(&mut **tx, &entity_type, None)
                .await?;
            let entity = crate::repos::entity_repo::EntityRepo::create_tx(
                &mut **tx,
                project_id,
                world_id,
                et.id,
                &name,
                summary.as_deref(),
                description.as_deref(),
                attributes,
            )
            .await?;
            record_event(
                &mut **tx,
                project_id,
                Some(entity.id),
                DomainEventType::EntityCreated,
                source,
                serde_json::json!({ "entity_type": entity_type }),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(entity.id);
            result.created_ids.push(entity.id);
            result.new_versions.insert(entity.id, 1);
        }
        MutationPayload::UpdateEntity {
            name,
            summary,
            description,
            attributes,
        } => {
            let mut entity = crate::repos::entity_repo::EntityRepo::get_by_id_with_project_tx(
                &mut **tx,
                project_id,
                cmd.target,
            )
            .await?
            .ok_or_else(|| MutationError::NotFound(format!("entity {}", cmd.target)))?;
            if let Some(v) = name {
                entity.name = v;
            }
            if let Some(v) = summary {
                entity.summary = Some(v);
            }
            if let Some(v) = description {
                entity.description = Some(v);
            }
            if let Some(v) = attributes {
                entity.attributes = v;
            }
            let expected = cmd.expected_version.unwrap_or(entity.version);
            let rows = crate::repos::entity_repo::EntityRepo::update_tx(&mut **tx, &entity).await?;
            if rows == 0 {
                return Err(MutationError::ConcurrentModification {
                    target: cmd.target,
                    expected,
                });
            }
            record_event(
                &mut **tx,
                project_id,
                Some(cmd.target),
                DomainEventType::EntityUpdated,
                source,
                serde_json::json!({}),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
            result.new_versions.insert(cmd.target, expected + 1);
        }
        MutationPayload::DeleteEntity => {
            let expected = cmd
                .expected_version
                .ok_or_else(|| MutationError::Validation("DeleteEntity requires expected_version".into()))?;
            let ok = crate::repos::entity_repo::EntityRepo::delete_tx(&mut **tx, project_id, cmd.target, expected)
                .await?;
            if !ok {
                return Err(MutationError::ConcurrentModification {
                    target: cmd.target,
                    expected,
                });
            }
            record_event(
                &mut **tx,
                project_id,
                Some(cmd.target),
                DomainEventType::EntityDeleted,
                source,
                serde_json::Value::Null,
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
        MutationPayload::CreateRelation {
            target_entity_id,
            relation_type,
            description,
            attributes,
        } => {
            // cmd.target 是 source entity
            let _source = crate::repos::entity_repo::EntityRepo::get_by_id_with_project_tx(
                &mut **tx,
                project_id,
                cmd.target,
            )
            .await?
            .ok_or_else(|| MutationError::NotFound(format!("source entity {}", cmd.target)))?;
            let _target = crate::repos::entity_repo::EntityRepo::get_by_id_with_project_tx(
                &mut **tx,
                project_id,
                target_entity_id,
            )
            .await?
            .ok_or_else(|| MutationError::NotFound(format!("target entity {}", target_entity_id)))?;
            let relation = crate::repos::entity_repo::RelationRepo::create_tx(
                &mut **tx,
                project_id,
                cmd.target,
                target_entity_id,
                &relation_type,
                description.as_deref(),
                attributes.unwrap_or(serde_json::json!({})),
            )
            .await?;
            record_event(
                &mut **tx,
                project_id,
                Some(cmd.target),
                DomainEventType::Custom("RelationCreated".to_string()),
                source,
                serde_json::json!({ "relation_id": relation.id, "relation_type": relation_type }),
                &mut result,
            )
            .await?;
            result.created_ids.push(relation.id);
            result.affected_entity_ids.push(cmd.target);
            result.affected_entity_ids.push(target_entity_id);
        }
        MutationPayload::EndRelation { valid_until } => {
            // 业务层禁止物理 DELETE：仅设置 valid_until（提案 五）
            let ok = crate::repos::entity_repo::RelationRepo::end_relation_tx(
                &mut **tx,
                project_id,
                cmd.target,
                valid_until.clone(),
            )
            .await?;
            if !ok {
                return Err(MutationError::NotFound(format!(
                    "active relation {} not found",
                    cmd.target
                )));
            }
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("RelationEnded".to_string()),
                source,
                serde_json::json!({ "relation_id": cmd.target, "valid_until": valid_until }),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
        MutationPayload::CreateEvent {
            name,
            description,
            event_type,
            event_time,
        } => {
            // 世界事件：不可变，只允许 INSERT，不 UPDATE/DELETE（提案 七）
            let event = crate::repos::event_repo::EventRepo::create_tx(
                &mut **tx,
                project_id,
                &name,
                &description,
                event_type.as_deref(),
                event_time.as_deref(),
            )
            .await?;
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("WorldEventCreated".to_string()),
                source,
                serde_json::json!({ "event_id": event.id }),
                &mut result,
            )
            .await?;
            result.created_ids.push(event.id);
            result.event_ids.push(event.id);
        }
        MutationPayload::SetEntityState {
            state_key,
            new_value,
        } => {
            // 走成熟的 StateRepository（提案 八）：CAS + 事务 + current_state 投影
            let event = DomainEvent::new(
                DomainEventType::Custom("StateChanged".to_string()),
                project_id,
                Some(cmd.target),
                serde_json::json!({ "state_key": state_key }),
            );
            sqlx::query(
                "INSERT INTO system_events (id, event_type, project_id, entity_id, data, source, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(event.id)
            .bind(format!("{:?}", event.event_type))
            .bind(event.project_id)
            .bind(event.entity_id)
            .bind(&event.data)
            .bind(source.as_str())
            .bind(event.created_at)
            .execute(&mut **tx)
            .await
            .context("Failed to persist state-change domain event")
            .map_err(MutationError::from)?;
            result.event_ids.push(event.id);

            let (record, new_version) = crate::repos::state_repo::StateRepo::commit_state_change_tx(
                &mut **tx,
                project_id,
                Some(event.id),
                "STATE_CHANGE",
                cmd.target,
                &state_key,
                new_value,
                Some(source.as_str()),
            )
            .await?;
            result.state_change_ids.push(record.id);
            result.affected_entity_ids.push(cmd.target);
            result.new_versions.insert(cmd.target, new_version);
        }
        MutationPayload::CreateFact {
            content,
            category,
            related_entity_ids,
        } => {
            // 事实仅允许 INSERT；不可 UPDATE / DELETE（提案 六）
            let related = related_entity_ids.as_deref().unwrap_or(&[]);
            let fact = crate::repos::entity_repo::FactRepo::create_tx(
                &mut **tx,
                project_id,
                &content,
                category.as_deref(),
                "CANON",
                related,
            )
            .await?;
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("FactCreated".to_string()),
                source,
                serde_json::json!({ "fact_id": fact.id }),
                &mut result,
            )
            .await?;
            result.created_ids.push(fact.id);
        }
        MutationPayload::SupersedeFact { superseded_by } => {
            let ok = crate::repos::entity_repo::FactRepo::set_status_tx(
                &mut **tx,
                project_id,
                cmd.target,
                "Superseded",
                Some(superseded_by),
            )
            .await?;
            if !ok {
                return Err(MutationError::NotFound(format!("fact {}", cmd.target)));
            }
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("FactSuperseded".to_string()),
                source,
                serde_json::json!({ "fact_id": cmd.target, "superseded_by": superseded_by }),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
        MutationPayload::InvalidateFact => {
            let ok = crate::repos::entity_repo::FactRepo::set_status_tx(
                &mut **tx,
                project_id,
                cmd.target,
                "Invalid",
                None,
            )
            .await?;
            if !ok {
                return Err(MutationError::NotFound(format!("fact {}", cmd.target)));
            }
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("FactInvalidated".to_string()),
                source,
                serde_json::json!({ "fact_id": cmd.target }),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
        MutationPayload::UpdateNarrativeNode {
            title,
            description,
            attributes,
            content,
            status,
        } => {
            // 叙事节点乐观锁（提案 四 / 六）：CAS on version，绝不物理 DELETE。
            let current_version: Option<i32> =
                sqlx::query_scalar("SELECT version FROM narrative_node WHERE id=$1 AND project_id=$2")
                    .bind(cmd.target)
                    .bind(project_id)
                    .fetch_optional(&mut **tx)
                    .await
                    .context("Failed to read narrative node version")?;
            let mut node = crate::repos::narrative_repo::NarrativeRepo::get_node_by_id_with_project_tx(
                &mut **tx,
                project_id,
                cmd.target,
            )
            .await?
            .ok_or_else(|| MutationError::NotFound(format!("narrative node {}", cmd.target)))?;
            if let Some(v) = title {
                node.title = v;
            }
            if let Some(v) = description {
                node.description = Some(v);
            }
            if let Some(v) = content {
                node.content = Some(v);
            }
            if let Some(v) = attributes {
                node.attributes = v;
            }
            if let Some(v) = status {
                node.status = crate::ser::parse_narrative_node_status(&v);
            }
            let expected = current_version.unwrap_or(1);
            let ok = crate::repos::narrative_repo::NarrativeRepo::update_node_tx(&mut **tx, &node, expected)
                .await?;
            if !ok {
                return Err(MutationError::ConcurrentModification {
                    target: cmd.target,
                    expected,
                });
            }
            record_event(
                &mut **tx,
                project_id,
                Some(cmd.target),
                DomainEventType::Custom("NarrativeNodeUpdated".to_string()),
                source,
                serde_json::json!({}),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
            result.new_versions.insert(cmd.target, expected + 1);
        }
        MutationPayload::DeleteNarrativeNode => {
            // 叙事节点软删除（含子节点），绝不物理 DELETE（提案 二十二）。
            let n = crate::repos::narrative_repo::NarrativeRepo::soft_delete_node_tx(&mut **tx, cmd.target)
                .await?;
            if n == 0 {
                return Err(MutationError::NotFound(format!("narrative node {}", cmd.target)));
            }
            record_event(
                &mut **tx,
                project_id,
                Some(cmd.target),
                DomainEventType::Custom("NarrativeNodeDeleted".to_string()),
                source,
                serde_json::json!({}),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
        MutationPayload::UpdateStoryline { title, description, .. } => {
            // storyline 表无 version 列，暂不做 CAS（提案 六）
            let ok = crate::repos::storyline_repo::StorylineRepo::update_tx(
                &mut **tx,
                cmd.target,
                project_id,
                title.as_deref(),
                description.as_deref(),
            )
            .await?;
            if !ok {
                return Err(MutationError::NotFound(format!("storyline {}", cmd.target)));
            }
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("StorylineUpdated".to_string()),
                source,
                serde_json::json!({}),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
        MutationPayload::UpdateForeshadow { title, description, status } => {
            // foreshadowing 表无 version 列，暂不做 CAS（提案 六）
            let ok = crate::repos::foreshadowing_repo::ForeshadowingRepo::update_tx(
                &mut **tx,
                cmd.target,
                project_id,
                title.as_deref(),
                description.as_deref(),
                status.as_deref(),
            )
            .await?;
            if !ok {
                return Err(MutationError::NotFound(format!("foreshadow {}", cmd.target)));
            }
            record_event(
                &mut **tx,
                project_id,
                None,
                DomainEventType::Custom("ForeshadowUpdated".to_string()),
                source,
                serde_json::json!({}),
                &mut result,
            )
            .await?;
            result.affected_entity_ids.push(cmd.target);
        }
    }

    Ok(result)
}

/// 写入一条 DomainEvent 到 system_event，并记入本次提交结果。
async fn record_event(
    executor: &mut PgConnection,
    project_id: Uuid,
    entity_id: Option<Uuid>,
    etype: DomainEventType,
    source: MutationSource,
    data: serde_json::Value,
    result: &mut MutationCommitResult,
) -> Result<(), MutationError> {
    let event = DomainEvent::new(etype, project_id, entity_id, data);
    sqlx::query(
        "INSERT INTO system_events (id, event_type, project_id, entity_id, data, source, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(event.id)
    .bind(format!("{:?}", event.event_type))
    .bind(event.project_id)
    .bind(event.entity_id)
    .bind(&event.data)
    .bind(source.as_str())
    .bind(event.created_at)
    .execute(executor)
    .await
    .context("Failed to persist domain event")
    .map_err(MutationError::from)?;
    result.event_ids.push(event.id);
    Ok(())
}
