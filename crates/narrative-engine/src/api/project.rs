//! Project API handlers
//!
//! 所有读写通过 application::project_service::ProjectService（依赖端口），
//! 不再直接持有 PgPool 执行 SQL。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::project_service::ProjectService;
use application::world_service::WorldService;
use db::application_ports::{DbProjectRepositoryPort, DbWorldRepositoryPort};
use std::sync::Arc;

fn service(state: &AppState) -> ProjectService {
    ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(state.pool.clone())),
        Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(
            state.pool.clone(),
        )))),
    )
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CreateProjectInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub description: Option<String>,
    /// 故事脑洞/前提（Agent 引导用户确认后由工具写入）。
    pub premise: Option<String>,
}

pub async fn list_projects(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let projects = service(&state).list_projects().await?;
    Ok(Json(serde_json::json!(projects)))
}

pub async fn get_project(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let project = service(&state)
        .get_project(id)
        .await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Project not found")))?;
    Ok(Json(project))
}

pub async fn create_project(State(state): State<AppState>, Json(input): Json<CreateProjectInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project = service(&state)
        .create_project(&input.name, input.description.as_deref(), None)
        .await?;
    Ok(Json(project))
}

pub async fn update_project(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateProjectInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let project = service(&state)
        .update_project(
            id,
            input.name.as_deref(),
            input.description.as_deref(),
            None,
            input.premise.as_deref(),
        )
        .await?;
    Ok(Json(project))
}

pub async fn delete_project(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    service(&state).delete_project(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
