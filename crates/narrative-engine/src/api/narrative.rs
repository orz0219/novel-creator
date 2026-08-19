//! Narrative API handlers

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct CreateNodeInput { pub node_type: String, pub parent_id: Option<String>, pub title: String, pub description: Option<String>, pub attributes: Option<serde_json::Value> }
#[derive(Deserialize)]
pub struct UpdateNodeInput { pub title: Option<String>, pub description: Option<String>, pub status: Option<String> }
#[derive(Deserialize)]
pub struct CreateStorylineInput { pub name: String, pub description: Option<String>, pub importance: Option<String> }
#[derive(Deserialize)]
pub struct CreateForeshadowInput { pub name: String, pub description: Option<String>, pub importance: Option<String>, pub hint_level: Option<String> }

fn pj(s: &str) -> serde_json::Value { serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({})) }

pub async fn list_nodes(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, String, String, Option<String>, String, Option<String>, String, i32, String, String, String)> = sqlx::query_as(
        "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes::text, sort_order, status, created_at::text, updated_at::text FROM narrative_node WHERE project_id = $1 AND status != 'Deleted' ORDER BY sort_order"
    ).bind(&project_id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, pid, wid, nt, par, title, desc, attrs, ord, st, cr, up)| {
        serde_json::json!({"id": id, "project_id": pid, "world_id": wid, "node_type": nt, "parent_id": par, "title": title, "description": desc, "attributes": pj(&attrs), "sort_order": ord, "status": st, "created_at": cr, "updated_at": up})
    }).collect::<Vec<_>>())))
}

pub async fn get_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row: Option<(String, String, String, String, Option<String>, String, Option<String>, String, i32, String, String, String)> = sqlx::query_as(
        "SELECT id, project_id, world_id, node_type, parent_id, title, description, attributes::text, sort_order, status, created_at::text, updated_at::text FROM narrative_node WHERE id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;
    match row {
        Some((id, pid, wid, nt, par, title, desc, attrs, ord, st, cr, up)) =>
            Ok(Json(serde_json::json!({"id": id, "project_id": pid, "world_id": wid, "node_type": nt, "parent_id": par, "title": title, "description": desc, "attributes": pj(&attrs), "sort_order": ord, "status": st, "created_at": cr, "updated_at": up}))),
        None => Err(AppError(anyhow::anyhow!("Narrative node not found")))
    }
}

pub async fn create_node(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let world_id: (String,) = sqlx::query_as("SELECT id FROM world WHERE project_id = $1 LIMIT 1").bind(&project_id).fetch_one(&state.pool).await?;
    let sort_order: (i32,) = sqlx::query_as("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM narrative_node WHERE project_id = $1 AND parent_id IS NOT DISTINCT FROM $2").bind(&project_id).bind(&input.parent_id).fetch_one(&state.pool).await?;
    sqlx::query("INSERT INTO narrative_node (id, project_id, world_id, node_type, parent_id, title, description, attributes, sort_order, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Draft')")
        .bind(&id).bind(&project_id).bind(&world_id.0).bind(&input.node_type).bind(&input.parent_id).bind(&input.title).bind(&input.description).bind(input.attributes.unwrap_or(serde_json::json!({}))).bind(sort_order.0)
        .execute(&state.pool).await?;
    get_node(State(state), Path(id)).await
}

pub async fn update_node(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateNodeInput>) -> Result<Json<serde_json::Value>, AppError> {
    // Cannot update deleted nodes
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM narrative_node WHERE id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;

    match exists {
        Some((status,)) if status == "Deleted" => {
            return Err(AppError(anyhow::anyhow!("Cannot update deleted narrative node")));
        }
        None => {
            return Err(AppError(anyhow::anyhow!("Narrative node not found")));
        }
        _ => {}
    }

    if let Some(t) = &input.title { sqlx::query("UPDATE narrative_node SET title=$1, updated_at=NOW() WHERE id=$2").bind(t).bind(&id).execute(&state.pool).await?; }
    if let Some(d) = &input.description { sqlx::query("UPDATE narrative_node SET description=$1, updated_at=NOW() WHERE id=$2").bind(d).bind(&id).execute(&state.pool).await?; }
    if let Some(s) = &input.status { sqlx::query("UPDATE narrative_node SET status=$1, updated_at=NOW() WHERE id=$2").bind(s).bind(&id).execute(&state.pool).await?; }
    get_node(State(state), Path(id)).await
}

pub async fn delete_node(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    // Soft delete - set status to Deleted instead of removing
    let result = sqlx::query(
        "UPDATE narrative_node SET status = 'Deleted', updated_at = NOW() WHERE id = $1 AND status != 'Deleted'"
    ).bind(&id).execute(&state.pool).await?;

    if result.rows_affected() == 0 {
        return Err(AppError(anyhow::anyhow!("Narrative node not found or already deleted")));
    }

    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

pub async fn list_storylines(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, Option<String>, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name, description, status, importance, created_at::text, updated_at::text FROM storyline WHERE project_id=$1"
    ).bind(&project_id).fetch_all(&state.pool).await?;
    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, name, desc, st, imp, cr, up)| serde_json::json!({"id": id, "project_id": project_id, "name": name, "description": desc, "status": st, "importance": imp, "created_at": cr, "updated_at": up})).collect::<Vec<_>>())))
}

pub async fn create_storyline(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateStorylineInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
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
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO foreshadowing (id, project_id, name, description, status, importance, hint_level) VALUES ($1,$2,$3,$4,'Planned',$5,$6)")
        .bind(&id).bind(&project_id).bind(&input.name).bind(&input.description).bind(input.importance.as_deref().unwrap_or("Normal")).bind(input.hint_level.as_deref().unwrap_or("Direct"))
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "project_id": project_id, "name": input.name, "status": "Planned"})))
}

pub async fn update_foreshadow(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<CreateForeshadowInput>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE foreshadowing SET name=$1, description=$2, updated_at=NOW() WHERE id=$3").bind(&input.name).bind(&input.description).bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "updated": true})))
}
