//! 工具框架（GPT 方案 2.1 / 2.2 / 2.3）
//!
//! 工具是**领域级 Action**，不是 CRUD。Agent 只暴露 name / description /
//! JSON Schema，具体数据库细节由工具实现内部决定。
//!
//! P1 提供示例工具以验证「注册表 + 调用 + Schema」闭环；真实 create_* 工具
//! （接 application service + DB）在 P2 接入。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use crate::types::ToolMeta;

/// Agent 工具接口。
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// 工具名（唯一键）
    fn name(&self) -> String;
    /// 自然语言描述（进入提示词，也用于前端展示）
    fn description(&self) -> String;
    /// 输入 JSON Schema（用于提示词拼接，P2 用于 Schema 校验）
    fn input_schema(&self) -> serde_json::Value;
    /// 执行工具，返回结构化结果。是否落库由实现决定。
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value>;
}

/// 工具注册表。内部用 `RwLock`，支持以 `&self` 运行时注册。
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn AgentTool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, tool: Arc<dyn AgentTool>) {
        if let Ok(mut g) = self.tools.write() {
            g.insert(tool.name(), tool);
        }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AgentTool>> {
        self.tools.read().ok()?.get(name).cloned()
    }

    pub fn list(&self) -> Vec<ToolMeta> {
        self.tools
            .read()
            .map(|g| {
                g.values()
                    .map(|t| ToolMeta {
                        name: t.name(),
                        description: t.description(),
                        input_schema: t.input_schema(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 回显工具：验证「注册表 + 调用 + Schema」闭环，无副作用。
pub struct EchoTool;

#[async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> String {
        "echo".into()
    }
    fn description(&self) -> String {
        "回显输入文本，用于验证工具调用链路。".into()
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(json!({ "echo": text }))
    }
}

/// 提问工具：让 Agent 向用户呈现选择题（≥10 个推荐选项）+ 自由文本输入。
///
/// 注意：在 chat 流中由 AgentRuntime 拦截 `<<ASK_QUESTION>>` 标记并渲染为前端 UI，
/// 不在此处真正"执行"。execute 仅作占位（直接回显输入）。
pub struct AskQuestionTool;

#[async_trait]
impl AgentTool for AskQuestionTool {
    fn name(&self) -> String {
        "ask_question".into()
    }
    fn description(&self) -> String {
        "向用户提出一个选择题，提供若干推荐选项（建议至少 10 个）供用户选择；若都不合适，用户可自由输入。用于需要用户从给定方向中做选择时（如题材、基调、主角设定方向）。".into()
    }
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "question": { "type": "string" },
                "options": { "type": "array", "items": { "type": "string" }, "minItems": 10 }
            },
            "required": ["question", "options"]
        })
    }
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        Ok(input)
    }
}
