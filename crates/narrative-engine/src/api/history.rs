//! History / Event / Fact / Version API handlers
use axum::extract::{Path, State, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct LimitQuery { pub limit: Option<i64> }

#[derive(Deserialize)]
pub struct CompareQuery { pub from: i32, pub to: i32 }

pub async fn list_events(State(state): State<AppState>, Path(project_id): Path<String>, Query(q): Query<LimitQuery>) -> Result<Json<serde_json::Value>, AppError> {
    let limit = q.limit.unwrap_or(50);
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String)>(
        "SELECT id, name, description, event_type, timestamp, created_at::text FROM event WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2"
    ).bind(&project_id).bind(limit).fetch_all(&state.pool).await?;

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, name, desc, etype, ts, created)| {
        serde_json::json!({"id": id, "name": name, "description": desc, "event_type": etype, "timestamp": ts, "created_at": created})
    }).collect::<Vec<_>>())))
}

pub async fn create_event(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let desc = input.get("description").and_then(|v| v.as_str()).unwrap_or("");
    sqlx::query("INSERT INTO event (id, project_id, name, description, involved_entity_ids, state_changes) VALUES ($1, $2, $3, $4, '{}', '[]')")
        .bind(&id).bind(&project_id).bind(name).bind(desc).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "name": name, "description": desc})))
}

pub async fn list_facts(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
        "SELECT id, content, category, certainty, created_at::text FROM fact WHERE project_id = $1 ORDER BY created_at DESC"
    ).bind(&project_id).fetch_all(&state.pool).await?;

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, content, cat, cert, created)| {
        serde_json::json!({"id": id, "project_id": project_id, "content": content, "category": cat, "certainty": cert, "created_at": created, "updated_at": created})
    }).collect::<Vec<_>>())))
}

pub async fn create_fact(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let category = input.get("category").and_then(|v| v.as_str());
    let certainty = input.get("certainty").and_then(|v| v.as_str()).unwrap_or("CANON");
    sqlx::query("INSERT INTO fact (id, project_id, content, category, certainty) VALUES ($1, $2, $3, $4, $5)")
        .bind(&id).bind(&project_id).bind(content).bind(category).bind(certainty).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"id": id, "project_id": project_id, "content": content, "certainty": certainty})))
}

pub async fn list_versions(State(_state): State<AppState>, Path(entity_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    // Version history - stub for now, will use entity version field
    Ok(Json(serde_json::json!([{"id": format!("v-{}", entity_id), "entity_id": entity_id, "version": 1, "description": "Initial", "actor": "user", "created_at": chrono::Utc::now().to_rfc3339(), "changes": {}}])))
}

pub async fn get_version(State(_state): State<AppState>, Path((entity_id, version)): Path<(String, i32)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"id": format!("v-{}-{}", entity_id, version), "entity_id": entity_id, "version": version, "description": "Version snapshot", "actor": "user", "created_at": chrono::Utc::now().to_rfc3339(), "changes": {}})))
}

pub async fn compare_versions(State(_state): State<AppState>, Path(entity_id): Path<String>, Query(q): Query<CompareQuery>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"entity_id": entity_id, "from": q.from, "to": q.to, "diff": {}})))
}
