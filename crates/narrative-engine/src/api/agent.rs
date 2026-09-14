//! Agent HTTP handlers（P1：/api/v1/agent/*）
//!
//! 对应 GPT 方案：创建 Session、工具列表、SSE 聊天、工具执行。
//! SSE 为**真实 token 级流式**：AgentRuntime 直接透传 LLM 的 delta，
//! 选择题则通过 `question` 事件下发（JSON：question + options）。

use axum::extract::{Path, Query, State, Json};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use uuid::Uuid;

use crate::api::error::AppError;
use crate::state::AppState;
use agent::{
    ChatRequest, CreateSessionRequest, CreateSessionResponse, ExecuteToolRequest,
    ExecuteToolResponse, ListToolsResponse,
};
use serde_json;

/// `POST /api/agent/session` —— 创建引导会话。
pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, AppError> {
    let id = state.agent.create_session(req.project_id).await?;
    Ok(Json(CreateSessionResponse { session_id: id }))
}

/// `GET /api/agent/session/{id}` —— 读取会话（含历史消息）。
pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<agent::AgentSession>, AppError> {
    let session = state
        .agent
        .get_session(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
    Ok(Json(session))
}

/// `GET /api/v1/agent/session/{id}/context` —— 该会话的上下文用量（聊天页预警）。
pub async fn context_usage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<agent::ContextUsage>, AppError> {
    let usage = state.agent.context_usage(id).await?;
    Ok(Json(usage))
}

/// `GET /api/agent/sessions?project_id=xxx` —— 列出某项目下的会话（按项目隔离）。
pub async fn list_sessions(
    State(state): State<AppState>,
    Query(params): Query<ListSessionsQuery>,
) -> Result<Json<Vec<agent::AgentSession>>, AppError> {
    let project_id = Uuid::parse_str(&params.project_id)
        .map_err(|_| anyhow::anyhow!("Invalid project ID"))?;
    let sessions = state.agent.list_sessions_by_project(project_id).await?;
    Ok(Json(sessions))
}

/// `DELETE /api/agent/session/{id}` —— 删除会话（含消息，由存储层级联）。
pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    state.agent.delete_session(id).await?;
    Ok(())
}

/// `PUT /api/agent/session/{id}` —— 重命名会话。
pub async fn rename_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameSessionRequest>,
) -> Result<(), AppError> {
    state.agent.rename_session(id, &req.title).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct TruncateRequest {
    /// 从第几条消息开始删除（0 基下标）；该条及其之后的全部消息都会被移除。
    pub from_index: usize,
}

/// `POST /api/v1/agent/session/{id}/truncate` —— 截断会话（只保留上半部分）。
///
/// 删除第 `from_index` 条消息及其之后的全部内容（含后续的用户消息、AI 回复与工具记录），
/// 用于把发错或跑偏的消息连同其造成的上下文污染一起清掉。
/// 返回截断后的完整会话，便于前端直接替换界面。
pub async fn truncate_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TruncateRequest>,
) -> Result<Json<agent::AgentSession>, AppError> {
    let session = state.agent.truncate_session(id, req.from_index).await?;
    Ok(Json(session))
}

#[derive(Debug, Deserialize)]
pub struct RenameSessionRequest {
    title: String,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    project_id: String,
}

/// `GET /api/agent/tools` —— 列出当前可用工具。
pub async fn list_tools(
    State(state): State<AppState>,
) -> Result<Json<ListToolsResponse>, AppError> {
    Ok(Json(ListToolsResponse {
        tools: state.agent.list_tools(),
    }))
}

/// `POST /api/agent/tool/execute` —— 执行一个工具。
pub async fn execute_tool(
    State(state): State<AppState>,
    Json(req): Json<ExecuteToolRequest>,
) -> Result<Json<ExecuteToolResponse>, AppError> {
    let result = state.agent
        .execute_tool(req.project_id, &req.name, req.input)
        .await?;
    Ok(Json(ExecuteToolResponse {
        name: req.name,
        result,
    }))
}

/// `POST /api/agent/chat` —— SSE 聊天端点（真实 token 级流式）。
///
/// 先发 `status: thinking` 让客户端立即拿到 SSE 头；随后把 AgentRuntime 发出的
/// 流事件逐一转成 SSE 事件：`token`（文本片段）、`question`（选择题 JSON）、
/// `done`（结束）、`error`（出错）。
pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    use async_stream::stream;
    use futures::StreamExt;
    let agent = state.agent.clone();
    let session_id = req.session_id;
    let message = req.message;

    let s = stream! {
        yield Ok::<_, Infallible>(Event::default().event("status").data("thinking"));
        let mut rs = match agent.chat_stream(session_id, &message).await {
            Ok(st) => st,
            Err(e) => {
                yield Ok(Event::default().event("error").data(e.to_string()));
                return;
            }
        };
        while let Some(ev) = rs.next().await {
            match ev {
                Ok(agent::AgentStreamEvent::Token(t)) => {
                    yield Ok(Event::default().event("token").data(t));
                }
                Ok(agent::AgentStreamEvent::Question { question, options }) => {
                    let payload = serde_json::json!({ "question": question, "options": options }).to_string();
                    yield Ok(Event::default().event("question").data(payload));
                }
                Ok(agent::AgentStreamEvent::Tool { name, input, ok, output }) => {
                    let payload = serde_json::json!({
                        "name": name,
                        "input": input,
                        "ok": ok,
                        "output": output,
                    })
                    .to_string();
                    yield Ok(Event::default().event("tool").data(payload));
                }
                Ok(agent::AgentStreamEvent::Usage(u)) => {
                    // 用量 + 缓存命中率：前端把它显示在「上下文」那一行，
                    // 用来判断网关侧提示缓存是否在正常工作（命中率长期为 0 说明每轮都在重算）。
                    let payload = serde_json::json!({
                        "prompt_tokens": u.prompt_tokens,
                        "completion_tokens": u.completion_tokens,
                        "total_tokens": u.total_tokens,
                        "cached_tokens": u.cached_tokens,
                        "cache_hit_rate": u.cache_hit_rate(),
                    })
                    .to_string();
                    yield Ok(Event::default().event("usage").data(payload));
                }
                Ok(agent::AgentStreamEvent::Done) => {
                    yield Ok(Event::default().event("done").data(""));
                }
                Ok(agent::AgentStreamEvent::Error(e)) => {
                    yield Ok(Event::default().event("error").data(e));
                }
                Err(e) => {
                    yield Ok(Event::default().event("error").data(e.to_string()));
                }
            }
        }
    };

    Sse::new(s).keep_alive(KeepAlive::default())
}

/// `GET /api/agent/prompt?scope=global` —— 读取当前生效提示词视图。
pub async fn get_prompt(
    State(state): State<AppState>,
    Query(params): Query<PromptQuery>,
) -> Result<Json<agent::PromptView>, AppError> {
    let scope = params.scope.unwrap_or_else(|| "global".to_string());
    let view = state.agent.get_prompt(&scope).await?;
    Ok(Json(view))
}

/// `PUT /api/agent/prompt` —— 保存（upsert）自定义提示词基座。
pub async fn save_prompt(
    State(state): State<AppState>,
    Json(req): Json<SavePromptRequest>,
) -> Result<Json<agent::PromptView>, AppError> {
    state
        .agent
        .save_prompt(&req.scope, &req.system_prompt, None)
        .await?;
    let view = state.agent.get_prompt(&req.scope).await?;
    Ok(Json(view))
}

/// `DELETE /api/agent/prompt?scope=global` —— 删除自定义覆盖（恢复内置默认）。
pub async fn delete_prompt(
    State(state): State<AppState>,
    Query(params): Query<PromptQuery>,
) -> Result<(), AppError> {
    let scope = params.scope.unwrap_or_else(|| "global".to_string());
    state.agent.delete_prompt(&scope).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct PromptQuery {
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SavePromptRequest {
    scope: String,
    system_prompt: String,
}

/// `POST /api/v1/agent/guide/confirm` —— 前端用户点"确认推进"按钮触发。
///
/// 调 confirm_step 工具（已注册到 ToolRegistry），传入 project_id。
/// 工具内部完成：组装 MinCompleteSnapshot → 校验 → 通过则推进
/// `project.config.current_step` 到下一步。失败则返回结构化报告。
///
/// 可选参数 `target_step`：当用户从"血肉小选择器"点过来时传入，
/// 让后端校验指定 step 的产物（不传则按当前 stored_step 校验）。
pub async fn confirm_guide_step(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let project_id = req
        .get("project_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("missing or invalid project_id"))?;
    let mut input = serde_json::json!({"project_id": project_id.to_string()});
    if let Some(ts) = req.get("target_step").and_then(|v| v.as_str()) {
        input["target_step"] = serde_json::json!(ts);
    }
    let result = state
        .agent
        .execute_tool(project_id, "confirm_step", input)
        .await?;
    Ok(Json(result))
}

/// `GET /api/v1/projects/{id}/guide/status` —— 引导进度的**只读真源**。
///
/// 前端步骤条的每一个对勾都取自这里，与 `confirm_step` 的推进校验共用同一套快照
/// 与同一个 `validate_step`，因此「界面打勾」与「点按钮能不能过」永远一致。
///
/// 存在理由：血肉步（地图 / 势力 / 道具 / 配角 / 副线）原先由前端各自数数量判断，
/// 一旦漏掉某一步，就会出现「产物明明齐了却永远不打勾」——而且没有任何报错。
pub async fn get_guide_status(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 走已注册的 get_project_status 工具，而不是另写一份查询：
    // 前端打对勾的依据必须与 AI 回答、与 confirm_step 的校验是同一份判定。
    let input = serde_json::json!({"project_id": project_id.to_string()});
    let report = state
        .agent
        .execute_tool(project_id, "get_project_status", input)
        .await?;
    Ok(Json(report))
}

/// `POST /api/v1/agent/session/{id}/summary` —— 会话收尾：归纳成滚动摘要。
///
/// 把整段会话（连同项目已有的旧摘要作为基线）交给模型归纳成结构化 JSON，
/// **覆盖**项目那一份摘要，并同步一份到 `agent_memory` 供新会话注入。
/// 重复调用只会刷新那一份，不会堆积多份。
pub async fn summarize_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<domain::session_summary::StoredSessionSummary>, AppError> {
    let summary = state.agent.summarize_session(id).await?;
    Ok(Json(summary))
}

/// `GET /api/v1/agent/session/{id}/summary` —— 读取该项目当前的滚动摘要。
///
/// 从未收尾过时返回 `null`（不是错误）：界面据此显示「还没有摘要」。
pub async fn get_session_summary(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<domain::session_summary::StoredSessionSummary>>, AppError> {
    let session = state
        .agent
        .get_session(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
    let summary = state.agent.load_summary(session.project_id).await?;
    Ok(Json(summary))
}
