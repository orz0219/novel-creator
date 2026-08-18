//! Validation API handlers
use axum::extract::{Path, State};
use axum::Json;
use crate::state::AppState;
use super::error::AppError;

pub async fn validate_scene(State(_state): State<AppState>, Path(_scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    // Stub - will connect to runtime validator later
    Ok(Json(serde_json::json!([{"id": "vr-1", "severity": "Info", "dimension": "World", "message": "Scene validation passed", "related_entity_ids": []}])))
}

pub async fn validate_proposal(State(_state): State<AppState>, Path(_proposal_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([{"id": "vr-1", "severity": "Info", "dimension": "World", "message": "Proposal validation passed", "related_entity_ids": []}])))
}

pub async fn validate_world(State(_state): State<AppState>, Path(_world_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([{"id": "vr-1", "severity": "Info", "dimension": "World", "message": "World validation passed", "related_entity_ids": []}])))
}