//! Extraction API — M1 文本→实体/关系抽取闭环
//!
//! POST /api/v1/projects/{id}/extract  { "text": "..." }
//! → 调 ExtractionExecutor（LLM 抽取 + 创建 Proposal 草稿）→ 返回 ExtractionResult 预览。
//! 草稿落在库里，由前端 ProposalReview 呈现，人工批准后经既有 commit 边界落到 Canon。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use domain::extraction::ExtractionResult;
use domain::ports::{LlmPort, ProposalRepositoryPort};
use infrastructure::llm::{InfraLlmPort, LlmClient, OpenAiCompatibleProvider};

use application::extraction_executor::ExtractionExecutor;
use db::application_ports::DbProposalRepositoryPort;

use super::error::AppError;

#[derive(Deserialize)]
pub struct ExtractTextInput {
    pub text: String,
}

/// 构建 ExtractionExecutor：复用与 generation 相同的真实 opencode Provider。
fn extraction_executor(state: &crate::state::AppState) -> ExtractionExecutor {
    let pool = state.pool.clone();
    let proposals =
        Arc::new(DbProposalRepositoryPort::new(pool.clone())) as Arc<dyn ProposalRepositoryPort>;

    let base_url = std::env::var("OPENCODE_BASE_URL")
        .unwrap_or_else(|_| "https://opencode.ai/zen/go/v1".to_string());
    let api_key = std::env::var("OPENCODE_API_KEY").ok();
    let model = std::env::var("OPENCODE_MODEL").unwrap_or_else(|_| "mimo-v2.5".to_string());
    let mut llm_client = LlmClient::new("opencode".to_string());
    llm_client.add_provider(Arc::new(OpenAiCompatibleProvider::new(
        base_url, api_key, model,
    )));
    let llm = Arc::new(InfraLlmPort::new(llm_client)) as Arc<dyn LlmPort>;

    ExtractionExecutor::new(proposals, llm)
}

pub async fn extract_text(
    State(state): State<crate::state::AppState>,
    Path(project_id): Path<String>,
    Json(input): Json<ExtractTextInput>,
) -> Result<Json<ExtractionResult>, AppError> {
    let project_id = Uuid::parse_str(&project_id)
        .map_err(|_| AppError::from(anyhow::anyhow!("Invalid project ID")))?;
    if input.text.trim().is_empty() {
        return Err(AppError::from(anyhow::anyhow!("text 不能为空")));
    }
    let executor = extraction_executor(&state);
    let result = executor.extract(project_id, &input.text).await?;
    Ok(Json(result))
}
