//! Agent HTTP 层请求 / 响应 DTO（与 `narrative-engine` 的 handler 共用）。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// `POST /api/agent/session`
#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub project_id: Option<Uuid>,
}

/// `POST /api/agent/session` 响应
#[derive(Serialize)]
pub struct CreateSessionResponse {
    pub session_id: Uuid,
}

/// `GET /api/agent/session/{id}` 响应（直接复用 `AgentSession`，见 session.rs）
/// 这里仅占位以便后续按需裁剪字段；当前直接序列化 `AgentSession`。

/// `POST /api/agent/chat`
#[derive(Deserialize)]
pub struct ChatRequest {
    pub session_id: Uuid,
    pub message: String,
}

/// `POST /api/agent/tool/execute`
#[derive(Deserialize)]
pub struct ExecuteToolRequest {
    pub name: String,
    pub input: Value,
}

/// `POST /api/agent/tool/execute` 响应
#[derive(Serialize)]
pub struct ExecuteToolResponse {
    pub name: String,
    pub result: Value,
}

/// 工具元信息（用于提示词拼接与前端展示）
#[derive(Serialize, Clone)]
pub struct ToolMeta {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// `GET /api/agent/tools`
#[derive(Serialize)]
pub struct ListToolsResponse {
    pub tools: Vec<ToolMeta>,
}
