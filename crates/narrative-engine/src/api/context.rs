//! Context API handlers
use axum::extract::{Path, State};
use axum::http::StatusCode;
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

// 以下 pin/unpin/exclude/unexclude 在持久层尚无实现（context 引擎未接线）。
// 按"错误显式暴露"原则，返回 501 Not Implemented，而非假成功 JSON 误导前端。

pub async fn pin_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("pin_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}

pub async fn unpin_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("unpin_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}

pub async fn exclude_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("exclude_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}

pub async fn unexclude_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("unexclude_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}
