//! Agent 核心层（P1：基础框架）
//!
//! 对应 GPT 方案 Phase 1：对话 / Session / SSE streaming / Tool framework。
//! 本 crate 提供**与 HTTP 无关**的 Agent 核心抽象，由 `narrative-engine`
//! 接线为 `/api/agent/*` 路由。
//!
//! 依赖方向：仅依赖 `domain`（取 `LlmPort` 端口），不直接依赖 `infrastructure`，
//! 具体 LLM 实现在组合根（narrative-engine）注入，符合现有依赖倒置约定。
//!
//! 已知 P1 范围边界（显式留给后续 Phase）：
//! - 会话 / 记忆用内存实现；持久化 4 张表（agent_sessions / agent_messages /
//!   agent_memory / guide_progress）在 P2/P3 接入。
//! - SSE 为**真实 token 级流式**：Provider `stream:true`，逐 delta 经 AgentRuntime
//!   透传到前端（端点级切词已不再使用）。
//! - 工具含基础工具（echo / ask_question）；真实领域工具（create_character /
//!   revise_entity / retire_entity …）在组合根 `narrative-engine` 构造并注册到
//!   `ToolRegistry`，覆盖 C / U / D（逻辑删除）+ R。
//! - Tool 调用做"输入须为对象"校验 + JSON Schema 轻量校验（required + type）。

pub mod types;
pub mod tool;
pub mod session;
pub mod memory;
pub mod prompt;
pub mod runtime;
pub mod prompt_store;

pub use runtime::AgentRuntime;
pub use runtime::AgentStreamEvent;
pub use runtime::PromptView;
pub use tool::{AgentTool, ToolRegistry, EchoTool, AskQuestionTool};
pub use session::{AgentSession, SessionStore, InMemorySessionStore, ChatMessage};
pub use memory::{AgentMemory, InMemoryAgentMemory, MemoryItem};
pub use prompt::DEFAULT_SYSTEM_PROMPT_BASE;
pub use prompt_store::InMemoryPromptRepo;
pub use types::*;
