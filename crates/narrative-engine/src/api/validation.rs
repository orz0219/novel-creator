//! Validation API handlers
//!
//! 这三个接口此前**返回编造的「校验通过」**：
//!   `Ok(Json(json!([{"severity": "Info", "message": "Scene validation passed"}])))`
//! 既不查库也不跑校验器 —— 用户以为内容被检查过、而且没问题，实际什么都没发生。
//! 这比缺功能更危险，因为它给出的是**错误的放心**。
//!
//! 按本项目既有的做法（见 api/context.rs 的 pin/exclude），
//! 未实现的接口一律返回 501 并把原因写清楚，不假装成功。
//! 真正接上 Validator 之后再把这里换成实现。
use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;
use crate::state::AppState;
use super::error::AppError;

fn not_implemented(what: &str, target: &str) -> AppError {
    AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!(
            "{} 尚未接入校验器（目标 {}）：当前版本不会做任何检查，请不要把这里的结果当作校验结论",
            what,
            target
        ),
    )
}

pub async fn validate_scene(State(_state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Err(not_implemented("validate_scene", &scene_id))
}

pub async fn validate_proposal(State(_state): State<AppState>, Path(proposal_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Err(not_implemented("validate_proposal", &proposal_id))
}

pub async fn validate_world(State(_state): State<AppState>, Path(world_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Err(not_implemented("validate_world", &world_id))
}
