//! Context API handlers
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use crate::state::AppState;
use super::error::AppError;

pub async fn get_context(State(_state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    // 真实上下文由 runtime ContextEngine 组装；此处显式暴露未接线，避免假成功误导前端。
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("get_context not implemented for scene {}", scene_id),
    ))
}

pub async fn build_context(State(_state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("build_context not implemented for scene {}", scene_id),
    ))
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
