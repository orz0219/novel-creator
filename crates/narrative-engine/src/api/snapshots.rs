//! Snapshots API handlers (novel_state_snapshot table)

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;

#[derive(Deserialize)]
pub struct CreateSnapshotInput {
    pub name: Option<String>,
    pub story_time: Option<String>,
    pub world_summary: Option<String>,
}

pub async fn list_snapshots(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, Option<i32>, Option<String>, String)> = sqlx::query_as(
        "SELECT id, scene_id, story_time, world_summary, main_character_state, current_location, active_threads_count, unresolved_foreshadows_count, known_characters_count, known_locations_count, state_data::text, created_at::text FROM novel_state_snapshot WHERE project_id = $1 ORDER BY created_at DESC"
    ).bind(&project_id).fetch_all(&state.pool).await?;

    Ok(Json(serde_json::json!(rows.into_iter().map(|(id, _scene, story_time, summary, _char_state, location, threads, foreshadows, chars, locs, state_data, created)| {
        let state_json: serde_json::Value = serde_json::from_str(&state_data.unwrap_or_else(|| "{}".to_string())).unwrap_or_default();
        let name = state_json.get("name").and_then(|v| v.as_str()).unwrap_or("快照");
        let progress = state_json.get("progress").and_then(|v| v.as_str()).unwrap_or("");
        serde_json::json!({
            "id": id,
            "name": name,
            "story_time": story_time.unwrap_or_default(),
            "world_summary": summary.unwrap_or_default(),
            "current_location": location.unwrap_or_default(),
            "active_threads_count": threads.unwrap_or(0),
            "unresolved_foreshadows_count": foreshadows.unwrap_or(0),
            "known_characters_count": chars.unwrap_or(0),
            "known_locations_count": locs.unwrap_or(0),
            "progress": progress,
            "created_at": created,
        })
    }).collect::<Vec<_>>())))
}

pub async fn create_snapshot(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateSnapshotInput>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::new_v4().to_string();
    let state_data = serde_json::json!({
        "name": input.name.as_deref().unwrap_or("手动快照"),
        "progress": ""
    });

    sqlx::query("INSERT INTO novel_state_snapshot (id, project_id, story_time, world_summary, state_data, active_threads_count, unresolved_foreshadows_count, known_characters_count, known_locations_count) VALUES ($1, $2, $3, $4, $5, 0, 0, 0, 0)")
        .bind(&id).bind(&project_id)
        .bind(input.story_time.as_deref().unwrap_or("now"))
        .bind(input.world_summary.as_deref().unwrap_or(""))
        .bind(state_data)
        .execute(&state.pool).await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "name": input.name.unwrap_or_else(|| "手动快照".to_string()),
        "story_time": input.story_time.unwrap_or_default(),
        "world_summary": input.world_summary.unwrap_or_default(),
        "created_at": chrono::Utc::now().to_rfc3339(),
    })))
}

pub async fn delete_snapshot(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM novel_state_snapshot WHERE id = $1").bind(&id).execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
