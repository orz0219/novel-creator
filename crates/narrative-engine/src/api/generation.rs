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
use db::application_ports::DbGenerationRepositoryPort;
use super::error::AppError;
use application::generation_service::GenerationService;
use application::generation_executor::GenerationExecutor;
use application::mutation::MutationCommitter;
use application::proposal_service::ProposalService;
use db::mutation_committer::DbMutationCommitter;
use infrastructure::llm::{InfraLlmPort, LlmClient, OpenAiCompatibleProvider};

#[derive(Deserialize)]
pub struct CreateGenerationInput { pub r#type: String, pub target_id: Option<String>, pub model: Option<String>, pub parameters: Option<serde_json::Value> }

/// 构建 GenerationExecutor：repo + snapshot repo + ProposalService（committer 背书）+ LlmPort。
fn generation_executor(state: &AppState) -> GenerationExecutor {
    let pool = state.pool.clone();
    let committer = Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(
        pool.clone(),
    ))));
    let proposals = Arc::new(ProposalService::new(
        Arc::new(db::application_ports::DbProposalRepositoryPort::new(pool.clone())),
        committer,
    ));
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
        proposals,
        llm,
    )
}

pub async fn execute_task(State(state): State<AppState>, Path(task_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let task_id = Uuid::parse_str(&task_id).map_err(|_| AppError(anyhow::anyhow!("Invalid task ID")))?;
    let output = generation_executor(&state).execute(task_id).await?;
    Ok(Json(serde_json::json!({ "output": output })))
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