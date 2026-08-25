//! Agent 核心层单测：用 Mock LLM + 内存存储，不依赖数据库或外部 LLM。

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use domain::ports::{LlmPort, PromptRepositoryPort};
use uuid::Uuid;

use agent::*;

/// 测试用假 LLM：不触网，回显 user prompt。
struct MockLlm;

#[async_trait]
impl LlmPort for MockLlm {
    async fn complete(&self, _system: &str, user_prompt: &str, _model: &str) -> Result<String> {
        Ok(format!("【模拟回复】收到：{}", user_prompt))
    }
}

fn make_runtime() -> Arc<AgentRuntime> {
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlm);
    let tools = Arc::new(ToolRegistry::new());
    let sessions = Arc::new(InMemorySessionStore::new());
    let memory = Arc::new(InMemoryAgentMemory::new());
    let prompt_store: Arc<dyn PromptRepositoryPort> = Arc::new(InMemoryPromptRepo::new());
    Arc::new(
        AgentRuntime::new(
            llm,
            tools,
            sessions,
            memory,
            prompt_store,
            agent::DEFAULT_SYSTEM_PROMPT_BASE.to_string(),
            "test".into(),
        )
        .with_default_tools(),
    )
}

#[tokio::test]
async fn session_create_and_chat_records_messages() {
    let rt = make_runtime();
    let id = rt.create_session(Uuid::nil()).await.unwrap();
    let reply = rt.chat(id, "我想写一个玄幻小说").await.unwrap();
    assert!(reply.contains("模拟回复"), "reply = {}", reply);

    let s = rt.get_session(id).await.unwrap().expect("session exists");
    assert_eq!(s.messages.len(), 2, "应记录 user + assistant 两条消息");
    assert_eq!(s.messages[0].role, "user");
    assert_eq!(s.messages[1].role, "assistant");
}

#[tokio::test]
async fn chat_unknown_session_errors() {
    let rt = make_runtime();
    let r = rt.chat(Uuid::new_v4(), "hi").await;
    assert!(r.is_err(), "未知会话应报错");
}

#[tokio::test]
async fn tool_execute_echo() {
    let rt = make_runtime();
    let out = rt
        .execute_tool(Uuid::nil(), "echo", serde_json::json!({ "text": "hello" }))
        .await
        .unwrap();
    assert_eq!(out["echo"], "hello");
}

#[tokio::test]
async fn tool_execute_unknown_errors() {
    let rt = make_runtime();
    let r = rt.execute_tool(Uuid::nil(), "nope", serde_json::json!({})).await;
    assert!(r.is_err(), "未知工具应报错");
}

#[tokio::test]
async fn tool_execute_non_object_input_errors() {
    let rt = make_runtime();
    let r = rt.execute_tool(Uuid::nil(), "echo", serde_json::json!("not an object")).await;
    assert!(r.is_err(), "非对象输入应报错");
}

#[tokio::test]
async fn tool_execute_missing_required_field_errors() {
    let rt = make_runtime();
    // echo 的 schema 要求必填字段 "text"，缺失应被 Schema 校验拦截。
    let r = rt.execute_tool(Uuid::nil(), "echo", serde_json::json!({})).await;
    assert!(r.is_err(), "缺少必填字段应报错");
    let msg = format!("{}", r.unwrap_err());
    assert!(msg.contains("text"), "错误信息应指明缺失字段: {}", msg);
}

#[tokio::test]
async fn tool_list_contains_examples() {
    let rt = make_runtime();
    let names: Vec<String> = rt.list_tools().into_iter().map(|t| t.name).collect();
    assert!(names.contains(&"echo".to_string()), "tools = {:?}", names);
    assert!(
        names.contains(&"ask_question".to_string()),
        "tools = {:?}",
        names
    );
}

#[tokio::test]
async fn memory_save_and_recall() {
    let rt = make_runtime();
    let sid = Uuid::nil();
    rt.remember(sid, "用户偏好", "喜欢黑暗奇幻").await.unwrap();
    rt.remember(sid, "故事风格", "严肃史诗").await.unwrap();

    let items = rt.recall(sid).await.unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|m| m.memory_type == "用户偏好"));
    assert!(items.iter().any(|m| m.memory_type == "故事风格"));
}
