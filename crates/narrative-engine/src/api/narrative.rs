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

#[derive(Deserialize)]
pub struct CreateNodeInput { pub node_type: String, pub parent_id: Option<String>, pub title: String, pub description: Option<String>, pub attributes: Option<serde_json::Value> }
#[derive(Deserialize)]
pub struct UpdateNodeInput { pub title: Option<String>, pub description: Option<String>, pub status: Option<String> }
#[derive(Deserialize)]
pub struct CreateStorylineInput { pub name: String, pub description: Option<String>, pub importance: Option<String> }
#[derive(Deserialize)]
pub struct CreateForeshadowInput { pub name: String, pub description: Option<String>, pub importance: Option<String>, pub hint_level: Option<String> }

pub async fn list_nodes(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = NarrativeService::new(state.pool.clone());
    let nodes = service.list_nodes(project_id).await?;
    Ok(Json(serde_json::json!(nodes)))
}

pub async fn get_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid node ID")))?;
    let service = NarrativeService::new(state.pool.clone());
    let node = service.get_node(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Narrative node not found")))?;
    Ok(Json(node))
}

pub async fn create_node(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let parent_id = input.parent_id.map(|p| Uuid::parse_str(&p)).transpose()
        .map_err(|_| AppError(anyhow::anyhow!("Invalid parent ID")))?;

    let service = NarrativeService::new(state.pool.clone());
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
    let service = NarrativeService::new(state.pool.clone());
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
    let service = NarrativeService::new(state.pool.clone());
    service.delete_node(id).await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

pub async fn list_storylines(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, Option<String>, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name, description, status, importance, created_at::text, updated_at::text FROM storyline WHERE project_id=$1"
    ).bind(&project_id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, name, desc, st, imp, cr, up)| serde_json::json!({"id": id, "project_id": project_id, "name": name, "description": desc, "status": st, "importance": imp, "created_at": cr, "updated_at": up})).collect::<Vec<_>>())))
}

pub async fn create_storyline(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO storyline (id, project_id, name, description, status, importance) VALUES ($1,$2,$3,$4,'Planned',$5)")
        .bind(&id).bind(&project_id).bind(&input.name).bind(&input.description).bind(input.importance.as_deref().unwrap_or("Normal"))
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "project_id": project_id, "name": input.name, "status": "Planned"})))
}

pub async fn update_storyline(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE storyline SET name=$1, description=$2, updated_at=NOW() WHERE id=$3").bind(&input.name).bind(&input.description).bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "updated": true})))
}

pub async fn list_foreshadows(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, Option<String>, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name, description, status, importance, hint_level, created_at::text, updated_at::text FROM foreshadowing WHERE project_id=$1"
    ).bind(&project_id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, name, desc, st, imp, hint, cr, up)| serde_json::json!({"id": id, "project_id": project_id, "name": name, "description": desc, "status": st, "importance": imp, "hint_level": hint, "related_entity_ids": [], "created_at": cr, "updated_at": up})).collect::<Vec<_>>())))
}

pub async fn create_foreshadow(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO foreshadowing (id, project_id, name, description, status, importance, hint_level) VALUES ($1,$2,$3,$4,'Planned',$5,$6)")
        .bind(&id).bind(&project_id).bind(&input.name).bind(&input.description).bind(input.importance.as_deref().unwrap_or("Normal")).bind(input.hint_level.as_deref().unwrap_or("Direct"))
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "project_id": project_id, "name": input.name, "status": "Planned"})))
}

pub async fn update_foreshadow(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE foreshadowing SET name=$1, description=$2, updated_at=NOW() WHERE id=$3").bind(&input.name).bind(&input.description).bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "updated": true})))
}
