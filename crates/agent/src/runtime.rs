//! Agent 运行时（GPT 方案 1 / 3 的编排核心）
//!
//! 持有 LLM 端口、工具注册表、会话存储、记忆，提供：
//! - 创建会话
//! - 聊天（拼接提示词 → 调 LLM → 落库消息）
//! - 工具执行
//! - 记忆存取
//!
//! 依赖方向：仅依赖 `domain::ports::LlmPort`（端口），具体 LLM 在组合根注入。

use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_stream::stream;
use chrono::Utc;
use domain::ports::{AgentPromptConfig, LlmPort, PromptRepositoryPort};
use futures::Stream;
use futures::StreamExt;
use serde::Serialize;
use uuid::Uuid;

use crate::memory::{AgentMemory, MemoryItem};
use crate::prompt::build_system_prompt;
use crate::session::{AgentSession, ChatMessage, SessionStore};
use crate::tool::{AskQuestionTool, EchoTool, ToolRegistry};
use crate::types::ToolMeta;

/// Agent 流式输出事件。
pub enum AgentStreamEvent {
    /// 一段助手文本 token。
    Token(String),
    /// 一道选择题（前端渲染为选项卡片 + 自由输入框）。
    Question { question: String, options: Vec<String> },
    /// 本轮结束。
    Done,
    /// 出错（message 为错误信息）。
    Error(String),
}

/// 提示词视图（API 返回）：含生效提示词、内置默认、是否已被自定义。
#[derive(Debug, Clone, Serialize)]
pub struct PromptView {
    pub scope: String,
    /// 当前生效提示词（若有自定义则用自定义，否则用内置默认）。
    pub system_prompt: String,
    /// 内置默认基座（用于「恢复默认」）。
    pub default_prompt: String,
    /// 是否存在用户自定义覆盖。
    pub is_customized: bool,
}

pub struct AgentRuntime {
    llm: Arc<dyn LlmPort>,
    tools: Arc<ToolRegistry>,
    sessions: Arc<dyn SessionStore>,
    memory: Arc<dyn AgentMemory>,
    /// 提示词持久化（用户自定义基座落库）。
    prompt_store: Arc<dyn PromptRepositoryPort>,
    /// 内置默认基座（无自定义时使用，且用于「恢复默认」）。
    default_system_prompt: String,
    /// 透传给 LLM 的模型名（具体生效值由 Provider 配置决定）。
    model: String,
}

impl AgentRuntime {
    pub fn new(
        llm: Arc<dyn LlmPort>,
        tools: Arc<ToolRegistry>,
        sessions: Arc<dyn SessionStore>,
        memory: Arc<dyn AgentMemory>,
        prompt_store: Arc<dyn PromptRepositoryPort>,
        default_system_prompt: String,
        model: String,
    ) -> Self {
        Self {
            llm,
            tools,
            sessions,
            memory,
            prompt_store,
            default_system_prompt,
            model,
        }
    }

    /// 注册 P1 基础工具（echo / ask_question）。
    /// 真实领域工具（create_character / revise_entity / retire_entity …）在组合根
    /// `narrative-engine` 构造并注册，不在此处。
    /// 返回 `self` 以便链式构造。
    pub fn with_default_tools(self) -> Self {
        self.tools.register(Arc::new(EchoTool));
        self.tools.register(Arc::new(AskQuestionTool));
        self
    }

    pub fn list_tools(&self) -> Vec<ToolMeta> {
        self.tools.list()
    }

    pub async fn create_session(&self, project_id: Option<Uuid>) -> Result<Uuid> {
        let s = AgentSession::new(project_id);
        let id = s.id;
        self.sessions.create(s).await?;
        Ok(id)
    }

    pub async fn get_session(&self, id: Uuid) -> Result<Option<AgentSession>> {
        self.sessions.get(id).await
    }

    /// 重命名会话（用于历史列表中的自定义标题）。
    pub async fn rename_session(&self, id: Uuid, title: &str) -> Result<()> {
        let mut s = self
            .sessions
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("session not found: {}", id))?;
        s.title = Some(title.to_string());
        self.sessions.update(s).await
    }

    /// 删除会话（含其消息，由存储层级联）。
    pub async fn delete_session(&self, id: Uuid) -> Result<()> {
        self.sessions.delete(id).await
    }

    /// 列出全部会话（用于历史会话侧栏）。
    pub async fn list_sessions(&self) -> Result<Vec<AgentSession>> {
        self.sessions.list().await
    }

    /// 读取某作用域的提示词视图（含内置默认与是否自定义）。
    pub async fn get_prompt(&self, scope: &str) -> Result<PromptView> {
        let cfg = self.prompt_store.load(scope).await?;
        let effective = cfg
            .as_ref()
            .map(|c| c.system_prompt.clone())
            .unwrap_or_else(|| self.default_system_prompt.clone());
        Ok(PromptView {
            scope: scope.to_string(),
            system_prompt: effective,
            default_prompt: self.default_system_prompt.clone(),
            is_customized: cfg.is_some(),
        })
    }

    /// 保存（upsert）某作用域的自定义提示词基座。
    pub async fn save_prompt(&self, scope: &str, system_prompt: &str, project_id: Option<Uuid>) -> Result<()> {
        let cfg = AgentPromptConfig {
            id: Uuid::new_v4(),
            scope: scope.to_string(),
            project_id,
            system_prompt: system_prompt.to_string(),
            updated_at: Utc::now(),
        };
        self.prompt_store.save(&cfg).await
    }

    /// 删除某作用域的自定义覆盖（恢复为内置默认）。
    pub async fn delete_prompt(&self, scope: &str) -> Result<()> {
        self.prompt_store.delete(scope).await
    }

    /// 解析当前生效的基座：有自定义用自定义，否则用内置默认。
    async fn resolve_base(&self) -> Result<String> {
        let cfg = self.prompt_store.load("global").await?;
        Ok(cfg
            .map(|c| c.system_prompt)
            .unwrap_or_else(|| self.default_system_prompt.clone()))
    }

    /// 聊天一轮：记录用户消息 → 拼接提示词 → 调 LLM → 记录助手消息。
    ///
    /// 注意：P1 复用**非流式** `LlmPort::complete`，把历史拍平进单个 user prompt。
    /// 多轮对话历史靠 session.messages 维护；真实消息数组式流式在 P2。
    pub async fn chat(&self, session_id: Uuid, message: &str) -> Result<String> {
        let mut session = self
            .sessions
            .get(session_id)
            .await?
            .with_context(|| format!("session not found: {}", session_id))?;

        session.messages.push(ChatMessage {
            role: "user".into(),
            content: message.to_string(),
            created_at: Utc::now(),
        });

        // 召回该项目已记住的设定 / 偏好，注入提示词
        let memories: Vec<String> = match session.project_id {
            Some(pid) => self
                .memory
                .list(pid)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|m| format!("[{}] {}", m.memory_type, m.content))
                .collect(),
            None => Vec::new(),
        };

        let system = build_system_prompt(
            &self.resolve_base().await?,
            &session.current_step,
            &self.tools.list(),
            &memories,
        );

        // P1：把历史拍平进单个 user prompt（复用非流式 complete）
        let history = session
            .messages
            .iter()
            .map(|m| format!("{}：{}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let user_prompt = format!("{}\n\n用户：{}", history, message);

        let reply = self.llm.complete(&system, &user_prompt, &self.model).await?;

        session.messages.push(ChatMessage {
            role: "assistant".into(),
            content: reply.clone(),
            created_at: Utc::now(),
        });
        session.updated_at = Utc::now();
        self.sessions.update(session).await?;

        Ok(reply)
    }

    /// 真正的流式聊天：调 `LlmPort::stream_complete`，逐 token 透传。
    ///
    /// - 普通回复：每个 LLM token 作为 `Token` 事件产出。
    /// - 选择题：若 LLM 以 `<<ASK_QUESTION>>` 起始、以 `<<END>>` 收尾输出 JSON，
    ///   则解析为 `Question` 事件并持久化为助手消息（便于重载渲染）；
    ///   解析失败则退回为普通文本。
    pub async fn chat_stream(
        &self,
        session_id: Uuid,
        message: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<AgentStreamEvent>> + Send>>> {
        let mut session = self
            .sessions
            .get(session_id)
            .await?
            .with_context(|| format!("session not found: {}", session_id))?;

        session.messages.push(ChatMessage {
            role: "user".into(),
            content: message.to_string(),
            created_at: Utc::now(),
        });

        // 检测本轮是否回答上一轮的提问（用于自动记忆捕获）
        let answered_question = session
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant" && m.content.contains("<<ASK_QUESTION>>"))
            .and_then(|m| extract_question_text(&m.content));
        let answer_text = message.to_string();

        // 跨刷新记忆：读取本会话已记住的偏好/设定，注入系统提示词
        let memories: Vec<String> = self
            .memory
            .list(session.id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| format!("[{}] {}", m.memory_type, m.content))
            .collect();

        let system = build_system_prompt(
            &self.resolve_base().await?,
            &session.current_step,
            &self.tools.list(),
            &memories,
        );
        let history = session
            .messages
            .iter()
            .map(|m| format!("{}：{}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let user_prompt = format!("{}\n\n用户：{}", history, message);

        session.updated_at = Utc::now();
        // 先落库 user 消息（clone 一份，原值留给流闭包统一再存）
        self.sessions.update(session.clone()).await?;

        // 克隆 Arc，移入流闭包，使返回的流为 'static + Send。
        let llm = self.llm.clone();
        let model = self.model.clone();
        let sessions = self.sessions.clone();
        let memory = self.memory.clone();
        // 把已加载的会话直接移入流闭包：流内不再回查，避免竞态导致助手消息漏存
        let mut session = session;
        // 自动记忆捕获所需的上下文（移入流闭包）
        let answered_question = answered_question;
        let answer_text = answer_text;
        let session_key = session_id;

        const MARKER: &str = "<<ASK_QUESTION>>";
        const END: &str = "<<END>>";

        let s = stream! {
            let mut llm_stream = match llm.stream_complete(&system, &user_prompt, &model).await {
                Ok(st) => st,
                Err(e) => { yield Ok::<_, anyhow::Error>(AgentStreamEvent::Error(e.to_string())); return; }
            };
            let mut acc = String::new();
            let mut in_q = false;
            let mut qjson = String::new();
            let mut question: Option<(String, Vec<String>)> = None;

            while let Some(res) = llm_stream.next().await {
                match res {
                    Ok(tok) => {
                        acc.push_str(&tok);
                        if !in_q {
                            if acc.contains(MARKER) {
                                in_q = true;
                                if let Some(pos) = acc.find(MARKER) {
                                    qjson = acc[pos + MARKER.len()..].to_string();
                                }
                            } else {
                                yield Ok(AgentStreamEvent::Token(tok));
                            }
                        }
                        if in_q {
                            if let Some(pos) = acc.find(MARKER) {
                                qjson = acc[pos + MARKER.len()..].to_string();
                            }
                            if qjson.contains(END) {
                                let json_str = qjson[..qjson.find(END).unwrap()].trim().to_string();
                                if let Some((q, opts)) = parse_ask_question(&json_str) {
                                    // 命中选择题：把助手消息（标记格式）写入会话
                                    question = Some((q.clone(), opts.clone()));
                                    let content = format!("{}{}{}", MARKER, json_str, END);
                                    session.messages.push(ChatMessage {
                                        role: "assistant".into(),
                                        content,
                                        created_at: Utc::now(),
                                    });
                                    // 先落库再下发：避免客户端在收到问题事件后立即断开导致漏存
                                    if let Err(e) = sessions.update(session.clone()).await {
                                        yield Ok(AgentStreamEvent::Error(format!("保存会话失败: {}", e)));
                                    }
                                    yield Ok(AgentStreamEvent::Question { question: q, options: opts });
                                    break;
                                } else {
                                    // 解析失败：退回文本（回放已收集内容）
                                    yield Ok(AgentStreamEvent::Token(qjson.clone()));
                                    in_q = false;
                                }
                            }
                        }
                    }
                    Err(e) => { yield Ok(AgentStreamEvent::Error(e.to_string())); return; }
                }
            }

            // 流结束：普通回复把整段文本作为助手消息（同样先落库再下发）
            if question.is_none() && !acc.trim().is_empty() {
                session.messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: acc.trim().to_string(),
                    created_at: Utc::now(),
                });
                if let Err(e) = sessions.update(session.clone()).await {
                    yield Ok(AgentStreamEvent::Error(format!("保存会话失败: {}", e)));
                }
            }
            if question.is_none() {
                yield Ok(AgentStreamEvent::Done);
            }
            // 统一落库（user + assistant），失败则上报错误事件
            if let Err(e) = sessions.update(session).await {
                yield Ok(AgentStreamEvent::Error(format!("保存会话失败: {}", e)));
            }
            // 自动记忆捕获：若本轮是对上一轮提问的回答，则持久化偏好，跨刷新保留
            if let Some(q) = answered_question {
                if let Err(e) = memory
                    .save(session_key, "preference", &format!("问：{}\n答：{}", q, answer_text))
                    .await
                {
                    yield Ok(AgentStreamEvent::Error(format!("记忆保存失败: {}", e)));
                }
            }
        };

        Ok(Box::pin(s))
    }

    /// 执行工具。先做"输入须为对象"校验，再做 JSON Schema 轻量校验（required + type），
    /// 最后交给工具实现。Schema 校验失败会返回明确错误，不静默放行。
    pub async fn execute_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let tool = self
            .tools
            .get(name)
            .with_context(|| format!("tool not found: {}", name))?;

        if !input.is_object() {
            anyhow::bail!("tool input must be a JSON object");
        }

        if let Err(e) = validate_input(&input, &tool.input_schema()) {
            anyhow::bail!("工具 '{}' 入参校验失败: {}", name, e);
        }

        tool.execute(input).await
    }

    pub async fn remember(&self, session_id: Uuid, memory_type: &str, content: &str) -> Result<()> {
        self.memory.save(session_id, memory_type, content).await
    }

    pub async fn recall(&self, session_id: Uuid) -> Result<Vec<MemoryItem>> {
        self.memory.list(session_id).await
    }
}

/// 从助手消息内容中提取提问文本（若其为 `<<ASK_QUESTION>>...<<END>>` 格式）。
fn extract_question_text(content: &str) -> Option<String> {
    const MARKER: &str = "<<ASK_QUESTION>>";
    const END: &str = "<<END>>";
    let start = content.find(MARKER)? + MARKER.len();
    let end = content.find(END)?;
    if start >= end {
        return None;
    }
    let json = &content[start..end];
    parse_ask_question(json.trim()).map(|(q, _)| q)
}

/// 解析 `ask_question` 的 JSON：`{"question":"...","options":["...",...]}`。
///
/// 容忍模型在 JSON 对象后附带多余字符（如 `}xxx<<END>>`），只取首个 `{` 到最后一个 `}` 之间的内容。
/// 模块级自由函数，便于在 `chat_stream` 的流闭包内直接调用。
fn parse_ask_question(s: &str) -> Option<(String, Vec<String>)> {
    let json = extract_json_object(s)?;
    let v: serde_json::Value = serde_json::from_str(&json).ok()?;
    let question = v.get("question")?.as_str()?.to_string();
    let options = v
        .get("options")?
        .as_array()?
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();
    if question.trim().is_empty() || options.is_empty() {
        return None;
    }
    Some((question, options))
}

/// 从文本中取出第一个完整的 JSON 对象（首个 `{` 到最后一个 `}`），容忍前后多余字符。
fn extract_json_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if start >= end {
        return None;
    }
    Some(s[start..=end].to_string())
}

/// 轻量 JSON Schema 校验：检查 `required` 字段存在且非空，并按 `properties` 中的
/// `type` 做基础类型校验（string/object/array/number/boolean）。不做深层结构校验。
fn validate_input(input: &serde_json::Value, schema: &serde_json::Value) -> Result<()> {
    let obj = input
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("input 不是 JSON 对象"))?;

    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        for r in required {
            if let Some(key) = r.as_str() {
                match obj.get(key) {
                    None => anyhow::bail!("缺少必填字段: {}", key),
                    Some(v) if v.is_null() => anyhow::bail!("必填字段为 null: {}", key),
                    _ => {}
                }
            }
        }
    }

    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (k, v) in obj {
            if let Some(spec) = props.get(k) {
                if let Some(t) = spec.get("type").and_then(|t| t.as_str()) {
                    let ok = match t {
                        "string" => v.is_string(),
                        "object" => v.is_object(),
                        "array" => v.is_array(),
                        "number" => v.is_number(),
                        "boolean" => v.is_boolean(),
                        "integer" => v.is_i64() || v.is_u64(),
                        _ => true,
                    };
                    if !ok {
                        anyhow::bail!("字段 '{}' 应为类型 {}", k, t);
                    }
                }
            }
        }
    }

    Ok(())
}
