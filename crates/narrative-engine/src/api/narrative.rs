//! Narrative API handlers
//!
//! 所有 mutation 通过 application service (NarrativeService)。
//! 删除使用软删除（status=Deleted）。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;
use application::narrative_service::NarrativeService;
use application::storyline_service::StorylineService;
use application::foreshadow_service::ForeshadowService;
use application::mutation::MutationCommitter;
use db::application_ports::{DbStorylineRepositoryPort, DbForeshadowRepositoryPort};
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateNodeInput { pub node_type: String, pub parent_id: Option<String>, pub title: String, pub description: Option<String>, pub attributes: Option<serde_json::Value> }
#[derive(Deserialize)]
pub struct UpdateNodeInput { pub title: Option<String>, pub description: Option<String>, pub status: Option<String> }
#[derive(Deserialize)]
pub struct CreateStorylineInput { pub name: String, pub description: Option<String>, pub importance: Option<String> }
#[derive(Deserialize)]
pub struct CreateForeshadowInput { pub name: String, pub description: Option<String>, pub importance: Option<String>, pub hint_level: Option<String> }

/// 构建 NarrativeService：repo + MutationCommitter（统一写入口）+ ProjectResolver。
fn narrative_service(state: &AppState) -> NarrativeService {
    let pool = state.pool.clone();
    let committer = Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(
        pool.clone(),
    ))));
    let resolver = Arc::new(DbProjectResolverPort::new(pool.clone()));
    NarrativeService::new(
        Arc::new(db::application_ports::DbNarrativeRepositoryPort::new(pool)),
        committer,
        resolver,
    )
}

pub async fn list_nodes(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = narrative_service(&state);
    let nodes = service.list_nodes(project_id).await?;
    Ok(Json(serde_json::json!(nodes)))
}

pub async fn get_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = narrative_service(&state);
    let node = service.get_node(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Narrative node not found")))?;
    Ok(Json(node))
}

pub async fn create_node(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let parent_id = input.parent_id.map(|p| Uuid::parse_str(&p)).transpose()
        .map_err(|_| AppError(anyhow::anyhow!("Invalid parent ID")))?;

    let service = narrative_service(&state);
    let node = service.create_node(
        project_id,
        &input.node_type,
        parent_id,
        &input.title,
        input.description.as_deref(),
        input.attributes.unwrap_or(serde_json::json!({})),
    ).await?;

    Ok(Json(node))
}

pub async fn update_node(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = narrative_service(&state);
    let node = service.update_node(
        id,
        input.title.as_deref(),
        input.description.as_deref(),
        input.status.as_deref(),
    ).await?;

    Ok(Json(node))
}

pub async fn delete_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = narrative_service(&state);
    service.delete_node(id).await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

pub async fn list_storylines(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let storylines = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .list_storylines(project_id)
        .await?;
    Ok(Json(serde_json::json!(storylines)))
}

pub async fn create_storyline(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let storyline = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .create_storyline(project_id, &input.name, input.description.as_deref(), input.importance.as_deref().unwrap_or("Normal"))
        .await?;
    Ok(Json(storyline))
}

pub async fn update_storyline(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid storyline ID")))?;
    let storyline = StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(state.pool.clone())))
        .update_storyline(id, &input.name, input.description.as_deref())
        .await?;
    Ok(Json(storyline))
}

pub async fn list_foreshadows(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let foreshadows = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .list_foreshadows(project_id)
        .await?;
    Ok(Json(serde_json::json!(foreshadows)))
}

pub async fn create_foreshadow(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let foreshadow = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .create_foreshadow(
            project_id,
            &input.name,
            input.description.as_deref(),
            input.importance.as_deref().unwrap_or("Normal"),
            input.hint_level.as_deref().unwrap_or("Direct"),
        )
        .await?;
    Ok(Json(foreshadow))
}

pub async fn update_foreshadow(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid foreshadow ID")))?;
    let foreshadow = ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(state.pool.clone())))
        .update_foreshadow(id, &input.name, input.description.as_deref())
        .await?;
    Ok(Json(foreshadow))
}