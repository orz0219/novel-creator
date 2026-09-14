//! History / Event / Fact / Version API handlers
//!
//! event / fact 读写通过 application::history_service::HistoryService（依赖
//! HistoryRepositoryPort）；version 读 `entity_snapshot`（每次改动前留的快照）。

use axum::extract::{Path, State, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;
use super::error::AppError;
use application::history_service::HistoryService;
use db::application_ports::DbHistoryRepositoryPort;
use db::application_ports::DbEntityRepositoryPort;
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use application::entity_service::EntityService;
use application::mutation::MutationCommitter;
use db::repos::entity_version_repo::EntityVersionRepo;
use std::sync::Arc;

/// 取档案用的服务（角色档案 / 地点档案 / 势力档案都在这上面）。
///
/// 构造方式与 `api/entity.rs` 的 `service()` 保持一致。
fn entity_service(state: &AppState) -> EntityService {
    let committer = Arc::new(MutationCommitter::new(Arc::new(
        DbMutationCommitter::new(state.pool.clone()),
    )));
    let resolver = Arc::new(DbProjectResolverPort::new(state.pool.clone()));
    EntityService::new(
        Arc::new(DbEntityRepositoryPort::new(state.pool.clone())),
        committer,
        resolver,
        "system",
    )
}

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
    // 缺失的必填字段**直接报错**：原先用 `unwrap_or("")` 兜底，
    // 于是「没传 name」会静默写出一条空名事件，比报错难查得多。
    let name = require_str(&input, "name")?;
    let desc = require_str(&input, "description")?;
    let attributes = application::history_service::collect_event_attributes(&input)?;
    let event = service(&state)
        .create_event(
            project_id,
            &name,
            &desc,
            input.get("event_type").and_then(|v| v.as_str()),
            input.get("when").and_then(|v| v.as_str()),
            input.get("duration").and_then(|v| v.as_str()),
            &attributes,
            // era_order：HTTP 侧暂不暴露（结构化字段由 AI 走工具写）
            None,
        )
        .await?;
    Ok(Json(event))
}

/// 取必填字符串字段；缺失 / null / 空串一律报错。
fn require_str(input: &serde_json::Value, key: &str) -> Result<String, AppError> {
    input
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError(anyhow::anyhow!("{} 缺失或为空", key)))
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

/// 某实体的版本时间线。
///
/// 数据来自 `entity_snapshot`（每次改动前留的档）+ `entity` 表的当前行。
/// 这三条接口此前是**占位桩**：不查库、永远返回一条写死的 "Initial"
/// （版本号恒为 1、changes 为空、时间还是请求时刻），电脑端看到的历史是编的。
pub async fn list_versions(State(state): State<AppState>, Path(entity_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&entity_id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let entries = EntityVersionRepo::list_entries(&state.pool, id).await?;
    Ok(Json(serde_json::json!(entries)))
}

pub async fn get_version(State(state): State<AppState>, Path((entity_id, version)): Path<(String, i32)>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&entity_id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    let entry = EntityVersionRepo::get_entry(&state.pool, id, version).await?;
    match entry {
        Some(v) => Ok(Json(v)),
        None => Err(AppError::with_status(
            axum::http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("实体 {} 没有第 {} 版", entity_id, version),
        )),
    }
}

/// 档案类改动的时间线（角色档案 / 当前状态 / 地点档案 / 势力档案）。
///
/// 与 `list_versions` 分开：那条走 `entity_snapshot`（实体行的字段），
/// 这条走 `entity_profile_snapshot`（整份档案 JSON）。两者数据结构不同，
/// 混在一个响应里会把电脑端既有的 `VersionEntry` 契约弄坏。
///
/// 差异计算需要「改后」那一份：最后一条快照没有下一条可比，
/// 所以这里按 kind 取一次当前档案补上。
pub async fn profile_history(State(state): State<AppState>, Path(entity_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&entity_id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;

    // 先看这个实体有没有档案快照，据此决定取哪种「当前档案」
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT kind FROM entity_profile_snapshot WHERE entity_id = $1 ORDER BY replaced_at DESC LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError(anyhow::anyhow!("读取档案历史失败: {}", e)))?;

    let current = match kind.as_deref() {
        Some("character") => {
            let svc = entity_service(&state);
            svc.get_character_profile(id).await?
        }
        Some("location") => {
            let svc = entity_service(&state);
            svc.get_location_profile(id).await?
        }
        Some("faction") => {
            let svc = entity_service(&state);
            svc.get_faction_profile(id).await?
        }
        _ => None,
    };

    let entries = db::repos::entity_version_repo::list_profile_entries(&state.pool, id, current).await?;
    Ok(Json(serde_json::json!(entries)))
}

/// 比较两个版本，返回 `{ 字段: { old, new } }`（电脑端 VersionDiff 按这个渲染）。
pub async fn compare_versions(State(state): State<AppState>, Path(entity_id): Path<String>, Query(q): Query<CompareQuery>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&entity_id).map_err(|_| AppError(anyhow::anyhow!("Invalid entity ID")))?;
    match EntityVersionRepo::compare(&state.pool, id, q.from, q.to).await? {
        Some(diff) => Ok(Json(diff)),
        None => Err(AppError::with_status(
            axum::http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("实体 {} 缺少第 {} 版或第 {} 版", entity_id, q.from, q.to),
        )),
    }
}
