//! 全局应用设置接口（设置页持久化）。

use axum::{extract::State, Json};
use serde_json::Value;
use crate::api::error::AppError;
use crate::state::AppState;
use db::application_ports::DbSettingsRepositoryPort;

pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let port = DbSettingsRepositoryPort::new(state.pool.clone());
    let settings = port.get_settings().await?;
    Ok(Json(settings))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let port = DbSettingsRepositoryPort::new(state.pool.clone());
    let settings = port.upsert_settings(input).await?;
    Ok(Json(settings))
}
