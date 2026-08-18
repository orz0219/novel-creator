//! Context API handlers
use axum::extract::{Path, State};
use axum::Json;
use crate::state::AppState;
use super::error::AppError;

pub async fn get_context(State(_state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    // Return mock context structure - will be connected to runtime context engine later
    Ok(Json(serde_json::json!({"id": format!("ctx-{}", scene_id), "scene_id": scene_id, "entities": [], "items": [], "total_tokens": 0, "created_at": chrono::Utc::now().to_rfc3339()})))
}

pub async fn build_context(State(_state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"id": format!("ctx-{}", scene_id), "scene_id": scene_id, "entities": [], "items": [], "total_tokens": 0, "created_at": chrono::Utc::now().to_rfc3339()})))
}

pub async fn pin_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"scene_id": scene_id, "entity_id": entity_id, "policy": "Pinned"})))
}

pub async fn unpin_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"scene_id": scene_id, "entity_id": entity_id, "policy": "Automatic"})))
}

pub async fn exclude_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"scene_id": scene_id, "entity_id": entity_id, "policy": "Excluded"})))
}

pub async fn unexclude_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"scene_id": scene_id, "entity_id": entity_id, "policy": "Automatic"})))
}
