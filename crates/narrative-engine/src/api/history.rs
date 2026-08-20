//! History / Event / Fact / Version API handlers
//!
//! event / fact 读写通过 application::history_service::HistoryService（依赖
//! HistoryRepositoryPort）；version 仍为占位 stub。

use axum::extract::{Path, State, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::history_service::HistoryService;
use db::application_ports::DbHistoryRepositoryPort;
use std::sync::Arc;

fn service(state: &AppState) -> HistoryService {
    HistoryService::new(Arc::new(DbHistoryRepositoryPort::new(state.pool.clone())))
}

#[derive(Deserialize)]
pub struct LimitQuery { pub limit: Option<i64> }

#[derive(Deserialize)]
pub struct CompareQuery { pub from: i32, pub to: i32 }

pub async fn list_events(State(state): State<AppState>, Path(project_id): Path<String>, Query(q): Query<LimitQuery>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let limit = q.limit.unwrap_or(50);
    let events = service(&state).list_events(project_id, limit).await?;
    Ok(Json(serde_json::json!(events)))
}

pub async fn create_event(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let desc = input.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let event = service(&state).create_event(project_id, name, desc).await?;
    Ok(Json(event))
}

pub async fn list_facts(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let facts = service(&state).list_facts(project_id).await?;
    Ok(Json(serde_json::json!(facts)))
}

pub async fn create_fact(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let category = input.get("category").and_then(|v| v.as_str());
    let certainty = input.get("certainty").and_then(|v| v.as_str()).unwrap_or("CANON");
    let fact = service(&state).create_fact(project_id, content, category, certainty).await?;
    Ok(Json(fact))
}

pub async fn list_versions(State(_state): State<AppState>, Path(entity_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!([{"id": format!("v-{}", entity_id), "entity_id": entity_id, "version": 1, "description": "Initial", "actor": "user", "created_at": chrono::Utc::now().to_rfc3339(), "changes": {}}])))
}

pub async fn get_version(State(_state): State<AppState>, Path((entity_id, version)): Path<(String, i32)>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"id": format!("v-{}-{}", entity_id, version), "entity_id": entity_id, "version": version, "description": "Version snapshot", "actor": "user", "created_at": chrono::Utc::now().to_rfc3339(), "changes": {}})))
}

pub async fn compare_versions(State(_state): State<AppState>, Path(entity_id): Path<String>, Query(q): Query<CompareQuery>) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(serde_json::json!({"entity_id": entity_id, "from": q.from, "to": q.to, "diff": {}})))
}
