//! Project API handlers

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CreateProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub world_setting: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

pub async fn list_projects(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let projects = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, String, String, String, String)>(
        "SELECT id, name, description, language, status, config::text, created_at::text, updated_at::text FROM project ORDER BY updated_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    let result: Vec<serde_json::Value> = projects.into_iter().map(|(id, name, desc, lang, status, config, created, updated)| {
        serde_json::json!({
            "id": id, "name": name, "description": desc, "language": lang,
            "status": status, "config": serde_json::from_str::<serde_json::Value>(&config).unwrap_or_default(),
            "default_params": {}, "created_at": created, "updated_at": updated
        })
    }).collect();

    Ok(Json(serde_json::json!(result)))
}

pub async fn get_project(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, String, String, String, String)>(
        "SELECT id, name, description, language, status, config::text, created_at::text, updated_at::text FROM project WHERE id = $1"
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;

    match row {
        Some((id, name, desc, lang, status, config, created, updated)) => {
            Ok(Json(serde_json::json!({
                "id": id, "name": name, "description": desc, "language": lang,
                "status": status, "config": serde_json::from_str::<serde_json::Value>(&config).unwrap_or_default(),
                "default_params": {}, "created_at": created, "updated_at": updated
            })))
        }
        None => Err(AppError(anyhow::anyhow!("Project not found")))
    }
}

pub async fn create_project(State(state): State<AppState>, Json(input): Json<CreateProjectInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO project (id, name, description, language, status, config) VALUES ($1, $2, $3, $4, 'Concept', '{}')")
        .bind(&id).bind(&input.name).bind(&input.description).bind(&input.language)
        .execute(&state.pool).await?;

    get_project(State(state), Path(id)).await
}

pub async fn update_project(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateProjectInput>) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(name) = &input.name {
        sqlx::query("UPDATE project SET name = $1, updated_at = NOW() WHERE id = $2").bind(name).bind(&id).execute(&state.pool).await?;
    }
    if let Some(desc) = &input.description {
        sqlx::query("UPDATE project SET description = $1, updated_at = NOW() WHERE id = $2").bind(desc).bind(&id).execute(&state.pool).await?;
    }
    if let Some(status) = &input.status {
        sqlx::query("UPDATE project SET status = $1, updated_at = NOW() WHERE id = $2").bind(status).bind(&id).execute(&state.pool).await?;
    }
    get_project(State(state), Path(id)).await
}

pub async fn delete_project(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM project WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}