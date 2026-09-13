//! HTTP API Layer - Axum routes and handlers
//!
//! Bridges frontend API calls to backend application services.

pub mod project;
pub mod world;
pub mod entity;
pub mod narrative;
pub mod context;
pub mod generation;
pub mod extraction;
pub mod proposal;
pub mod validation;
pub mod history;
pub mod rules;
pub mod snapshots;
pub mod trace;
pub mod settings;
pub mod agent;
pub mod error;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Router, routing::{get, post, put, delete}};
use tower_http::cors::{CorsLayer, Any};
use crate::state::AppState;

/// Build the API router with all routes
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Projects
        .route("/api/v1/projects", get(project::list_projects).post(project::create_project))
        .route("/api/v1/projects/{id}", get(project::get_project).put(project::update_project).delete(project::delete_project))
        // World
        .route("/api/v1/projects/{id}/world", get(world::get_world).put(world::update_world))
        // Entities
        .route("/api/v1/worlds/{id}/entities", get(entity::list_entities).post(entity::create_entity))
        .route("/api/v1/entities/{id}", get(entity::get_entity).put(entity::update_entity).delete(entity::delete_entity))
        // Characters (specialized entity)
        .route("/api/v1/worlds/{id}/characters", get(entity::list_characters).post(entity::create_character))
        .route("/api/v1/characters/{id}", get(entity::get_character).put(entity::update_entity).delete(entity::delete_entity))
        .route("/api/v1/characters/{id}/profile", get(entity::get_character_profile).put(entity::update_character_profile))
        .route("/api/v1/characters/{id}/state", get(entity::get_character_state).put(entity::update_character_state))
        .route("/api/v1/characters/{id}/knowledge", get(entity::get_character_knowledge))
        .route("/api/v1/characters/{id}/relationships", get(entity::get_character_relationships))
        // Locations (specialized entity)
        .route("/api/v1/worlds/{id}/locations", get(entity::list_locations).post(entity::create_location))
        .route("/api/v1/locations/{id}", get(entity::get_location).put(entity::update_entity).delete(entity::delete_entity))
        .route("/api/v1/locations/{id}/profile", get(entity::get_location_profile).put(entity::upsert_location_profile))
        .route("/api/v1/locations/{id}/entities", get(entity::get_location_entities))
        .route("/api/v1/locations/{id}/events", get(entity::get_location_events))
        // Factions (specialized entity)
        .route("/api/v1/worlds/{id}/factions", get(entity::list_factions).post(entity::create_faction))
        .route("/api/v1/factions/{id}", get(entity::get_entity).put(entity::update_entity).delete(entity::delete_entity))
        .route("/api/v1/factions/{id}/profile", get(entity::get_faction_profile).put(entity::upsert_faction_profile))
        // Relations
        .route("/api/v1/worlds/{id}/relations", get(entity::list_relations).post(entity::create_relation))
        .route("/api/v1/relations/{id}", delete(entity::delete_relation))
        // Events
        .route("/api/v1/projects/{id}/events", get(history::list_events).post(history::create_event))
        // Facts
        .route("/api/v1/projects/{id}/facts", get(history::list_facts).post(history::create_fact))
        // Narrative
        .route("/api/v1/projects/{id}/narrative", get(narrative::list_nodes).post(narrative::create_node))
        .route("/api/v1/narrative/{id}", get(narrative::get_node).put(narrative::update_node).delete(narrative::delete_node))
        // Storylines
        .route("/api/v1/projects/{id}/storylines", get(narrative::list_storylines).post(narrative::create_storyline))
        .route("/api/v1/projects/{id}/storyline-relations", get(narrative::list_storyline_relations))
        .route("/api/v1/storylines/{id}", put(narrative::update_storyline).delete(narrative::delete_storyline))
        // Foreshadows
        .route("/api/v1/projects/{id}/foreshadows", get(narrative::list_foreshadows).post(narrative::create_foreshadow))
        .route("/api/v1/foreshadows/{id}", put(narrative::update_foreshadow).delete(narrative::delete_foreshadow))
        // Context
        .route("/api/v1/scenes/{id}/context", get(context::get_context))
        .route("/api/v1/scenes/{id}/context/build", post(context::build_context))
        .route("/api/v1/scenes/{id}/context/pin/{entity_id}", post(context::pin_entity).delete(context::unpin_entity))
        .route("/api/v1/scenes/{id}/context/exclude/{entity_id}", post(context::exclude_entity).delete(context::unexclude_entity))
        // Generation
        .route("/api/v1/projects/{id}/generations", get(generation::list_tasks).post(generation::create_task))
        .route("/api/v1/generations/{id}", get(generation::get_task))
        .route("/api/v1/generations/{id}/cancel", post(generation::cancel_task))
        .route("/api/v1/generations/{id}/execute", post(generation::execute_task))
        // Extraction (M1: 文本 → 实体/关系抽取闭环)
        .route("/api/v1/projects/{id}/extract", post(extraction::extract_text))
        // Proposals
        .route("/api/v1/projects/{id}/proposals", get(proposal::list_proposals))
        .route("/api/v1/proposals/{id}", get(proposal::get_proposal))
        .route("/api/v1/proposals/{id}/accept", post(proposal::accept_proposal))
        .route("/api/v1/proposals/{id}/reject", post(proposal::reject_proposal))
        .route("/api/v1/proposals/{id}/changes/{change_id}/accept", post(proposal::accept_change))
        .route("/api/v1/proposals/{id}/changes/{change_id}/reject", post(proposal::reject_change))
        // Validation
        .route("/api/v1/scenes/{id}/validate", post(validation::validate_scene))
        .route("/api/v1/proposals/{id}/validate", post(validation::validate_proposal))
        .route("/api/v1/worlds/{id}/validate", post(validation::validate_world))
        // Versions / History
        .route("/api/v1/entities/{id}/versions", get(history::list_versions))
        .route("/api/v1/entities/{id}/versions/{version}", get(history::get_version))
        .route("/api/v1/entities/{id}/versions/compare", get(history::compare_versions))
        // 档案（角色档案 / 当前状态 / 地点档案 / 势力档案）的改动历史
        .route("/api/v1/entities/{id}/profile-history", get(history::profile_history))
        // Rules (canon_rule)
        .route("/api/v1/worlds/{id}/rules", get(rules::list_rules).post(rules::create_rule))
        .route("/api/v1/rules/{id}", get(rules::get_rule).put(rules::update_rule).delete(rules::delete_rule))
        // Snapshots
        .route("/api/v1/projects/{id}/snapshots", get(snapshots::list_snapshots).post(snapshots::create_snapshot))
        .route("/api/v1/snapshots/{id}", delete(snapshots::delete_snapshot))
        .route("/api/v1/snapshots/{id}/restore", post(snapshots::restore_snapshot))
        // AI 可追溯
        .route("/api/v1/projects/{id}/generation-runs", get(trace::list_generation_runs))
        .route("/api/v1/projects/{id}/validation-runs", get(trace::list_validation_runs))
        // Settings (global)
        .route("/api/v1/settings", get(settings::get_settings).put(settings::update_settings))
        .route("/api/v1/settings/test-connection", post(settings::test_connection))
        .route("/api/v1/settings/models", post(settings::list_models))
        .route("/api/v1/settings/model-catalog", get(settings::model_catalog))
        // Health
        .route("/api/v1/health", get(health_check))
        // 接收手机备份（手机 → 电脑，局域网内使用）
        .route("/api/v1/backup/receive", post(receive_backup))
        // 项目数据导出（供手机单机版导入）
        .route("/api/v1/projects/{id}/export", get(export_project))
        // Agent（P1 引导式 Agent 框架）：会话 / 工具 / SSE 聊天
        .route("/api/v1/agent/session", post(agent::create_session))
        .route("/api/v1/agent/session/{id}", get(agent::get_session).delete(agent::delete_session).put(agent::rename_session))
        .route("/api/v1/agent/session/{id}/context", get(agent::context_usage))
        .route("/api/v1/agent/session/{id}/truncate", post(agent::truncate_session))
        .route("/api/v1/agent/sessions", get(agent::list_sessions))
        .route("/api/v1/agent/tools", get(agent::list_tools))
        .route("/api/v1/agent/chat", post(agent::chat))
        .route("/api/v1/agent/tool/execute", post(agent::execute_tool))
        .route("/api/v1/agent/prompt", get(agent::get_prompt).put(agent::save_prompt).delete(agent::delete_prompt))
        // 引导推进（用户点按钮触发，agent 不应主动调）
        .route("/api/v1/agent/guide/confirm", post(agent::confirm_guide_step))
        .with_state(state)
        .layer(cors)
}

/// `POST /api/v1/backup/receive` —— 接收手机上传的一个项目备份，存到磁盘。
///
/// 手机端没有可靠的方式把文件写到用户可见的目录（见实施记录），
/// 所以采用「手机主动传到电脑」这条路：不依赖任何系统权限。
///
/// 安全说明：后端只监听局域网/本机，且这里只把数据落盘、不覆盖电脑上的
/// 任何项目，因此不做鉴权；但会校验请求体是合法的导出 JSON，
/// 并限制表名，避免异常内容被当成数据写下去。
async fn receive_backup(
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<axum::Json<serde_json::Value>, crate::api::error::AppError> {
    // 大小上限 64MB：个人项目远小于此，超了说明不是正常备份
    if body.len() > 64 * 1024 * 1024 {
        return Err(anyhow::anyhow!("备份内容过大（{} bytes）", body.len()).into());
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| anyhow::anyhow!("不是合法 JSON: {}", e))?;

    // 必须是导出格式：有 tables 与 root_project_id
    let tables = parsed
        .get("tables")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("缺少 tables 字段，不是导出文件"))?;
    let root_id = parsed
        .get("root_project_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 root_project_id，不是导出文件"))?;

    // 表名做白名单式校验：只允许字母数字与下划线，防止拼出奇怪路径
    for name in tables.keys() {
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(anyhow::anyhow!("表名不合法: {}", name).into());
        }
    }

    let project_name = parsed
        .get("tables")
        .and_then(|t| t.get("project"))
        .and_then(|p| p.get("rows"))
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .and_then(|row| row.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("未命名项目");

    // 存到仓库根目录下的 tmp/backups/
    //
    // CARGO_MANIFEST_DIR 是 `crates/narrative-engine`，
    // 所以要向上两级才是仓库根（之前只退一级，落到了 crates/tmp/ 这个错误位置）。
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest
        .parent() // crates/
        .and_then(|p| p.parent()) // 仓库根
        .ok_or_else(|| anyhow::anyhow!("无法定位仓库根目录"))?;
    let dir = repo_root.join("tmp").join("backups");
    std::fs::create_dir_all(&dir)
        .map_err(|e| anyhow::anyhow!("创建备份目录失败: {}", e))?;

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let safe_name: String = project_name
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let file = dir.join(format!(
        "{}-{}.json",
        stamp,
        if safe_name.trim().is_empty() { "project" } else { safe_name.trim() }
    ));

    std::fs::write(&file, body.as_bytes())
        .map_err(|e| anyhow::anyhow!("写入备份文件失败: {}", e))?;

    let size = body.len();
    tracing::info!(
        "收到手机备份: {} ({} bytes, 来自 {:?})",
        file.display(),
        size,
        headers.get("x-novel-device").and_then(|v| v.to_str().ok())
    );

    Ok(axum::Json(serde_json::json!({
        "ok": true,
        "saved_to": file.to_string_lossy(),
        "bytes": size,
        "tables": tables.len(),
        "project_id": root_id,
        "project_name": project_name,
    })))
}

/// `GET /api/v1/projects/{id}/export` —— 导出整个项目（含关联数据）为 JSON。
///
/// 手机单机版用这个接口把电脑端的项目搬到手机上。
async fn export_project(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::Json<db::export::ProjectExport>, crate::api::error::AppError> {
    let export = db::export::export_project(&state.pool, &id).await?;
    Ok(axum::Json(export))
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => (StatusCode::OK, "OK").into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("DB unavailable: {e}"),
        )
            .into_response(),
    }
}
