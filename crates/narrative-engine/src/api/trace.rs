//! AI 可追溯 API：generation_run / validation_run 只读视图。
//!
//! 通过 application::trace_service::TraceService（依赖 TraceQueryPort）。

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::trace_service::TraceService;
use db::application_ports::DbTraceQueryPort;
use std::sync::Arc;

fn service(state: &AppState) -> TraceService {
    TraceService::new(Arc::new(DbTraceQueryPort::new(state.pool.clone())))
}

#[derive(Deserialize, Default)]
pub struct TraceQuery {
    pub limit: Option<i64>,
}

/// GET /api/v1/projects/{id}/generation-runs
pub async fn list_generation_runs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let runs = service(&state)
        .list_generation_runs(project_id, q.limit.unwrap_or(50))
        .await?;
    Ok(Json(serde_json::json!(runs)))
}

/// GET /api/v1/projects/{id}/validation-runs
pub async fn list_validation_runs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<TraceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let runs = service(&state)
        .list_validation_runs(project_id, q.limit.unwrap_or(50))
        .await?;
    Ok(Json(serde_json::json!(runs)))
}
