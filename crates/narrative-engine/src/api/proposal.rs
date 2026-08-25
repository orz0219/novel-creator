//! Proposal API handlers
//!
//! 所有 mutation 通过 application service (ProposalService)。
//! 状态转换必须通过 ProposedChangeStatus.can_transition_to() 验证。

use axum::extract::{Path, State};
use axum::Json;
use domain::validation::{ProposedChange, ProposedChangeStatus, ProposedChangeType};
use crate::state::AppState;
use super::error::AppError;
use application::proposal_service::ProposalService;
use application::mutation::MutationCommitter;
use db::application_ports::DbProposalRepositoryPort;
use db::mutation_committer::DbMutationCommitter;
use std::sync::Arc;
use uuid::Uuid;

/// 构建 ProposalService：repo + MutationCommitter（批准即提交，提案 九）。
fn proposal_service(state: &AppState) -> ProposalService {
    let committer = Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(
        state.pool.clone(),
    ))));
    ProposalService::new(
        Arc::new(DbProposalRepositoryPort::new(state.pool.clone())),
        committer,
    )
}

pub async fn list_proposals(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = proposal_service(&state);
    let proposals = service.list_proposals(project_id).await?;

    Ok(Json(serde_json::json!(proposals.into_iter().map(proposal_to_json).collect::<Vec<_>>())))
}

pub async fn get_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid proposal ID")))?;
    let service = proposal_service(&state);
    let proposal = service.get_proposal(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Proposal not found")))?;

    Ok(Json(proposal_to_json(proposal)))
}

/// 批准提案 - 通过 ProposalService，验证状态转换
pub async fn accept_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid proposal ID")))?;
    let service = proposal_service(&state);
    let proposal = service.approve_proposal(id).await?;

    Ok(Json(serde_json::json!({"id": proposal.id, "status": proposal.status.description()})))
}

/// 拒绝提案 - 通过 ProposalService，验证状态转换
pub async fn reject_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid proposal ID")))?;
    let service = proposal_service(&state);
    let proposal = service.reject_proposal(id).await?;

    Ok(Json(serde_json::json!({"id": proposal.id, "status": proposal.status.description()})))
}

/// 接受单条 change：每个 proposal 即一条 proposed_change，复用 approve_proposal 真正落库（经 MutationCommitter 提交到 Canon）。
pub async fn accept_change(State(state): State<AppState>, Path((proposal_id, change_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    let change_id = Uuid::parse_str(&change_id).map_err(|_| AppError(anyhow::anyhow!("Invalid change ID")))?;
    let service = proposal_service(&state);
    let proposal = service.approve_proposal(change_id).await?;
    Ok(Json(serde_json::json!({"proposal_id": proposal_id, "change_id": change_id, "accepted": true, "status": proposal.status.description()})))
}

/// 拒绝单条 change：复用 reject_proposal 真正落库。
pub async fn reject_change(State(state): State<AppState>, Path((proposal_id, change_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    let change_id = Uuid::parse_str(&change_id).map_err(|_| AppError(anyhow::anyhow!("Invalid change ID")))?;
    let service = proposal_service(&state);
    let proposal = service.reject_proposal(change_id).await?;
    Ok(Json(serde_json::json!({"proposal_id": proposal_id, "change_id": change_id, "rejected": true, "status": proposal.status.description()})))
}

/// 将 ProposedChange 映射为前端 Proposal JSON（含真实 changes 列表）。
///
/// 后端 `proposed_change` 表每行即一条 change，故 `changes` 由该条记录直接构造；
/// `validation_results` 当前无持久化来源（校验结果由 Validator 按需计算），暂为空。
fn proposal_to_json(p: ProposedChange) -> serde_json::Value {
    let accepted = matches!(
        &p.status,
        ProposedChangeStatus::Approved | ProposedChangeStatus::Committed | ProposedChangeStatus::Applied
    );
    let state_change = if p.change_type == ProposedChangeType::StateChange {
        Some(p.payload.clone())
    } else {
        None
    };
    serde_json::json!({
        "id": p.id,
        "generation_task_id": p.task_id,
        "status": p.status.description(),
        "changes": [{
            "id": p.id,
            "change_type": map_change_type(&p.change_type),
            "target_entity_type": map_entity_type(&p.change_type),
            "target_entity_id": p.target_entity_id,
            "target_entity_name": null,
            "state_change": state_change,
            "description": p.description,
            "risk_level": "Low",
            "accepted": accepted
        }],
        "validation_results": [],
        "reason": p.description,
        "created_at": p.created_at
    })
}

fn map_change_type(ct: &ProposedChangeType) -> &'static str {
    match ct {
        ProposedChangeType::EntityCreate
        | ProposedChangeType::RelationCreate
        | ProposedChangeType::EventCreate => "Added",
        ProposedChangeType::EntityDelete | ProposedChangeType::RelationDelete => "Removed",
        _ => "Modified",
    }
}

fn map_entity_type(ct: &ProposedChangeType) -> String {
    match ct {
        ProposedChangeType::StateChange => "State",
        ProposedChangeType::EntityCreate
        | ProposedChangeType::EntityUpdate
        | ProposedChangeType::EntityDelete => "Entity",
        ProposedChangeType::RelationCreate
        | ProposedChangeType::RelationUpdate
        | ProposedChangeType::RelationDelete => "Relation",
        ProposedChangeType::EventCreate => "Event",
        ProposedChangeType::KnowledgeUpdate => "Knowledge",
        ProposedChangeType::Custom(_) => "Custom",
    }
    .to_string()
}
