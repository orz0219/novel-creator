//! Rules API handlers (canon_rule table)

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct CreateRuleInput {
    pub rule_content: String,
    pub rule_level: Option<String>,
    pub affected_scope: Option<String>,
    pub enforcement: Option<String>,
    #[allow(dead_code)]
    pub severity: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRuleInput {
    pub rule_content: Option<String>,
    pub rule_level: Option<String>,
    pub severity: Option<String>,
}

pub async fn list_rules(State(state): State<AppState>, Path(world_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, String, String, Option<String>, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, created_at::text, updated_at::text FROM canon_rule WHERE world_id = $1 ORDER BY created_at"
    ).bind(&world_id).fetch_all(&state.pool).await?;

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, pid, wid, level, content, scope, enforce, cr, up)| {
        serde_json::json!({
            "id": id, "project_id": pid, "world_id": wid,
            "rule_level": level, "rule_content": content,
            "affected_scope": scope, "enforcement": enforce,
            "created_at": cr, "updated_at": up
        })
    }).collect::<Vec<_>>())))
}

pub async fn create_rule(State(state): State<AppState>, Path(world_id): Path<String>, Json(input): Json<CreateRuleInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let project_id: (String,) = sqlx::query_as("SELECT project_id FROM world WHERE id = $1")
        .bind(&world_id).fetch_one(&state.pool).await?;

    sqlx::query("INSERT INTO canon_rule (id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(&id).bind(&project_id.0).bind(&world_id)
        .bind(input.rule_level.as_deref().unwrap_or("RULE-2"))
        .bind(&input.rule_content)
        .bind(input.affected_scope.as_deref().unwrap_or("general"))
        .bind(input.enforcement.as_deref().unwrap_or("Allow"))
        .execute(&state.pool).await?;

    get_rule(State(state), Path(id)).await
}

pub async fn get_rule(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let row: Option<(String, String, String, Option<String>, String, Option<String>, String, String, String)> = sqlx::query_as(
        "SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, created_at::text, updated_at::text FROM canon_rule WHERE id = $1"
    ).bind(&id).fetch_optional(&state.pool).await?;

    match row {
        Some((id, pid, wid, level, content, scope, enforce, cr, up)) => {
            Ok(Json(serde_json::json!({
                "id": id, "project_id": pid, "world_id": wid,
                "rule_level": level, "rule_content": content,
                "affected_scope": scope, "enforcement": enforce,
                "created_at": cr, "updated_at": up
            })))
        }
        None => Err(AppError(anyhow::anyhow!("Rule not found")))
    }
}

pub async fn update_rule(State(state): State<AppState>, Path(id): Path<String>, Json(input): Json<UpdateRuleInput>) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(content) = &input.rule_content {
        sqlx::query("UPDATE canon_rule SET rule_content = $1, updated_at = NOW() WHERE id = $2")
            .bind(content).bind(&id).execute(&state.pool).await?;
    }
    if let Some(level) = &input.rule_level {
        sqlx::query("UPDATE canon_rule SET rule_level = $1, updated_at = NOW() WHERE id = $2")
            .bind(level).bind(&id).execute(&state.pool).await?;
    }
    if let Some(severity) = &input.severity {
        sqlx::query("UPDATE canon_rule SET enforcement = $1, updated_at = NOW() WHERE id = $2")
            .bind(severity).bind(&id).execute(&state.pool).await?;
    }
    get_rule(State(state), Path(id)).await
}

pub async fn delete_rule(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM canon_rule WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
