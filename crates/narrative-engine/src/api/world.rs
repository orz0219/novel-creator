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
        None => {
            // Auto-create world for existing projects that don't have one
            let project_name: Option<(String,)> = sqlx::query_as("SELECT name FROM project WHERE id = $1")
                .bind(&project_id).fetch_optional(&state.pool).await?;
            if let Some((name,)) = project_name {
                let world_id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO world (id, project_id, name, description, config, is_main) VALUES ($1, $2, $3, $4, '{}', true)")
                    .bind(&world_id).bind(&project_id).bind(&name).bind::<Option<String>>(None)
                    .execute(&state.pool).await?;
                // Return the newly created world
                let new_row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String, String)>(
                    "SELECT id, project_id, name, description, world_rules, created_at::text, updated_at::text FROM world WHERE id = $1"
                ).bind(&world_id).fetch_one(&state.pool).await?;
                let (id, pid, name, desc, rules, created, updated) = new_row;
                Ok(Json(serde_json::json!({"id": id, "project_id": pid, "name": name, "description": desc, "world_rules": rules, "config": {}, "is_main": true, "created_at": created, "updated_at": updated})))
            } else {
                Err(AppError(anyhow::anyhow!("Project not found")))
            }
        }
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
