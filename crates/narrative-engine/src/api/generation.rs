//! Generation API handlers
//!
//! 所有 mutation 通过 application service (GenerationService)。
//! 取消任务时验证状态。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use std::sync::Arc;
use db::application_ports::DbGenerationRepositoryPort;
use super::error::AppError;
use application::generation_service::GenerationService;

#[derive(Deserialize)]
pub struct CreateGenerationInput { pub r#type: String, pub target_id: Option<String>, pub model: Option<String>, pub parameters: Option<serde_json::Value> }

pub async fn list_tasks(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    let tasks = service.list_tasks(project_id).await?;
    Ok(Json(serde_json::json!(tasks)))
}

pub async fn get_task(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid task ID")))?;
    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    let task = service.get_task(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Generation task not found")))?;
    Ok(Json(task))
}

pub async fn create_task(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateGenerationInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let target_id = input.target_id.map(|t| Uuid::parse_str(&t)).transpose()
        .map_err(|_| AppError(anyhow::anyhow!("Invalid target ID")))?;

    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    let task = service.create_task(
        project_id,
        &input.r#type,
        target_id,
        input.model.as_deref(),
        input.parameters.unwrap_or(serde_json::json!({})),
    ).await?;

    Ok(Json(task))
}

pub async fn cancel_task(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid task ID")))?;
    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    service.cancel_task(id).await?;
    Ok(Json(serde_json::json!({"id": id, "status": "Cancelled"})))
}