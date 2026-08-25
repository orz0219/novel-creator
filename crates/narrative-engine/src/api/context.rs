//! Context API handlers
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;
use uuid::Uuid;
use crate::state::AppState;
use super::error::AppError;
use runtime::execution::context_engine::{ContextEngine, ContextEngineDeps};
use domain::ports::NarrativeRepositoryPort;
use db::runtime_ports::{
    DbNarrativePort, DbEntityPort, DbStatePort, DbKnowledgePort,
    DbRelationPort, DbEventPort, DbCanonRulePort, DbContextSnapshotPort,
};
use db::application_ports::DbNarrativeRepositoryPort;

/// 默认上下文 Token 预算（无显式预算时使用）。
const DEFAULT_TOKEN_BUDGET: i32 = 4000;

/// 从 scene 节点解析其所属 project_id（ContextEngine 需要 project 作用域）。
async fn resolve_project_id(pool: &sqlx::PgPool, scene_id: Uuid) -> Result<Uuid, AppError> {
    let narr = DbNarrativeRepositoryPort::new(pool.clone());
    let scene = narr.get_node(scene_id).await
        .map_err(|e| AppError(anyhow::anyhow!("查询场景失败: {}", e)))?
        .ok_or_else(|| AppError(anyhow::anyhow!("场景不存在: {}", scene_id)))?;
    let pid = scene.get("project_id").and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError(anyhow::anyhow!("场景缺少有效的 project_id: {}", scene_id)))?;
    Ok(pid)
}

/// 用数据库连接构造 ContextEngine（组合根：注入 8 个运行时仓储端口）。
fn build_engine(pool: &sqlx::PgPool) -> ContextEngine {
    let deps = ContextEngineDeps {
        narrative: Arc::new(DbNarrativePort::new(pool.clone())),
        entity: Arc::new(DbEntityPort::new(pool.clone())),
        state: Arc::new(DbStatePort::new(pool.clone())),
        knowledge: Arc::new(DbKnowledgePort::new(pool.clone())),
        relation: Arc::new(DbRelationPort::new(pool.clone())),
        event: Arc::new(DbEventPort::new(pool.clone())),
        canon: Arc::new(DbCanonRulePort::new(pool.clone())),
        snapshot: Arc::new(DbContextSnapshotPort::new(pool.clone())),
    };
    ContextEngine::new(deps)
}

pub async fn get_context(State(state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let scene_id = Uuid::parse_str(&scene_id).map_err(|_| AppError(anyhow::anyhow!("Invalid scene ID")))?;
    let project_id = resolve_project_id(&state.pool, scene_id).await?;
    let engine = build_engine(&state.pool);
    let pkg = engine.build_context(project_id, scene_id, DEFAULT_TOKEN_BUDGET, None).await?;
    Ok(Json(serde_json::to_value(&pkg).map_err(|e| AppError(anyhow::anyhow!("序列化上下文失败: {}", e)))?))
}

pub async fn build_context(State(state): State<AppState>, Path(scene_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    // 与 get_context 同语义（POST 触发显式重建）。
    let scene_id = Uuid::parse_str(&scene_id).map_err(|_| AppError(anyhow::anyhow!("Invalid scene ID")))?;
    let project_id = resolve_project_id(&state.pool, scene_id).await?;
    let engine = build_engine(&state.pool);
    let pkg = engine.build_context(project_id, scene_id, DEFAULT_TOKEN_BUDGET, None).await?;
    Ok(Json(serde_json::to_value(&pkg).map_err(|e| AppError(anyhow::anyhow!("序列化上下文失败: {}", e)))?))
}

// 以下 pin/unpin/exclude/unexclude 在持久层尚无实现（context 引擎未接线）。
// 按"错误显式暴露"原则，返回 501 Not Implemented，而非假成功 JSON 误导前端。

pub async fn pin_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("pin_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}

pub async fn unpin_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("unpin_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}

pub async fn exclude_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("exclude_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}

pub async fn unexclude_entity(State(_state): State<AppState>, Path((scene_id, entity_id)): Path<(String, String)>) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::with_status(
        StatusCode::NOT_IMPLEMENTED,
        anyhow::anyhow!("unexclude_entity not implemented: scene {} entity {}", scene_id, entity_id),
    ))
}
