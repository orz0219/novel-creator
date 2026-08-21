//! Snapshots API handlers (novel_state_snapshot table)
//!
//! 通过 application::snapshot_service::SnapshotService（依赖 SnapshotRepositoryPort）。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::snapshot_service::SnapshotService;
use db::application_ports::{DbNarrativeStateWritePort, DbSnapshotRepositoryPort};
use std::sync::Arc;

fn service(state: &AppState) -> SnapshotService {
    SnapshotService::new(
        Arc::new(DbSnapshotRepositoryPort::new(state.pool.clone())),
        Arc::new(DbNarrativeStateWritePort::new(state.pool.clone())),
    )
}

#[derive(Deserialize)]
pub struct CreateSnapshotInput {
    pub name: Option<String>,
    pub story_time: Option<String>,
    pub world_summary: Option<String>,
}

pub async fn list_snapshots(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let snapshots = service(&state).list_snapshots(project_id).await?;
    Ok(Json(serde_json::json!(snapshots)))
}

pub async fn create_snapshot(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateSnapshotInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let snapshot = service(&state)
        .create_snapshot(
            project_id,
            input.name.as_deref(),
            input.story_time.as_deref(),
            input.world_summary.as_deref(),
        )
        .await?;
    Ok(Json(snapshot))
}

pub async fn delete_snapshot(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid snapshot ID")))?;
    service(&state).delete_snapshot(id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}

/// 恢复快照：把快照的宏观状态回写到 narrative_state（World 维度），幂等。
pub async fn restore_snapshot(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid snapshot ID")))?;
    let result = service(&state).restore_snapshot(id).await.map_err(|e| {
        if e.to_string().contains("Snapshot not found") {
            AppError::with_status(axum::http::StatusCode::NOT_FOUND, e)
        } else {
            AppError(e)
        }
    })?;
    Ok(Json(result))
}
