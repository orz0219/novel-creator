//! World API handlers
//!
//! 通过 application::world_service::WorldService（依赖 WorldRepositoryPort）。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::world_service::WorldService;
use db::application_ports::DbWorldRepositoryPort;
use std::sync::Arc;

fn service(state: &AppState) -> WorldService {
    WorldService::new(Arc::new(DbWorldRepositoryPort::new(state.pool.clone())))
}

#[derive(Deserialize)]
pub struct UpdateWorldInput { pub name: Option<String>, pub description: Option<String>, pub world_rules: Option<String> }

pub async fn get_world(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let world = service(&state)
        .get_or_create_main_world(project_id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Project not found")))?;
    let value = serde_json::to_value(&world).map_err(|e| AppError(anyhow::anyhow!(e)))?;
    Ok(Json(value))
}

pub async fn update_world(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<UpdateWorldInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let world = service(&state)
        .update_main_world(
            project_id,
            input.name.as_deref(),
            input.description.as_deref(),
            input.world_rules.as_deref(),
        )
        .await?;
    let value = serde_json::to_value(&world).map_err(|e| AppError(anyhow::anyhow!(e)))?;
    Ok(Json(value))
}
