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
use domain::ports::{
    AgentPromptConfig, AiSettingsPort, GenerationPurpose, GuideProgressPort, LlmPort,
    LlmStreamChunk, LlmUsage, PromptRepositoryPort,
};
use futures::Stream;
use futures::StreamExt;
use serde::Serialize;
use uuid::Uuid;

use crate::memory::{AgentMemory, MemoryItem};
use domain::session_summary::{SessionSummaryPort, StoredSessionSummary};
use crate::prompt::build_system_prompt;
use crate::session::{AgentSession, ChatMessage, SessionStore};
use crate::tool::{AskQuestionTool, EchoTool, ToolRegistry};
use crate::types::ToolMeta;
use crate::usage::{estimate_message_tokens, ContextUsage};

/// Agent 流式输出事件。
pub enum AgentStreamEvent {
    /// 一段助手文本 token。
    Token(String),
    /// 一道选择题（前端渲染为选项卡片 + 自由输入框）。
    Question { question: String, options: Vec<String> },
    /// 工具调用及其结果（前端渲染为工具卡片）。
    Tool {
        /// 工具名。
        name: String,
        /// 模型传入的入参（原样回显）。
        input: serde_json::Value,
        /// 是否执行成功。
        ok: bool,
        /// 成功时为结果 JSON（已美化）；失败时为错误信息。
        output: String,
    },
    /// 本轮 LLM 调用的用量统计（含提示缓存命中）。
    ///
    /// 工具循环里可能调用多次模型，这里下发的是**最后一次**的统计——
    /// 它对应"当前上下文有多大、缓存命中了多少"，正是用户想看的指标。
    Usage(LlmUsage),
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
    /// 运行时 AI 配置（模型名等）：每次对话前读取，设置页改完立即生效。
    ai_settings: Arc<dyn AiSettingsPort>,
    /// 引导进度：以 `project.config.current_step` 为唯一真源。
    guide_progress: Arc<dyn GuideProgressPort>,
    /// 滚动摘要（项目级，恒定一份）：会话收尾时更新，开新会话时读到。
    summaries: Arc<dyn SessionSummaryPort>,
}

impl AgentRuntime {
    pub fn new(
        llm: Arc<dyn LlmPort>,
        tools: Arc<ToolRegistry>,
        sessions: Arc<dyn SessionStore>,
        memory: Arc<dyn AgentMemory>,
        prompt_store: Arc<dyn PromptRepositoryPort>,
        default_system_prompt: String,
        ai_settings: Arc<dyn AiSettingsPort>,
        guide_progress: Arc<dyn GuideProgressPort>,
        summaries: Arc<dyn SessionSummaryPort>,
    ) -> Self {
        Self {
            llm,
            tools,
            sessions,
            memory,
            prompt_store,
            default_system_prompt,
            ai_settings,
            guide_progress,
            summaries,
        }
    }

    /// 当前引导阶段：以项目级 `config.current_step` 为准（多会话共享）。
    ///
    /// 项目尚未写入步骤时用 `guide::INITIAL_STEP`——即「还没开始引导」的正常起点。
    async fn current_step_of(&self, project_id: Uuid) -> Result<String> {
        Ok(self
            .guide_progress
            .current_step(project_id)
            .await?
            .unwrap_or_else(|| crate::guide::INITIAL_STEP.to_string()))
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

    pub async fn create_session(&self, project_id: Uuid) -> Result<Uuid> {
        let s = AgentSession::new(project_id);
        let id = s.id;
        self.sessions.create(s).await?;
        Ok(id)
    }

    pub async fn get_session(&self, id: Uuid) -> Result<Option<AgentSession>> {
        self.sessions.get(id).await
    }

    /// 当前会话的上下文用量（聊天页显示「已用 / 上限」并预警）。
    ///
    /// 由于整段会话历史会被拍平成一次请求，用量按会话全部消息累加估算，
    /// 上限取设置页配置的预算（网关不返回模型真实上下文长度，见 `usage` 模块说明）。
    pub async fn context_usage(&self, id: Uuid) -> Result<ContextUsage> {
        let session = self
            .sessions
            .get(id)
            .await?
            .context("会话不存在，无法统计上下文用量")?;
        let config = self.ai_settings.load().await?;

        let used_tokens = session
            .messages
            .iter()
            .map(|m| estimate_message_tokens(&m.content))
            .sum();

        Ok(ContextUsage {
            used_tokens,
            limit_tokens: config.context_limit,
            message_count: session.messages.len(),
            model: config.model,
        })
    }

    /// 截断会话：只保留前 `keep` 条消息，其余全部删除。
    ///
    /// 用于「删除某条消息及其之后的全部内容」——错误或跑偏的消息会污染后续上下文，
    /// 因此必须连同其后产生的用户消息、AI 回复与工具记录一并移除。
    /// `keep = 0` 表示清空全部消息（会话本身保留）。
    pub async fn truncate_session(&self, id: Uuid, keep: usize) -> Result<AgentSession> {
        let mut session = self
            .sessions
            .get(id)
            .await?
            .context("会话不存在，无法截断")?;

        if keep > session.messages.len() {
            anyhow::bail!(
                "保留条数 {} 超出当前消息总数 {}",
                keep,
                session.messages.len()
            );
        }

        session.messages.truncate(keep);
        session.updated_at = Utc::now();
        self.sessions.update(session.clone()).await?;
        Ok(session)
    }

    /// 重命名会话（用于历史列表中的自定义标题）。
    pub async fn rename_session(&self, id: Uuid, title: &str) -> Result<()> {        let mut s = self
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

    /// 列出某项目下的会话（按项目隔离，用于历史会话侧栏）。
    pub async fn list_sessions_by_project(&self, project_id: Uuid) -> Result<Vec<AgentSession>> {
        self.sessions.list_by_project(project_id).await
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

    /// 解析当前生效的基座：优先指定 scope（如 project:<uuid>），否则 global，再否则内置默认。
    async fn resolve_base(&self, scope: &str) -> Result<String> {
        if let Some(cfg) = self.prompt_store.load(scope).await? {
            return Ok(cfg.system_prompt);
        }
        if let Some(cfg) = self.prompt_store.load("global").await? {
            return Ok(cfg.system_prompt);
        }
        Ok(self.default_system_prompt.clone())
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
        let memories: Vec<String> = self
            .memory
            .list(session.project_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| format!("[{}] {}", m.memory_type, m.content))
            .collect();

        let scope = format!("project:{}", session.project_id);
        let step = self.current_step_of(session.project_id).await?;
        let system = build_system_prompt(
            &self.resolve_base(&scope).await?,
            &step,
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

        let config = self
            .ai_settings
            .load()
            .await
            .context("读取运行时 AI 配置失败")?;
        let reply = self
            .llm
            .complete(
                &system,
                &user_prompt,
                config.model_for(GenerationPurpose::Agent),
                config.temperature_for(GenerationPurpose::Agent),
            )
            .await?;

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

        session.updated_at = Utc::now();
        // 先落库 user 消息（助手/工具消息在流闭包内按迭代逐步落库）
        self.sessions.update(session.clone()).await?;

        // 克隆 Arc，移入流闭包，使返回的流为 'static + Send。
        let llm = self.llm.clone();
        let ai_settings = self.ai_settings.clone();
        let sessions = self.sessions.clone();
        let memory = self.memory.clone();
        // 单独留一份给流闭包里的诊断读取（闭包要 'static，不能借用 self）
        let settings = self.ai_settings.clone();
        // 把已加载的会话直接移入流闭包：流内不再回查，避免竞态导致助手消息漏存
        let mut session = session;
        // 自动记忆捕获所需的上下文（移入流闭包）
        let answered_question = answered_question;
        let answer_text = answer_text;
        let session_key = session.project_id;

        // 提示词基座与工具列表在流外确定一次（owned，避免闭包捕获 &self）
        let base = self.resolve_base(&format!("project:{}", session.project_id)).await?;
        // 引导阶段也在流外确定一次：一次对话内不会变化，且必须以项目级进度为准
        let step = self.current_step_of(session.project_id).await?;
        let tools = self.tools.clone();
        let tool_list = tools.list();

        let s = stream! {
            const MARKER: &str = "<<ASK_QUESTION>>";
            const END: &str = "<<END>>";
            const TOOL_MARKER: &str = "<<CALL_TOOL>>";
            const TOOL_RESULT_MARKER: &str = "<<TOOL_RESULT>>";
            // 单轮对话内允许的工具调用次数上限。
            //
            // 这是一道**安全阀**（防止模型陷入无意义循环把 token 烧光），不是产品功能限制：
            // 批量创建十几个地点是常见用法，每次创建算一次调用，因此上限必须留足余量。
            // 用户随时可以用界面上的「停止」按钮主动中断，达到此上限时本轮也会正常收尾
            // （已完成的工具调用均已落库），再发一条消息即可接着做。
            const MAX_TOOL_ITERS: usize = 50;
            // 模型使用不支持的 XML / DSML 工具调用格式时，允许它收到错误后自我修正的次数。
            const MAX_FORMAT_RETRIES: usize = 3;
            // 工具调用 JSON 不完整时，允许模型收到错误后自我修正的次数；超过就停止本轮，避免无限空转。
            const MAX_PARSE_RETRIES: usize = 3;
            // 正文下发的内部分隔标记（用于判断"末尾是否可能是标记前缀"）。
            const STREAM_MARKERS: [&str; 2] = [TOOL_MARKER, MARKER];

            let mut tool_iters: usize = 0;
            let mut format_retries: usize = 0;
            let mut parse_retries: usize = 0;

            // 本轮对话使用的模型与温度：启动时不固化，每次对话前从设置页读取。
            // 用途固定为 Agent（引导对话 + 细纲落库）：它要的是逻辑与稳定，
            // 不是文采——所以可以和"正文生成"用不同的模型与温度。
            let (model, temperature) = match ai_settings.load().await {
                Ok(config) => (
                    config.model_for(GenerationPurpose::Agent).to_string(),
                    config.temperature_for(GenerationPurpose::Agent),
                ),
                Err(e) => {
                    yield Ok(AgentStreamEvent::Error(format!("读取运行时 AI 配置失败：{}", e)));
                    return;
                }
            };

            loop {
                // 每次迭代重建提示词与历史（含已积累的 tool 消息），让模型看到上一轮工具结果
                let memories: Vec<String> = memory
                    .list(session.project_id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|m| format!("[{}] {}", m.memory_type, m.content))
                    .collect();
                let system = build_system_prompt(&base, &step, &tool_list, &memories);
                let history = session
                    .messages
                    .iter()
                    .filter_map(|m| {
                        let body = if m.role == "tool" {
                            match parse_tool_result(&m.content) {
                                Some((name, ok, output)) => format!(
                                    "工具 {} 执行{}：{}",
                                    name,
                                    if ok { "成功" } else { "失败" },
                                    output
                                ),
                                None => m.content.clone(),
                            }
                        } else if m.role == "assistant" {
                            // 历史里可能残留旧版本未清理的 XML / DSML 工具调用：
                            // 回灌给模型前先剥掉，避免继续污染上下文、强化错误格式。
                            strip_xml_tool_calls(&m.content)
                        } else {
                            m.content.clone()
                        };
                        let body = body.trim();
                        if body.is_empty() {
                            None
                        } else {
                            Some(format!("{}：{}", m.role, body))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut llm_stream = match llm
                    .stream_complete(&system, &history, &model, temperature)
                    .await
                {
                    Ok(st) => st,
                    Err(e) => {
                        yield Ok(AgentStreamEvent::Error(e.to_string()));
                        return;
                    }
                };

                let mut acc = String::new();
                let mut in_q = false;
                let mut qjson = String::new();
                let mut in_tool = false;
                let mut tjson = String::new();
                let mut question: Option<(String, Vec<String>)> = None;
                let mut tool_json: Option<String> = None;
                // `acc` 中已下发给前端的字节数（正文增量发送的游标）。
                let mut emitted = 0usize;
                // 本轮流里最后一次用量统计（含缓存命中），流结束时下发给前端。
                let mut last_usage: Option<LlmUsage> = None;
                // 流结束原因；`length` 表示输出撞到 max_tokens，JSON 很可能被截断。
                let mut last_finish_reason: Option<String> = None;

                while let Some(res) = llm_stream.next().await {
                    match res {
                        // 用量统计：记录即可，不参与正文解析
                        Ok(LlmStreamChunk::Usage(usage)) => {
                            last_usage = Some(usage);
                        }
                        // 结束原因只记录，正文/工具解析结束后再决定如何提示
                        Ok(LlmStreamChunk::Finish(reason)) => {
                            last_finish_reason = Some(reason);
                        }
                        Ok(LlmStreamChunk::Token(tok)) => {
                            acc.push_str(&tok);
                            // 优先识别工具调用标记；其次选择题标记；否则按 token 透传
                            if !in_q && !in_tool {
                                if let Some(pos) = acc.find(TOOL_MARKER) {
                                    // 标记之前的正文照常下发，标记本身绝不下发
                                    if pos > emitted {
                                        yield Ok(AgentStreamEvent::Token(
                                            acc[emitted..pos].to_string(),
                                        ));
                                        emitted = pos;
                                    }
                                    in_tool = true;
                                    tjson = acc[pos + TOOL_MARKER.len()..].to_string();
                                } else if let Some((start, end)) =
                                    find_fullwidth_marker(&acc, "CALL_TOOL")
                                {
                                    // 模型把标记写成了全角竖线形态（｜｜CALL_TOOL｜｜）：
                                    // 同样按工具调用处理，并让游标跳过这段伪标记（绝不下发）。
                                    if start > emitted {
                                        yield Ok(AgentStreamEvent::Token(
                                            acc[emitted..start].to_string(),
                                        ));
                                    }
                                    emitted = end;
                                    in_tool = true;
                                    tjson = acc[end..].to_string();
                                } else if let Some(pos) = acc.find(MARKER) {
                                    if pos > emitted {
                                        yield Ok(AgentStreamEvent::Token(
                                            acc[emitted..pos].to_string(),
                                        ));
                                        emitted = pos;
                                    }
                                    in_q = true;
                                    qjson = acc[pos + MARKER.len()..].to_string();
                                } else if let Some((start, end)) =
                                    find_fullwidth_marker(&acc, "ASK_QUESTION")
                                {
                                    // 同上：全角竖线形态的选择题标记
                                    if start > emitted {
                                        yield Ok(AgentStreamEvent::Token(
                                            acc[emitted..start].to_string(),
                                        ));
                                    }
                                    emitted = end;
                                    in_q = true;
                                    qjson = acc[end..].to_string();
                                } else {
                                    // 无标记：只下发「确定不可能属于标记前缀」的部分。
                                    // 标记常被模型分片成多个 token（如 `<<CAL` + `L_TOOL>>`），
                                    // 直接透传会把前半截泄漏到聊天界面。
                                    let safe_end = floor_char_boundary(
                                        &acc,
                                        acc.len().saturating_sub(marker_prefix_hold(
                                            &acc,
                                            &STREAM_MARKERS,
                                        )),
                                    );
                                    if safe_end > emitted {
                                        yield Ok(AgentStreamEvent::Token(
                                            acc[emitted..safe_end].to_string(),
                                        ));
                                        emitted = safe_end;
                                    }
                                }
                            }
                            if in_tool {
                                if let Some(pos) = acc.find(TOOL_MARKER) {
                                    tjson = acc[pos + TOOL_MARKER.len()..].to_string();
                                }
                                if tjson.contains(END) {
                                    tool_json =
                                        Some(tjson[..tjson.find(END).unwrap()].trim().to_string());
                                    break;
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
                                            yield Ok(AgentStreamEvent::Error(format!(
                                                "保存会话失败: {}",
                                                e
                                            )));
                                        }
                                        yield Ok(AgentStreamEvent::Question {
                                            question: q,
                                            options: opts,
                                        });
                                        break;
                                    } else {
                                        // 解析失败：退回文本（回放已收集内容）
                                        yield Ok(AgentStreamEvent::Token(qjson.clone()));
                                        emitted = acc.len();
                                        in_q = false;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            yield Ok(AgentStreamEvent::Error(e.to_string()));
                            return;
                        }
                    }
                }

                // 流结束：补发此前为防标记泄漏而暂留的正文
                if !in_tool && !in_q && emitted < acc.len() {
                    yield Ok(AgentStreamEvent::Token(acc[emitted..].to_string()));
                }

                // 流结束：未闭合的工具标记也强制进入工具处理（解析会失败并回灌错误）
                if in_tool {
                    tool_json = tool_json.or(Some(tjson.trim().to_string()));
                }

                // 工具调用：执行（含入参校验），结果/错误作为 tool 消息回灌模型后继续循环
                if let Some(json_str) = tool_json {
                    tool_iters += 1;
                    if tool_iters > MAX_TOOL_ITERS {
                        // 安全阀触发：本轮正常收尾，已完成的调用都已落库。
                        // 提示里说明"如何继续"，避免用户以为整轮白做了。
                        yield Ok(AgentStreamEvent::Error(format!(
                            "本轮工具调用已达上限（{} 次安全阀）。已完成的改动均已保存；\
                             如需继续，直接再发一条消息即可（例如「继续」）。",
                            MAX_TOOL_ITERS
                        )));
                        break;
                    }
                    let (name, input, ok, output) = match parse_tool_call(&json_str) {
                        Ok((n, i)) => {
                            match tool_execute(&tools, session.project_id, &n, i.clone()).await {
                                Ok(v) => (
                                    n,
                                    i,
                                    true,
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                ),
                                Err(e) => (n, i, false, e.to_string()),
                            }
                        }
                        Err(e) => {
                            parse_retries += 1;

                            // 把真正的诊断信息返回给 AI 和用户，而不是只写日志、
                            // 更不是所有错误都写“输出过长”。
                            //
                            // 两条最容易被误判的信息必须显式标注：
                            // 1. 本次请求实发的输出上限——没有它就无法判断「是不是被截断」；
                            // 2. finish_reason 缺失——网关没给这个字段时，截断与连接中断
                            //    在数据上无法区分（实测某次 completion_tokens=0 且
                            //    finish_reason=unknown，JSON 只有 3358 字节，远没到上限）。
                            let finish_reason = last_finish_reason.clone();
                            let completion_tokens = last_usage
                                .as_ref()
                                .map(|u| u.completion_tokens);
                            let max_output_tokens = settings
                                .load()
                                .await
                                .map(|c| c.max_output_tokens.to_string())
                                .unwrap_or_else(|e| format!("读取失败：{}", e));
                            let tail: String = {
                                let chars: Vec<char> = json_str.chars().collect();
                                chars[chars.len().saturating_sub(180)..].iter().collect()
                            };
                            // 字节数与字符数都给：中文一个字 3 字节，只看字节数会把
                            // 「已经写了 1000 字」误读成「才 3000，离上限还远」。
                            let json_bytes = json_str.len();
                            let json_chars = json_str.chars().count();
                            tracing::warn!(
                                finish_reason = ?finish_reason,
                                completion_tokens = ?completion_tokens,
                                max_output_tokens = %max_output_tokens,
                                json_bytes = json_bytes,
                                json_chars = json_chars,
                                json_tail = %tail,
                                "工具调用 JSON 解析失败"
                            );

                            // 返回原始诊断，不做解释、不做美化：
                            // 用户和 AI 需要看到原始错误、finish_reason、长度和尾部，
                            // 由他们决定怎么反馈/修正。
                            append_tool_error_log(&serde_json::json!({
                                "time": Utc::now().to_rfc3339(),
                                "session_id": session.id.to_string(),
                                "model": model.clone(),
                                "raw_error": e.to_string(),
                                "finish_reason": finish_reason.clone(),
                                "completion_tokens": completion_tokens,
                                "max_output_tokens": max_output_tokens,
                                "json_length_bytes": json_str.len(),
                                "json_length_chars": json_str.chars().count(),
                                "json_full": json_str.clone(),
                            }));

                            let mut message = format!(
                                "工具调用 JSON 解析失败（原始错误，未美化）：\n\
                                 raw_error: {}\n\
                                 finish_reason: {}\n\
                                 completion_tokens: {}\n\
                                 max_output_tokens（本次请求实发的输出上限，1 token ≈ 1 个汉字）: {}\n\
                                 json_length（字节 / 字符）: {} 字节 / {} 字符\n\
                                 json_tail: {}\n\
                                 完整 JSON 已写入日志：tmp/agent_tool_errors.jsonl",
                                e,
                                match &finish_reason {
                                    Some(r) => r.clone(),
                                    None => "（网关未返回该字段）".to_string(),
                                },
                                match completion_tokens {
                                    Some(n) => n.to_string(),
                                    None => "（网关未返回用量）".to_string(),
                                },
                                max_output_tokens,
                                json_bytes,
                                json_chars,
                                tail,
                            );
                            // 把「能不能据此判定截断」直接讲清楚，省掉一轮误判
                            match finish_reason.as_deref() {
                                Some("length") => message.push_str(
                                    "\n判定：输出确实撞到了 max_output_tokens 上限（finish_reason=length）——\
                                     请缩小单次输出的内容量，或分多次调用（长文本可分次追加写入）。",
                                ),
                                Some(_) => message.push_str(
                                    "\n判定：不是截断（finish_reason 不是 length），问题在模型输出的 JSON 本身。",
                                ),
                                None => message.push_str(
                                    "\n判定：本次**已完整收到 <<END>> 结束标记**（没收到就不会进入解析这一步），\
                                     所以不是网关中途断流；usage / finish_reason 分片排在 <<END>> 之后，\
                                     本轮没有继续采集，因此这两个字段为空**并不代表截断**。\
                                     请直接看 raw_error 里的字符级定位与结构诊断：\
                                     若诊断为「某处少了闭合符」，就是模型算错了括号层级，\
                                     把这一批拆小或把嵌套压平后重发即可。",
                                ),
                            }
                            if parse_retries >= MAX_PARSE_RETRIES {
                                message.push_str("\n已连续多次失败，本轮将停止自动重试。");
                            }

                            (
                                "tool_call_json_error".to_string(),
                                serde_json::json!({}),
                                false,
                                message,
                            )
                        }
                    };
                    // 持久化工具结果（前端重载时按 <<TOOL_RESULT>> 标记渲染为工具卡片）
                    let result_content = format!(
                        "{}{}{}",
                        TOOL_RESULT_MARKER,
                        serde_json::json!({ "name": name, "input": input, "ok": ok, "output": output }),
                        END
                    );
                    session.messages.push(ChatMessage {
                        role: "tool".into(),
                        content: result_content,
                        created_at: Utc::now(),
                    });
                    if let Err(e) = sessions.update(session.clone()).await {
                        yield Ok(AgentStreamEvent::Error(format!("保存会话失败: {}", e)));
                    }
                    yield Ok(AgentStreamEvent::Tool {
                        name,
                        input,
                        ok,
                        output,
                    });

                    // JSON 连续不完整时不要无限重试：给用户一个明确收尾，避免一直刷错误卡片。
                    if parse_retries >= MAX_PARSE_RETRIES {
                        yield Ok(AgentStreamEvent::Error(format!(
                            "AI 连续 {} 次生成不完整的工具调用 JSON，本轮已停止自动重试。\
                             请拆小任务后重试；若反复出现，需要根据后端日志检查模型/网关是否稳定。",
                            MAX_PARSE_RETRIES
                        )));
                        break;
                    }
                    continue;
                }

                // 选择题：已在流内落库并下发，此处仅收尾
                if let Some((q, opts)) = question {
                    let _ = (q, opts);
                    break;
                }

                // 普通文本：剥离模型自带的 XML / DSML 风格工具调用。
                let (stripped, had_unsupported_call) = strip_xml_tool_calls_with_flag(&acc);
                let text = stripped.trim().to_string();

                if had_unsupported_call {
                    format_retries += 1;
                    let (attempted_name, attempted_input) = extract_unsupported_tool_call(&acc);
                    let output = format!(
                        "工具调用格式错误：检测到不受支持的 XML/DSML 调用格式，因此本次没有执行。\
                         请改用 <<CALL_TOOL>>{{\"name\":\"工具名\",\"input\":{{...}}}}<<END>> 重新输出；\
                         不要使用 <invoke>、<parameter>、<calls> 或 \u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C} 这类标签。{}",
                        if attempted_name.is_empty() || attempted_name == "tool_format_error" {
                            String::new()
                        } else {
                            format!("看起来你原本想调用 `{}`。", attempted_name)
                        }
                    );
                    let display_name = if attempted_name.is_empty() {
                        "tool_format_error".to_string()
                    } else {
                        attempted_name.clone()
                    };

                    // 模型在坏调用之外写的正文仍然保留，方便用户理解上下文。
                    if !text.is_empty() {
                        session.messages.push(ChatMessage {
                            role: "assistant".into(),
                            content: text.clone(),
                            created_at: Utc::now(),
                        });
                    }

                    // 以 `tool` 消息回灌失败结果：下一轮模型会看到具体错误并自我修正。
                    let result_content = format!(
                        "{}{}{}",
                        TOOL_RESULT_MARKER,
                        serde_json::json!({
                            "name": display_name.clone(),
                            "input": attempted_input.clone(),
                            "ok": false,
                            "output": output.clone(),
                        }),
                        END
                    );
                    session.messages.push(ChatMessage {
                        role: "tool".into(),
                        content: result_content,
                        created_at: Utc::now(),
                    });
                    if let Err(e) = sessions.update(session.clone()).await {
                        yield Ok(AgentStreamEvent::Error(format!("保存会话失败: {}", e)));
                    }
                    yield Ok(AgentStreamEvent::Tool {
                        name: display_name,
                        input: attempted_input,
                        ok: false,
                        output,
                    });

                    if format_retries > MAX_FORMAT_RETRIES {
                        yield Ok(AgentStreamEvent::Error(format!(
                            "AI 多次使用不受支持的工具调用格式，本轮已停止自动重试。\
                             请再发一条消息让它重试，或检查模型输出格式。"
                        )));
                        break;
                    }
                    continue;
                }

                if !text.is_empty() {
                    session.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content: text,
                        created_at: Utc::now(),
                    });
                    if let Err(e) = sessions.update(session.clone()).await {
                        yield Ok(AgentStreamEvent::Error(format!("保存会话失败: {}", e)));
                    }
                }
                // 先下发用量统计（含缓存命中），再宣告结束——前端在 done 时即可展示
                if let Some(usage) = last_usage {
                    yield Ok(AgentStreamEvent::Usage(usage));
                }
                yield Ok(AgentStreamEvent::Done);
                break;
            }

            // 统一落库（user + assistant/tool），失败则上报错误事件
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
        project_id: Uuid,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        tool_execute(&self.tools, project_id, name, input).await
    }

    pub async fn remember(&self, project_id: Uuid, memory_type: &str, content: &str) -> Result<()> {
        self.memory.save(project_id, memory_type, content).await
    }

    pub async fn recall(&self, project_id: Uuid) -> Result<Vec<MemoryItem>> {
        self.memory.list(project_id).await
    }

    /// 读取项目当前的滚动摘要（从未收尾过时为 `None`）。
    pub async fn load_summary(
        &self,
        project_id: Uuid,
    ) -> Result<Option<StoredSessionSummary>> {
        self.summaries.load(project_id).await
    }

    /// 会话收尾：把整段会话归纳成结构化摘要，**覆盖**项目那一份，并同步一份到
    /// `agent_memory`。
    ///
    /// 为什么两处都写：
    /// - `session_summary` 是权威的「当前状态」快照，供界面展示与回读；
    /// - `agent_memory`（memory_type = `session_summary`）是注入通道——现有
    ///   `build_system_prompt` 会把项目记忆注入系统提示词，新会话因此自动读到
    ///   「上一次聊到哪」。这里直接覆盖而不是新增碎片，避免记忆无限膨胀。
    pub async fn summarize_session(&self, session_id: Uuid) -> Result<StoredSessionSummary> {
        let session = self
            .sessions
            .get(session_id)
            .await?
            .with_context(|| format!("session not found: {}", session_id))?;

        let previous = self
            .summaries
            .load(session.project_id)
            .await?
            .map(|s| s.content);

        let transcript = session
            .messages
            .iter()
            .filter_map(|m| {
                let body = m.content.trim();
                if body.is_empty() {
                    return None;
                }
                let role = match m.role.as_str() {
                    "user" => "用户",
                    "assistant" => "助手",
                    "tool" => "工具",
                    other => other,
                };
                Some(format!("{}：{}", role, body))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = crate::summary::build_summary_prompt(previous.as_ref(), &transcript)?;

        let config = self
            .ai_settings
            .load()
            .await
            .context("读取运行时 AI 配置失败")?;
        let raw = self
            .llm
            .complete(
                crate::summary::SUMMARY_SYSTEM_PROMPT,
                &prompt,
                config.model_for(GenerationPurpose::Utility),
                config.temperature_for(GenerationPurpose::Utility),
            )
            .await
            .context("生成会话摘要失败")?;

        let summary = crate::summary::parse_summary_response(&raw)?;
        let stored = self.summaries.save(session.project_id, &summary).await?;

        // 注入通道：供之后新开的会话在系统提示词里读到。
        // 用 replace_by_type 而非 save：项目全部记忆都会注入提示词，
        // 追加会在里面堆出多份互相矛盾的摘要。
        let injected = serde_json::to_string(&stored.content)
            .context("序列化摘要用于记忆注入失败")?;
        self.memory
            .replace_by_type(
                session.project_id,
                crate::summary::SUMMARY_MEMORY_TYPE,
                &injected,
            )
            .await?;

        Ok(stored)
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

/// 末尾需要暂留的字节数：`s` 的后缀若可能是某个标记的前缀，这部分就不能立即下发。
///
/// 例：`s` 以 `<<CAL` 结尾时返回 5（`<<CALL_TOOL>>` 以它开头）；
/// 普通正文（不含 `<<`）返回 0，可零延迟下发，因此不影响正常流式观感。
fn marker_prefix_hold(s: &str, markers: &[&str]) -> usize {
    let max_len = markers.iter().map(|m| m.len()).max().unwrap_or(0);
    if max_len <= 1 {
        return 0;
    }
    // 只看末尾 max_len - 1 字节：更长的后缀不可能是某个标记的前缀
    let tail_start = floor_char_boundary(s, s.len().saturating_sub(max_len - 1));
    let tail = &s[tail_start..];

    let mut hold = 0;
    for len in 1..=tail.len() {
        let start = floor_char_boundary(tail, tail.len() - len);
        let suffix = &tail[start..];
        if suffix.len() != len {
            continue; // 落在多字节字符中间
        }
        if markers.iter().any(|m| m.starts_with(suffix)) {
            hold = len;
        }
    }
    hold
}

/// 在累积文本中查找「全角竖线形态」的标记，返回 `(起始字节位置, 结束字节位置)`。
///
/// 部分模型会把 `<<CALL_TOOL>>` 输出成 `｜｜CALL_TOOL｜｜`——`｜` 是 U+FF5C
/// **全角竖线**，在界面上与 `<` `>` 几乎无法分辨，但字节完全不同，
/// 会导致标记识别失败：调用既不执行，内容又被当作正文显示给用户。
fn find_fullwidth_marker(haystack: &str, name: &str) -> Option<(usize, usize)> {
    let needle = format!("\u{FF5C}\u{FF5C}{}\u{FF5C}\u{FF5C}", name);
    haystack.find(&needle).map(|i| (i, i + needle.len()))
}

/// 剥离模型输出中的 XML / DSML 风格工具调用块。
///
/// 部分模型（如 DeepSeek 系）会用 `<invoke name="…"><parameter …/></invoke>` 这类
/// 原生 function-calling 格式，而本项目只解析 `<<CALL_TOOL>>`。这类内容
/// **不会被执行**；若原样落库，下一轮还会作为上下文回灌给模型，反过来强化这种错误格式。
///
/// 除了标准 XML，还要兼容实际出现过的畸形写法：
/// - `<<> calls>` / `</<> invoke>`：模型把 `<>` 当成标签分隔符；
/// - `<\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C} calls>`：DSML 包裹标签（`\u{FF5C}` 是全角竖线）。
///
/// 未闭合的块（流式中途）一并吞到文末。
fn strip_xml_tool_calls(text: &str) -> String {
    strip_xml_tool_calls_with_flag(text).0
}

/// 同 [`strip_xml_tool_calls`]，但同时返回“是否真的检测到不支持的调用格式”。
///
/// 调用方可用这个标记决定要不要给用户补一句“该次调用未执行”的说明，
/// 避免模型用坏格式调用工具时，用户侧看起来像 AI 什么都没做。
fn strip_xml_tool_calls_with_flag(text: &str) -> (String, bool) {
    // 打开标签 → 对应闭合标签。必须成对匹配，不能一律取“最近闭合”，
    // 否则 `<calls><invoke>…</invoke></calls>` 会只吃掉 `</invoke>` 而留下 `</calls>`。
    const BLOCKS: [(&str, &str); 4] = [
        ("<tool_calls>", "</tool_calls>"),
        ("<calls>", "</calls>"),
        ("<invoke", "</invoke>"),
        ("<function_call>", "</function_call>"),
    ];

    let normalized = normalize_malformed_tool_tags(text);
    let had_unsupported = BLOCKS
        .iter()
        .any(|(open, _close)| normalized.contains(open));

    let mut out = String::with_capacity(normalized.len());
    let mut rest = normalized.as_str();

    loop {
        let Some((start, _open, close)) = BLOCKS
            .iter()
            .filter_map(|(open, close)| rest.find(open).map(|i| (i, *open, *close)))
            .min_by_key(|(i, _, _)| *i)
        else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        // 块结束：优先找当前打开标签对应的闭合标签；没有闭合则吞到文末。
        let end = tail
            .find(close)
            .map(|i| i + close.len())
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }

    (strip_orphan_xml_tags(&out), had_unsupported)
}

/// 把模型写坏的标签归一化成普通 XML 形态，方便 `strip_xml_tool_calls` 统一处理。
fn normalize_malformed_tool_tags(text: &str) -> String {
    let mut out = text.to_string();

    // 形态一：`<<> calls>` / `</<> invoke>`（`<>` 被当成分隔符）。
    // 模型通常在 `<<>` 后留一个空格，必须先连同空白一起替换，否则会留下 `< calls>`。
    out = out
        .replace("<<> ", "<")
        .replace("<<>\t", "<")
        .replace("<<>\n", "<")
        .replace("<<>", "<")
        .replace("</<> ", "</")
        .replace("</<>\t", "</")
        .replace("</<>\n", "</")
        .replace("</<>", "</");

    // 形态二：DSML 包裹，如 `<\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C} calls>`。
    // 同时兼容全角竖线和 ASCII `||DSML||`。
    for marker in ["\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C}", "||DSML||"] {
        // 模型通常会在包裹符后留一个空格：`<｜｜DSML｜｜ calls>`。
        out = out.replace(&format!("<{} ", marker), "<");
        out = out.replace(&format!("</{} ", marker), "</");
        // 无空格兜底。
        out = out.replace(&format!("<{}", marker), "<");
        out = out.replace(&format!("</{}", marker), "</");
    }

    out
}

/// 清掉剥离主块后仍残留的孤立 XML 标签（如单独的 `<parameter name="id">`）。
fn strip_orphan_xml_tags(text: &str) -> String {
    const TAGS: [&str; 10] = [
        "<parameter",
        "</parameter>",
        "<arguments",
        "</arguments>",
        "<tool_calls>",
        "</tool_calls>",
        "<calls>",
        "</calls>",
        "<invoke",
        "</invoke>",
    ];

    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    loop {
        let Some((start, _)) = TAGS
            .iter()
            .filter_map(|tag| rest.find(tag).map(|i| (i, *tag)))
            .min_by_key(|(i, _)| *i)
        else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        // 从标签起点删到最近的 `>`；没有 `>`（流式半截）则直接丢弃到文末。
        if let Some(gt) = tail.find('>') {
            rest = &tail[gt + 1..];
        } else {
            rest = "";
        }
    }

    out
}

/// 从 `i` 向左找最近的可切分字符边界（避免切坏多字节字符）。
fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 把工具调用 JSON 失败的**原始记录**追加到 JSONL 日志。
///
/// 默认写到项目根下的 `tmp/agent_tool_errors.jsonl`；可用环境变量
/// `NOVEL_TOOL_ERROR_LOG` 覆盖。写入失败不打断对话（日志只是诊断辅助）。
fn append_tool_error_log(record: &serde_json::Value) {
    use std::io::Write;

    let path = std::env::var("NOVEL_TOOL_ERROR_LOG")
        .unwrap_or_else(|_| "tmp/agent_tool_errors.jsonl".to_string());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{}", record);
    }
}

/// 解析 `<<CALL_TOOL>>` 的 JSON：`{"name":"...","input":{...}}`。
///
/// 复用 `extract_json_object` 容忍多余字符。`name` 必填；`input` 缺省为空对象；
/// `input` 非对象则报错（与 `execute_tool` 的约束一致）。
///
/// **input 兜底（防 LLM 把 JSON schema 原文当 input 传）**：
/// LLM 偶尔会看到工具的 input_schema（包含 `properties` 字段），误以为应该
/// 把所有字段嵌套在 `properties` 里。检测到 input 形如
/// `{ "type": "object", "properties": {...}, "required": [...] }` 时，
/// 自动 unwrap 成 `{ ...properties }` 形式，并保留 project_id 注入。
fn parse_tool_call(s: &str) -> Result<(String, serde_json::Value)> {
    let json = extract_json_object(s)
        .ok_or_else(|| anyhow::anyhow!("工具调用 JSON 解析失败"))?;

    let v: serde_json::Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(first) => {
            let first_msg = first.to_string();
            // 模型非常常见的坏习惯：JSON 结尾少一个/多个右括号。
            // 只在 EOF 且能用栈扫描确定缺失的闭合符时做“补齐”，不改动已有结构。
            if first_msg.contains("EOF while parsing") {
                if let Some(repaired) = repair_incomplete_json(&json) {
                    match serde_json::from_str::<serde_json::Value>(&repaired) {
                        Ok(v) => {
                            tracing::warn!(
                                original_len = json.len(),
                                repaired_len = repaired.len(),
                                "自动补齐工具调用 JSON 缺失的右括号"
                            );
                            v
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("工具调用 JSON 解析失败: {}", e));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("工具调用 JSON 解析失败: {}", first_msg));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "工具调用 JSON 解析失败: {}\n{}",
                    first_msg,
                    json_error_diagnosis(&json, &first)
                ));
            }
        }
    };

    finish_parse_tool_value(v)
}

/// 解析失败时给出**字符级**定位与结构诊断。
///
/// 为什么需要：serde_json 报的 `column` 是**字节**偏移，中文报文里它会明显大于
/// 人眼数出来的字符位置（实测 867 字符的 batch_call 报 column 759，而真正的缺陷
/// 在字符 429 附近），照着报错位置去看根本看不到问题点。
///
/// 这里换算成字符列、给出出错点前后各 40 字，并用栈扫描判断「是不是少闭合符号、
/// 少几个」。刻意**不做自动补齐**：缺陷在 JSON 中间时，补错位置会静默写出结构不对的
/// 数据（例如 attributes 少一层），比报错难查得多——诊断给足，让模型自己重发。
fn json_error_diagnosis(json: &str, err: &serde_json::Error) -> String {
    let byte_col = err.column();
    let prefix = json
        .as_bytes()
        .get(..byte_col.saturating_sub(1))
        .unwrap_or_default();
    let char_col = String::from_utf8_lossy(prefix).chars().count() + 1;

    let chars: Vec<char> = json.chars().collect();
    let start = char_col.saturating_sub(41);
    let end = (char_col + 40).min(chars.len());
    let snippet: String = chars[start..end].iter().collect();

    // 括号栈扫描（忽略字符串字面量里的括号）
    let mut opens: Vec<(char, usize)> = Vec::new();
    let mut extra_closes = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in chars.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if *ch == '\\' {
                escaped = true;
            } else if *ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' | '[' => opens.push((*ch, i + 1)),
            '}' => {
                if matches!(opens.last(), Some(('{', _))) {
                    opens.pop();
                } else {
                    extra_closes += 1;
                }
            }
            ']' => {
                if matches!(opens.last(), Some(('[', _))) {
                    opens.pop();
                } else {
                    extra_closes += 1;
                }
            }
            _ => {}
        }
    }

    let mut out = format!(
        "出错位置：第 {} 个字符（serde 报的是**字节**列 {}，含中文时两者不同）\n出错点附近：{}…\n",
        char_col, byte_col, snippet
    );
    if opens.is_empty() && extra_closes == 0 {
        out.push_str(
            "结构诊断：括号是配平的，缺陷多半在**逗号 / 引号 / 键名**\
             （例如对象里漏写键名，或两个值之间少了逗号）。\n",
        );
    } else {
        if !opens.is_empty() {
            let detail: Vec<String> = opens
                .iter()
                .take(3)
                .map(|(ch, at)| format!("{}（第 {} 个字符处开启）", ch, at))
                .collect();
            let missing: String = opens
                .iter()
                .rev()
                .map(|(ch, _)| match ch {
                    '{' => '}',
                    '[' => ']',
                    other => *other,
                })
                .collect();
            out.push_str(&format!(
                "结构诊断：有 {} 个容器没有闭合——{}。缺少的闭合符是「{}」。\
                 这类缺陷往往**不在报文末尾**，框架只在末尾补括号，所以这次不会自动修。\
                 请把这一批拆小（或把嵌套压平）后重发本次调用。\n",
                opens.len(),
                detail.join(" / "),
                missing
            ));
        }
        if extra_closes > 0 {
            out.push_str(&format!(
                "结构诊断：另有 {} 个多余的闭合符（多打了 }} 或 ]）。\n",
                extra_closes
            ));
        }
    }
    out
}

fn finish_parse_tool_value(v: serde_json::Value) -> Result<(String, serde_json::Value)> {
    let name = v
        .get("name")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow::anyhow!("工具调用缺少 name 字段"))?
        .to_string();
    let input = v.get("input").cloned().unwrap_or(serde_json::json!({}));
    let input = unwrap_schema_shaped_input(input);
    if !input.is_object() {
        anyhow::bail!("工具调用 input 必须为 JSON 对象");
    }
    Ok((name, input))
}

/// 对“JSON 对象已结束，只是外层少右括号”的常见情况做最小修复：
/// 用栈扫描字符串（忽略字符串字面量里的括号），只补缺失的 `}` / `]`。
/// 不改动已有内容、不修缺少逗号/引号这类无法安全推断的错误。
fn repair_incomplete_json(s: &str) -> Option<String> {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in s.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' | '[' => stack.push(ch),
            '}' => {
                if stack.pop() != Some('{') {
                    return None;
                }
            }
            ']' => {
                if stack.pop() != Some('[') {
                    return None;
                }
            }
            _ => {}
        }
    }

    // 字符串没闭合、或没有缺失闭合符时，不做猜测。
    if in_string || stack.is_empty() {
        return None;
    }

    let mut out = s.trim_end().to_string();
    while out.ends_with(',') {
        out.pop();
    }
    for open in stack.iter().rev() {
        out.push(if *open == '{' { '}' } else { ']' });
    }
    Some(out)
}

/// 检测 input 是不是 LLM 把 JSON schema 原文当 input 传。
/// 形如 `{ "type": "object", "properties": { ...字段 }, "required": [...] }` → 自动取出 properties。
fn unwrap_schema_shaped_input(input: serde_json::Value) -> serde_json::Value {
    let obj = match input.as_object() {
        Some(o) => o,
        None => return input,
    };
    // schema 形态：含 type="object" + properties（且其他字段都属 schema 关键字）
    let is_schema_shape = obj.get("type").and_then(|v| v.as_str()) == Some("object")
        && obj.contains_key("properties")
        && obj.keys().all(|k| matches!(k.as_str(), "type" | "properties" | "required" | "additionalProperties" | "title" | "description"));
    if !is_schema_shape {
        return input;
    }
    // unwrap：取 properties 字段
    match obj.get("properties") {
        Some(p) if p.is_object() => p.clone(),
        _ => input,
    }
}

#[cfg(test)]
mod tests_parse_tool_call {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normal_input_passes_through() {
        // 普通业务 input：{ premise: "..." } → 原样
        let input = json!({"premise": "hello"});
        let out = unwrap_schema_shaped_input(input.clone());
        assert_eq!(out, input);
    }

    #[test]
    fn test_schema_shaped_input_gets_unwrapped() {
        // LLM 把 schema 原文当 input 传
        let schema = json!({
            "type": "object",
            "properties": {"premise": "hello world"},
            "required": ["premise"]
        });
        let out = unwrap_schema_shaped_input(schema);
        assert_eq!(out, json!({"premise": "hello world"}));
    }

    #[test]
    fn test_normal_object_with_type_field_not_unwrapped() {
        // 业务字段里恰好有 "type" 字段（不应误判）
        let v = json!({"type": "Faction", "name": "x"});
        let out = unwrap_schema_shaped_input(v.clone());
        assert_eq!(out, v);
    }

    #[test]
    fn repairs_missing_outer_brace() {
        // 数据库里真实出现过：外层对象少一个右花括号。
        let broken = r#"{"name":"update_character_profile","input":{"id":"x","capabilities":{"skills":["a"],"limitations":["b"]}}"#;
        let repaired = repair_incomplete_json(broken).expect("应能补齐右括号");
        let v: serde_json::Value = serde_json::from_str(&repaired).expect("补齐后应是合法 JSON");
        assert_eq!(v["input"]["capabilities"]["skills"][0], "a");
    }

    #[test]
    fn does_not_repair_unterminated_string() {
        // 字符串没闭合时无法安全猜测，应拒绝修复。
        assert!(repair_incomplete_json("{\"a\":\"").is_none());
    }

    #[test]
    fn diagnoses_missing_brace_in_the_middle() {
        // 实测样本（batch_call：嵌套 attributes + 多工具）：`attributes` 少一个 `}`。
        // 正确结构需要 3 个 `}`（beats 数组 → attributes → 子调用），样本里只有 2 个，
        // 于是报错落在**报文中间**（`key must be a string`），旧的末尾补括号逻辑不触发。
        let broken = r#"{"name":"batch_call","input":{"calls":[{"tool":"create_node","input":{"title":"卷一","attributes":{"beats":["a"]}},{"tool":"get_node","input":{"id":"x"}}]}}"#;
        let err = parse_tool_call(broken).unwrap_err().to_string();
        assert!(err.contains("工具调用 JSON 解析失败"), "{}", err);
        assert!(
            err.contains("没有闭合"),
            "诊断应指出有容器没闭合，实际：{}",
            err
        );
        assert!(
            err.contains("第 ") && err.contains("个字符"),
            "诊断应给出字符级位置，实际：{}",
            err
        );
    }
}

/// 从历史回读 `<<TOOL_RESULT>>...<<END>>` 标记，转换为模型可读文本（避免把标记原样喂给模型）。
fn parse_tool_result(content: &str) -> Option<(String, bool, String)> {
    const M: &str = "<<TOOL_RESULT>>";
    const END: &str = "<<END>>";
    if !content.starts_with(M) {
        return None;
    }
    let inner = &content[M.len()..];
    let end = inner.find(END)?;
    let v: serde_json::Value = serde_json::from_str(&inner[..end]).ok()?;
    let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
    let output = v.get("output").and_then(|x| x.as_str()).unwrap_or("").to_string();
    Some((name, ok, output))
}

/// 在流闭包内执行工具（不依赖 `&self`，仅用已克隆的 `ToolRegistry`）。
///
/// 逻辑与 `AgentRuntime::execute_tool` 一致：先注入 / 覆盖 `project_id`
/// （物理隔离），再交给注册表做「查表 + JSON Schema 校验 + 调用」。
/// 注入与校验都实现在 `tool` 模块里，`batch_call` 的子调用调用同一对函数，
/// 因此两条路径的行为一致，不存在「批处理里少了注入」的旁路。
async fn tool_execute(
    tools: &Arc<ToolRegistry>,
    project_id: Uuid,
    name: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value> {
    tools.execute_in_project(project_id, name, input).await
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

/// 从不受支持的 XML / DSML 工具调用文本里，尽量提取「模型原本想调用的工具名和入参」。
///
/// 只用于把错误写得更清楚并回灌给模型，不会真的执行这个坏格式调用。
fn extract_unsupported_tool_call(text: &str) -> (String, serde_json::Value) {
    let normalized = normalize_malformed_tool_tags(text);

    // 形态一：DSML 里常带一段完整 JSON：<invoke name="CALL_TOOL">{"name":"get_entity","input":{...}}
    if let Some(json) = extract_json_object(&normalized) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(name) = value.get("name").and_then(|x| x.as_str()) {
                let name = name.trim();
                if !name.is_empty() {
                    let input = value
                        .get("input")
                        .cloned()
                        .filter(|v| v.is_object())
                        .unwrap_or_else(|| serde_json::json!({}));
                    return (name.to_string(), input);
                }
            }
        }
    }

    // 形态二：原生 XML function-call：<invoke name="get_entity"><parameter ...>
    if let Some(idx) = normalized.find("<invoke") {
        let tail = &normalized[idx..];
        if let Some(attr_start) = tail.find("name=\"") {
            let rest = &tail[attr_start + 6..];
            if let Some(end) = rest.find('"') {
                let name = rest[..end].trim();
                if !name.is_empty() && name != "CALL_TOOL" {
                    return (name.to_string(), serde_json::json!({}));
                }
            }
        }
    }

    ("tool_format_error".to_string(), serde_json::json!({}))
}

/// 轻量 JSON Schema 校验：检查 `required` 字段存在且非空，并按 `properties` 中的
/// `type` 做基础类型校验（string/object/array/number/boolean）。不做深层结构校验。
/// 轻量 JSON Schema 校验（required + type + enum）。
///
/// `pub(crate)`：`tool::ToolRegistry::execute` 复用它，保证批量调用与单次调用
/// 走同一套校验，不给 batch_call 留绕过校验的旁路。
pub(crate) fn validate_input(input: &serde_json::Value, schema: &serde_json::Value) -> Result<()> {
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
                match spec.get("type") {
                    // 单一类型
                    Some(serde_json::Value::String(t)) => {
                        if !type_matches(v, t) {
                            anyhow::bail!("字段 '{}' 应为类型 {}", k, t);
                        }
                    }
                    // 多类型（如 ["string", "object"]）：满足任一即可。
                    // 有些字段本就允许"简单写法或结构化写法"两种形态，
                    // 若只声明一种，模型的另一种合理写法会被硬拒（实测踩到过）。
                    Some(serde_json::Value::Array(types)) => {
                        let names: Vec<&str> = types.iter().filter_map(|t| t.as_str()).collect();
                        if !names.is_empty() && !names.iter().any(|t| type_matches(v, t)) {
                            anyhow::bail!("字段 '{}' 应为类型 {}", k, names.join(" 或 "));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// 单个 JSON 值是否符合给定的类型名。
fn type_matches(v: &serde_json::Value, t: &str) -> bool {
    match t {
        "string" => v.is_string(),
        "object" => v.is_object(),
        "array" => v.is_array(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "integer" => v.is_i64() || v.is_u64(),
        "null" => v.is_null(),
        _ => true,
    }
}

#[cfg(test)]
mod stream_marker_tests {
    use super::*;

    const MARKERS: [&str; 2] = ["<<CALL_TOOL>>", "<<ASK_QUESTION>>"];

    #[test]
    fn plain_text_is_never_held_back() {
        // 普通正文（不含 `<<`）可以零延迟下发
        assert_eq!(marker_prefix_hold("你好，我来修改主角", &MARKERS), 0);
        assert_eq!(marker_prefix_hold("hello world", &MARKERS), 0);
        assert_eq!(marker_prefix_hold("", &MARKERS), 0);
    }

    #[test]
    fn partial_marker_is_held_back() {
        // 模型把标记分片成多个 token 时，这些片段绝不能下发
        assert_eq!(marker_prefix_hold("<<", &MARKERS), 2);
        assert_eq!(marker_prefix_hold("我来改<<CAL", &MARKERS), 5);
        assert_eq!(marker_prefix_hold("<<CALL_TOOL>", &MARKERS), 12);
        assert_eq!(marker_prefix_hold("<<ASK_QUES", &MARKERS), 10);
    }

    #[test]
    fn single_angle_bracket_is_not_a_marker_prefix() {
        // 标记都以 `<<` 开头，单个 `<` 不构成前缀
        assert_eq!(marker_prefix_hold("a<b", &MARKERS), 0);
        assert_eq!(marker_prefix_hold("a<x>", &MARKERS), 0);
    }

    #[test]
    fn floor_char_boundary_never_splits_multibyte_chars() {
        let s = "你好"; // 每个汉字 3 字节
        assert_eq!(floor_char_boundary(s, 4), 3);
        assert_eq!(floor_char_boundary(s, 6), 6);
        assert_eq!(floor_char_boundary(s, 99), 6);
        assert!(s.is_char_boundary(floor_char_boundary(s, 4)));
    }

    #[test]
    fn strips_complete_xml_tool_call() {
        let s = "好的，我来修改。\n<tool_calls>\n<invoke name=\"get_entity\">\n<parameter name=\"id\">abc</parameter>\n</invoke>\n</tool_calls>\n";
        let out = strip_xml_tool_calls(s);
        assert!(out.contains("好的，我来修改"));
        assert!(!out.contains("invoke"));
        assert!(!out.contains("parameter"));
    }

    #[test]
    fn strips_unclosed_xml_tool_call_to_end() {
        let s = "正文内容\n<invoke name=\"get_entity\">\n<parameter name=\"id\">abc</parameter>";
        assert_eq!(strip_xml_tool_calls(s).trim(), "正文内容");
    }

    #[test]
    fn keeps_plain_text_with_angle_brackets() {
        let s = "这是一段普通正文，包含 < 和 > 符号。";
        assert_eq!(strip_xml_tool_calls(s), s);
    }

    #[test]
    fn strips_calls_tag_block_without_tool_calls_prefix() {
        // 模型有时省略 <tool_calls>，直接写 <calls>（也是数据库里出现过的形态）。
        let s = "好的\n<calls>\n<invoke name=\"get_entity\">\n<parameter name=\"id\">abc</parameter>\n</invoke>\n</calls>";
        let out = strip_xml_tool_calls(s);
        assert!(out.contains("好的"));
        for leaked in ["calls", "invoke", "parameter", "abc"] {
            assert!(!out.contains(leaked), "不应残留 {leaked}: {out}");
        }
    }

    #[test]
    fn strips_malformed_double_angle_tool_call() {
        // `<<> calls>` / `</<> invoke>`：模型把 `<>` 当成了标签分隔符。
        let s = "重试\n<<> calls>\n<<> invoke name=\"get_entity\">\n<<> parameter name=\"id\">abc</<> parameter>\n</<> invoke>\n</<> calls>";
        let out = strip_xml_tool_calls(s);
        assert!(out.contains("重试"));
        for leaked in ["<<>", "invoke", "parameter", "abc"] {
            assert!(!out.contains(leaked), "不应残留 {leaked}: {out}");
        }
    }

    #[test]
    fn strips_dsml_wrapped_tool_call() {
        // 数据库真实样本：<\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C} calls> ...
        let marker = "\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C}";
        let s = format!(
            "重试\n<{marker} calls>\n<{marker} invoke name=\"get_character_profile\">\n<{marker} parameter name=\"id\" string=\"true\">133b6607-1cc8-4161-92b4-40c54857e9c0</{marker} parameter>\n</{marker} invoke>\n</{marker} calls>"
        );
        let out = strip_xml_tool_calls(&s);
        assert!(out.contains("重试"));
        for leaked in ["DSML", "calls", "invoke", "parameter", "133b6607", "\u{FF5C}"] {
            assert!(!out.contains(leaked), "不应残留 {leaked}: {out}");
        }
    }

    #[test]
    fn extracts_attempted_tool_from_dsml_json() {
        // 坏格式里带完整 JSON 时，应尽量提取出模型原本想调用的工具。
        let marker = "\u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C}";
        let s = format!(
            "<{marker} calls>\n<{marker} invoke name=\"CALL_TOOL\">{{\"name\":\"get_entity\",\"input\":{{\"id\":\"abc\"}}}}"
        );
        let (name, input) = extract_unsupported_tool_call(&s);
        assert_eq!(name, "get_entity");
        assert_eq!(input["id"], "abc");
    }

    #[test]
    fn extracts_attempted_tool_from_xml_invoke() {
        let s = "<calls><invoke name=\"get_character_profile\"><parameter name=\"id\">abc</parameter></invoke></calls>";
        let (name, input) = extract_unsupported_tool_call(s);
        assert_eq!(name, "get_character_profile");
        assert!(input.is_object());
    }

    #[test]
    fn detects_fullwidth_bar_marker() {
        // 模型把 `<<CALL_TOOL>>` 写成了全角竖线形态
        let s = "<\u{FF5C}\u{FF5C}CALL_TOOL\u{FF5C}\u{FF5C}>{\"name\":\"x\"}";
        let (start, end) = find_fullwidth_marker(s, "CALL_TOOL").expect("应识别出全角标记");
        assert_eq!(&s[start..end], "\u{FF5C}\u{FF5C}CALL_TOOL\u{FF5C}\u{FF5C}");
        // 标记之后的 JSON 可以正常取出
        assert!(s[end..].contains("\"name\""));
    }

    #[test]
    fn ignores_ascii_marker_in_fullwidth_lookup() {
        // ASCII 形态不应被全角查找命中（两套检测互相独立）
        assert!(find_fullwidth_marker("<<CALL_TOOL>>{}", "CALL_TOOL").is_none());
    }

    #[test]
    fn ignores_plain_text_with_fullwidth_bars() {
        // 普通中文里的全角竖线不应被误判
        assert!(find_fullwidth_marker("表格｜列｜说明", "CALL_TOOL").is_none());
    }
}
