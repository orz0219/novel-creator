//! World API handlers
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct UpdateWorldInput { pub name: Option<String>, pub description: Option<String>, pub world_rules: Option<String> }

pub async fn get_world(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String, String)>(
        "SELECT id, project_id, name, description, world_rules, created_at::text, updated_at::text FROM world WHERE project_id = $1 AND is_main = true LIMIT 1"
    ).bind(&project_id).fetch_optional(&state.pool).await?;

    match row {
        Some((id, pid, name, desc, rules, created, updated)) => {
            Ok(Json(serde_json::json!({"id": id, "project_id": pid, "name": name, "description": desc, "world_rules": rules, "config": {}, "is_main": true, "created_at": created, "updated_at": updated})))
        }
        None => Err(AppError(anyhow::anyhow!("World not found")))
    }
}

pub async fn update_world(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<UpdateWorldInput>) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(name) = &input.name {
        sqlx::query("UPDATE world SET name = $1, updated_at = NOW() WHERE project_id = $2 AND is_main = true").bind(name).bind(&project_id).execute(&state.pool).await?;
    }
    if let Some(desc) = &input.description {
        sqlx::query("UPDATE world SET description = $1, updated_at = NOW() WHERE project_id = $2 AND is_main = true").bind(desc).bind(&project_id).execute(&state.pool).await?;
    }
    if let Some(rules) = &input.world_rules {
        sqlx::query("UPDATE world SET world_rules = $1, updated_at = NOW() WHERE project_id = $2 AND is_main = true").bind(rules).bind(&project_id).execute(&state.pool).await?;
    }
    get_world(State(state), Path(project_id)).await
}
