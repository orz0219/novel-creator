//! Generation API handlers
//!
//! 所有 mutation 通过 application service (GenerationService)。
//! 取消任务时验证状态。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use crate::state::AppState;
use std::sync::Arc;
use db::application_ports::{DbEntityRepositoryPort, DbGenerationRepositoryPort, DbNarrativeRepositoryPort};
use domain::ports::{EntityRepositoryPort, GenerationRepositoryPort, NarrativeRepositoryPort};
use super::error::AppError;
use application::generation_service::GenerationService;
use application::generation_executor::GenerationExecutor;
use infrastructure::llm::{InfraLlmPort, LlmClient, OpenAiCompatibleProvider};

#[derive(Deserialize)]
pub struct CreateGenerationInput { pub r#type: String, pub target_id: Option<String>, pub model: Option<String>, pub parameters: Option<serde_json::Value> }

/// 构建 GenerationExecutor：repo + snapshot repo + proposal repo（落库草稿） + LlmPort。
///
/// 生成文本经抽取后直接通过 ProposalRepositoryPort 写入可提交的草稿，
/// 批准时由 ProposalService + MutationCommitter 落到 World Canon。
fn generation_executor(state: &AppState) -> GenerationExecutor {
    let pool = state.pool.clone();
    let prop_repo = Arc::new(db::application_ports::DbProposalRepositoryPort::new(pool.clone()));
    let snapshots = Arc::new(db::application_ports::DbContextSnapshotRepositoryPort::new(
        pool.clone(),
    ));
    // 注册真实 OpenAI 兼容 Provider（opencode.ai / mimo-v2.5），通过环境变量配置。
    let base_url = std::env::var("OPENCODE_BASE_URL")
        .unwrap_or_else(|_| "https://opencode.ai/zen/go/v1".to_string());
    let api_key = std::env::var("OPENCODE_API_KEY").ok();
    let model = std::env::var("OPENCODE_MODEL").unwrap_or_else(|_| "mimo-v2.5".to_string());
    let mut llm_client = LlmClient::new("opencode".to_string());
    llm_client.add_provider(Arc::new(OpenAiCompatibleProvider::new(
        base_url, api_key, model,
    )));
    let llm = Arc::new(InfraLlmPort::new(llm_client));
    GenerationExecutor::new(
        Arc::new(DbGenerationRepositoryPort::new(pool.clone())),
        snapshots,
        prop_repo,
        llm,
    )
}

pub async fn execute_task(State(state): State<AppState>, Path(task_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| AppError(anyhow::anyhow!("Invalid task ID")))?;
    // 组装「场景 + 世界观」上下文，让生成真正贴合设定（而非空输入客套话）。
    let context = build_scene_context(&state.pool, task_id).await;
    let output = generation_executor(&state).execute(task_id, context).await?;
    Ok(Json(serde_json::json!({ "output": output })))
}

/// 为生成任务组装「场景信息 + 世界观/角色」上下文文本。
///
/// 仅依赖 DB 仓储（narrative-engine 已依赖 db），不触碰未接线的 runtime ContextEngine。
/// 返回 None 表示该任务没有关联场景（无需上下文）。
async fn build_scene_context(pool: &sqlx::PgPool, task_id: Uuid) -> Option<String> {
    let gen_repo = DbGenerationRepositoryPort::new(pool.clone());
    let task = gen_repo.get_task_struct(task_id).await.ok().flatten()?;
    let scene_id = task.scene_id?;

    let narr = DbNarrativeRepositoryPort::new(pool.clone());
    let scene = narr.get_node(scene_id).await.ok().flatten()?;
    let world_id = scene.get("world_id").and_then(|v| v.as_str())?.to_string();
    let world_id = Uuid::parse_str(&world_id).ok()?;
    let title = scene.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let desc = scene.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let attrs = scene.get("attributes").cloned().unwrap_or(serde_json::Value::Null);

    let ent = DbEntityRepositoryPort::new(pool.clone());
    let entities = ent.list_entities(world_id, None).await.ok().unwrap_or_default();
    tracing::info!(scene_id = %scene_id, world_id = %world_id, entity_count = entities.len(), "build_scene_context assembled");

    let mut ctx = String::new();
    ctx.push_str(&format!("【场景】{}\n", title));
    if !desc.is_empty() {
        ctx.push_str(&format!("描述：{}\n", desc));
    }
    if attrs != serde_json::Value::Null {
        ctx.push_str(&format!(
            "设定：{}\n",
            serde_json::to_string_pretty(&attrs).unwrap_or_default()
        ));
    }
    if !entities.is_empty() {
        ctx.push_str("\n【世界观与角色】\n");
        for e in &entities {
            let etype = e.get("entity_type_id").and_then(|v| v.as_str()).unwrap_or("");
            let name = e.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let summary = e.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            let edesc = e.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let mut line = format!("- [{}] {}：", etype, name);
            if !summary.is_empty() {
                line.push_str(&format!("{}；", summary));
            }
            if !edesc.is_empty() {
                line.push_str(edesc);
            }
            line.push('\n');
            ctx.push_str(&line);
        }
    }
    Some(ctx)
}

pub async fn list_tasks(State(state): State<AppState>, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    let tasks = service.list_tasks(project_id).await?;
    Ok(Json(serde_json::json!(tasks)))
}

pub async fn get_task(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid task ID")))?;
    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    let task = service.get_task(id).await?
        .ok_or_else(|| AppError(anyhow::anyhow!("Generation task not found")))?;
    Ok(Json(task))
}

pub async fn create_task(State(state): State<AppState>, Path(project_id): Path<String>, Json(input): Json<CreateGenerationInput>) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = Uuid::parse_str(&project_id).map_err(|_| AppError(anyhow::anyhow!("Invalid project ID")))?;
    let target_id = input.target_id.map(|t| Uuid::parse_str(&t)).transpose()
        .map_err(|_| AppError(anyhow::anyhow!("Invalid target ID")))?;

    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    let task = service.create_task(
        project_id,
        &input.r#type,
        target_id,
        input.model.as_deref(),
        input.parameters.unwrap_or(serde_json::json!({})),
    ).await?;

    Ok(Json(task))
}

pub async fn cancel_task(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError(anyhow::anyhow!("Invalid task ID")))?;
    let service = GenerationService::new(Arc::new(DbGenerationRepositoryPort::new(state.pool.clone())));
    service.cancel_task(id).await?;
    Ok(Json(serde_json::json!({"id": id, "status": "Cancelled"})))
}