//! Proposal API handlers
use axum::extract::{Path, State};
use axum::Json;
use crate::state::AppState;
use super::error::AppError;

pub async fn list_proposals(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, String)>(
        "SELECT id, status, COALESCE(generation_task_id, ''), description, created_at::text FROM proposed_change WHERE project_id = $1 ORDER BY created_at DESC"
    ).bind(&project_id).fetch_all(&state.pool).await?;

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, status, task_id, desc, created)| {
        serde_json::json!({"id": id, "generation_task_id": task_id, "status": status, "changes": [], "validation_results": [], "reason": desc, "created_at": created})
    }).collect::<Vec<_>>())))
}

pub async fn get_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, String)>(
        "SELECT id, status, COALESCE(generation_task_id, ''), description, created_at::text FROM proposed_change WHERE id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;

    match row {
        Some(r) => Ok(Json(serde_json::json!({"id": r.0, "generation_task_id": r.2, "status": r.1, "changes": [], "validation_results": [], "reason": r.3, "created_at": r.4}))),
        None => Err(AppError(anyhow::anyhow!("Proposal not found")))
    }
}

pub async fn accept_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE proposed_change SET status = 'Accepted' WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "status": "Accepted"})))
}

pub async fn reject_proposal(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE proposed_change SET status = 'Rejected' WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "status": "Rejected"})))
}

pub async fn accept_change(State(state): State<AppState>, Path((proposal_id, change_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    // Change-level accept - for now just return success
    Ok(Json(serde_json::json!({"proposal_id": proposal_id, "change_id": change_id, "accepted": true})))
}

pub async fn reject_change(State(state): State<AppState>, Path((proposal_id, change_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"proposal_id": proposal_id, "change_id": change_id, "rejected": true})))
}
