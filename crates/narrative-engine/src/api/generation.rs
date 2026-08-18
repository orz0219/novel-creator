//! Generation API handlers
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct CreateGenerationInput { pub r#type: String, pub target_id: Option<String>, pub model: Option<String>, pub parameters: Option<serde_json::Value> }

pub async fn list_tasks(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<i32>, String)>(
        "SELECT id, task_type, status, COALESCE(model, ''), target_id, result::text, context_tokens, created_at::text FROM generation_task WHERE project_id = $1 ORDER BY created_at DESC"
    ).bind(&project_id).fetch_all(&state.pool).await?;

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, ttype, status, model, target, result, tokens, created)| {
        serde_json::json!({"id": id, "type": ttype, "status": status, "model": model, "target_id": target, "result": result, "context_tokens": tokens, "parameters": {}, "created_at": created, "updated_at": created})
    }).collect::<Vec<_>>())))
}

pub async fn get_task(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<i32>, String)>(
        "SELECT id, task_type, status, COALESCE(model, ''), target_id, result::text, context_tokens, created_at::text FROM generation_task WHERE id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;

    match row {
        Some(r) => Ok(Json(serde_json::json!({"id": r.0, "type": r.1, "status": r.2, "model": r.3, "target_id": r.4, "result": r.5, "context_tokens": r.6, "parameters": {}, "created_at": r.7, "updated_at": r.7}))),
        None => Err(AppError(anyhow::anyhow!("Generation task not found")))
    }
}

pub async fn create_task(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateGenerationInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO generation_task (id, project_id, task_type, target_id, model, parameters, status) VALUES ($1, $2, $3, $4, $5, $6, 'Pending')")
        .bind(&id).bind(&project_id).bind(&input.r#type).bind(&input.target_id).bind(&input.model).bind(input.parameters.unwrap_or(serde_json::json!({})))
        .execute(&state.pool).await?;
    get_task(State(state), Path(id)).await
}

pub async fn cancel_task(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE generation_task SET status = 'Cancelled' WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "status": "Cancelled"})))
}
