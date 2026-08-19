//! Proposal API handlers
//!
//! 所有 mutation 通过 application service (ProposalService)。
//! 状态转换必须通过 ProposedChangeStatus.can_transition_to() 验证。

use axum::extract::{Path, State};
use axum::Json;
use crate::state::AppState;
use super::error::AppError;
use application::proposal_service::ProposalService;
use uuid::Uuid;

pub async fn list_proposals(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = ProposalService::new(state.pool.clone());
    let proposals = service.list_proposals(project_id).await?;

    Ok(Json(serde_json::json!(proposals.into_iter().map(|p| {
        serde_json::json!({"id": p.id, "generation_task_id": p.task_id, "status": p.status.description(), "changes": [], "validation_results": [], "reason": p.description, "created_at": p.created_at})
    }).collect::<Vec<_>>())))
}

pub async fn get_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid proposal ID")))?;
    let service = ProposalService::new(state.pool.clone());
    let proposal = service.get_proposal(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Proposal not found")))?;

    Ok(Json(serde_json::json!({"id": proposal.id, "generation_task_id": proposal.task_id, "status": proposal.status.description(), "changes": [], "validation_results": [], "reason": proposal.description, "created_at": proposal.created_at})))
}

/// 批准提案 - 通过 ProposalService，验证状态转换
pub async fn accept_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid proposal ID")))?;
    let service = ProposalService::new(state.pool.clone());
    let proposal = service.approve_proposal(id).await?;

    Ok(Json(serde_json::json!({"id": proposal.id, "status": proposal.status.description()})))
}

/// 拒绝提案 - 通过 ProposalService，验证状态转换
pub async fn reject_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid proposal ID")))?;
    let service = ProposalService::new(state.pool.clone());
    let proposal = service.reject_proposal(id).await?;

    Ok(Json(serde_json::json!({"id": proposal.id, "status": proposal.status.description()})))
}

pub async fn accept_change(State(_state): State<AppState>, Path((proposal_id, change_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    // Change-level accept - for now just return success
    Ok(Json(serde_json::json!({"proposal_id": proposal_id, "change_id": change_id, "accepted": true})))
}

pub async fn reject_change(State(_state): State<AppState>, Path((proposal_id, change_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"proposal_id": proposal_id, "change_id": change_id, "rejected": true})))
}
