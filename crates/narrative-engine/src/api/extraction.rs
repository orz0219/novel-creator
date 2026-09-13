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

    // 真实 OpenAI 兼容 Provider：网关参数由运行时 AI 配置提供（设置页可改）。
    let mut llm_client = LlmClient::new("opencode".to_string());
    llm_client.add_provider(Arc::new(OpenAiCompatibleProvider::new(
        state.ai_settings.clone(),
    )));
    let llm = Arc::new(InfraLlmPort::new(llm_client, state.ai_settings.clone()))
        as Arc<dyn LlmPort>;

    ExtractionExecutor::new(proposals, llm, state.ai_settings.clone())
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
    let text = input.text;

    /*
     * 抽取必须放在**独立任务**里跑，handler 只等它的结果。
     *
     * 原因：这一步最长实测 67 秒（同步等 LLM）。如果直接在 handler 里 await，
     * 客户端一断开（手机切后台会被系统挂起），handler 的 future 就被 drop，
     * LLM 调用连同「把草稿写进库」一起中止——用户白等一场，而且什么都不产出。
     * 实测确认过：3 秒后断开连接，75 秒后库里依旧 0 条草稿。
     *
     * 交给 spawn 之后，客户端断开不再影响任务跑完，用户回到 App 至少能在
     * 「AI 提案」里看到抽出来的草稿。
     */
    let task = tokio::spawn(async move { executor.extract(project_id, &text).await });
    match task.await {
        Ok(Ok(result)) => Ok(Json(result)),
        Ok(Err(e)) => Err(AppError::from(e)),
        Err(join_err) => Err(AppError::from(anyhow::anyhow!(
            "抽取任务异常终止: {}",
            join_err
        ))),
    }
}
