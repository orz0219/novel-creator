//! Agent 核心层单测：用 Mock LLM + 内存存储，不依赖数据库或外部 LLM。

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use domain::ports::{
    AiRuntimeConfig, AiSettingsPort, GuideProgressPort, LlmPort, PromptRepositoryPort,
};
use uuid::Uuid;

use agent::*;

/// 测试用假 LLM：不触网，回显 user prompt（经转义，保证输出是合法 JSON）。
///
/// 返回合法 JSON（含 `story_state`），这样同一份 mock 既能伺候聊天断言，
/// 也能让 `summarize_session` 走通解析；`calls` 计数让每次生成的内容不同，
/// 用于验证「再次收尾是覆盖而不是追加」。
#[derive(Default)]
struct MockLlm {
    calls: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl LlmPort for MockLlm {
    async fn complete(
        &self,
        _system: &str,
        user_prompt: &str,
        _model: &str,
        _temperature: f32,
    ) -> Result<String> {
        let n = self
            .calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        // 必须转义：摘要 prompt 里带换行，直接拼进 JSON 字符串会解析失败
        let echoed = serde_json::to_string(&format!(
            "【模拟回复】第 {} 次，收到：{}",
            n, user_prompt
        ))?;
        Ok(format!(
            "{{\"story_state\":{},\"confirmed\":[\"模拟确认项\"],\
              \"open_threads\":[],\"next_step\":\"模拟下一步\"}}",
            echoed
        ))
    }
}

/// 测试用假 AI 配置：固定模型名，不读数据库。
struct MockAiSettings;

#[async_trait]
impl AiSettingsPort for MockAiSettings {
    async fn load(&self) -> Result<AiRuntimeConfig> {
        Ok(AiRuntimeConfig {
            base_url: "http://127.0.0.1:1".to_string(),
            api_key: None,
            model: "test".to_string(),
            // 测试不验证按用途路由，留空即全部回落到 model / 用途默认温度
            task_models: Default::default(),
            task_temperatures: Default::default(),
            context_limit: 128_000,
            max_output_tokens: 22_000,
        })
    }
}

/// 测试用假引导进度：固定停在第一步。
struct MockGuideProgress;

#[async_trait]
impl GuideProgressPort for MockGuideProgress {
    async fn current_step(&self, _project_id: Uuid) -> Result<Option<String>> {
        Ok(Some("premise".to_string()))
    }
}

fn make_runtime() -> Arc<AgentRuntime> {
    make_runtime_with_summary(Arc::new(InMemorySessionSummary::new()))
}

/// 构造运行时并**保留摘要端口句柄**（测试需要直接查摘要内容时使用）。
fn make_runtime_with_summary(
    summaries: Arc<InMemorySessionSummary>,
) -> Arc<AgentRuntime> {
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlm::default());
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
            Arc::new(MockAiSettings),
            Arc::new(MockGuideProgress),
            summaries,
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

#[tokio::test]
async fn summarize_session_stores_one_rolling_summary() {
    let summaries = Arc::new(InMemorySessionSummary::new());
    let rt = make_runtime_with_summary(summaries.clone());
    let project = Uuid::new_v4();
    let sid = rt.create_session(project).await.unwrap();
    rt.chat(sid, "我想写一个玄幻小说").await.unwrap();

    rt.summarize_session(sid).await.unwrap();
    let first = rt.load_summary(project).await.unwrap().expect("应写入摘要");
    assert!(!first.content.story_state.is_empty());

    // 第二次收尾：项目级摘要必须被**覆盖**，不是变成两份
    rt.summarize_session(sid).await.unwrap();
    let second = rt.load_summary(project).await.unwrap().expect("摘要仍在");
    assert!(
        second.content.story_state.contains("第 3 次"),
        "应是最新一次生成的内容：{}",
        second.content.story_state
    );
}

#[tokio::test]
async fn summarize_session_feeds_previous_summary_as_baseline() {
    let summaries = Arc::new(InMemorySessionSummary::new());
    let rt = make_runtime_with_summary(summaries.clone());
    let project = Uuid::new_v4();
    let sid = rt.create_session(project).await.unwrap();

    rt.summarize_session(sid).await.unwrap();
    let first = rt.load_summary(project).await.unwrap().unwrap();

    // 第二次收尾前，会话里没有任何消息，能出现在 prompt 里的旧内容只可能来自摘要基线。
    // （MockLlm 回显 prompt 并把换行转义，故这里匹配不含换行的片段。）
    rt.summarize_session(sid).await.unwrap();
    let second = rt.load_summary(project).await.unwrap().unwrap();
    assert!(
        second.content.story_state.contains("上一位记录员的摘要"),
        "第二次生成必须把旧摘要当基线喂进提示词，否则会静默丢掉前几轮结论；\
         实际 story_state = {}",
        second.content.story_state
    );
    assert!(
        !first.content.story_state.is_empty(),
        "第一次收尾也应产出内容"
    );
}

#[tokio::test]
async fn summarize_session_writes_injectable_memory_entry() {
    let rt = make_runtime();
    let project = Uuid::new_v4();
    let sid = rt.create_session(project).await.unwrap();

    rt.summarize_session(sid).await.unwrap();
    // 再收尾一次：注入通道也必须**仍然只有一条**摘要记忆。
    // 否则项目全部记忆都会进系统提示词，多份摘要会互相矛盾。
    rt.summarize_session(sid).await.unwrap();

    let items = rt.recall(project).await.unwrap();
    let injected: Vec<_> = items
        .iter()
        .filter(|m| m.memory_type == "session_summary")
        .collect();
    assert_eq!(
        injected.len(),
        1,
        "重复收尾后注入通道仍应只有一条摘要记忆：{:?}",
        items
    );
    assert!(
        injected[0].content.contains("story_state"),
        "记忆里应是结构化摘要：{}",
        injected[0].content
    );
}

#[tokio::test]
async fn summarize_unknown_session_errors() {
    let rt = make_runtime();
    let r = rt.summarize_session(Uuid::new_v4()).await;
    assert!(r.is_err(), "未知会话收尾应报错");
}

// ---------- batch_call：批量调用 ----------
//
// 它靠「实测」不足以锁住行为：上限、嵌套拒绝、失败中止都是安全阀，
// 一旦回归（例如中止逻辑失效）会表现为「模型以为批量都成功了」。

/// 测试用假工具：把入参原样回显，并记录被调用的次数。
struct SpyTool {
    name: String,
    calls: std::sync::atomic::AtomicUsize,
}

impl SpyTool {
    fn new(name: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            calls: std::sync::atomic::AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl AgentTool for SpyTool {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        "测试用回显工具".into()
    }
    fn input_schema(&self) -> serde_json::Value {
        // 故意把 `project_id` 声明成必填：真实领域工具（如 list_foreshadows）就是这样。
        // 一旦框架没把 project_id 注入到 batch_call 的子调用里，这里立刻以
        // 「缺少必填字段: project_id」失败——即「批处理里调用项目级工具报
        // project_id 缺失」的回归测试。
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" },
                "project_id": { "type": "string" }
            },
            "required": ["text", "project_id"]
        })
    }
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // 回显 project_id：用于验证框架注入是否真的落到了每个子调用上
        Ok(serde_json::json!({
            "echo": input.get("text"),
            "project_id": input.get("project_id"),
        }))
    }
}

/// 构造带 batch_call 的注册表（batch_call 需要拿到注册表本身）。
fn registry_with_batch(spy: &Arc<SpyTool>) -> Arc<ToolRegistry> {
    let reg = Arc::new(ToolRegistry::new());
    reg.register(spy.clone());
    reg.register(Arc::new(BatchCallTool::new(reg.clone())));
    reg
}

#[tokio::test]
async fn batch_call_runs_items_in_order() {
    let spy = SpyTool::new("spy");
    let reg = registry_with_batch(&spy);
    let project_id = Uuid::new_v4();
    let out = reg
        .execute_in_project(
            project_id,
            "batch_call",
            serde_json::json!({"calls": [
                {"tool": "spy", "input": {"text": "一"}},
                {"tool": "spy", "input": {"text": "二"}},
                {"tool": "spy", "input": {"text": "三"}}
            ]}),
        )
        .await
        .unwrap();

    assert_eq!(out["ok"], serde_json::json!(true));
    assert_eq!(out["executed"], serde_json::json!(3));
    assert_eq!(out["total"], serde_json::json!(3));
    assert_eq!(spy.calls.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(out["results"][0]["result"]["echo"], serde_json::json!("一"));
    assert_eq!(out["results"][2]["result"]["echo"], serde_json::json!("三"));
    // 每个子调用都拿到了框架注入的 project_id
    for i in 0..3 {
        assert_eq!(
            out["results"][i]["result"]["project_id"],
            serde_json::json!(project_id.to_string()),
            "第 {} 项子调用没有拿到注入的 project_id",
            i
        );
    }
}

/// 子调用里模型自己写的 project_id 必须被会话值覆盖（物理隔离不能只覆盖顶层）。
#[tokio::test]
async fn batch_call_overrides_model_supplied_project_id_in_children() {
    let spy = SpyTool::new("spy");
    let reg = registry_with_batch(&spy);
    let project_id = Uuid::new_v4();
    let out = reg
        .execute_in_project(
            project_id,
            "batch_call",
            serde_json::json!({"calls": [
                {"tool": "spy", "input": {
                    "text": "一",
                    "project_id": "00000000-0000-0000-0000-000000000000"
                }}
            ]}),
        )
        .await
        .unwrap();

    assert_eq!(out["ok"], serde_json::json!(true));
    assert_eq!(
        out["results"][0]["result"]["project_id"],
        serde_json::json!(project_id.to_string()),
        "模型伪造的 project_id 必须被会话值覆盖"
    );
}

/// 未经框架注入直接调 batch_call：必须报错，不得「不过滤项目」继续跑。
#[tokio::test]
async fn batch_call_without_injected_project_id_errors_loudly() {
    let spy = SpyTool::new("spy");
    let reg = registry_with_batch(&spy);
    let err = reg
        .execute(
            "batch_call",
            serde_json::json!({"calls": [{"tool": "spy", "input": {"text": "一"}}]}),
        )
        .await
        .unwrap_err();

    assert!(err.to_string().contains("project_id"), "{}", err);
    assert_eq!(
        spy.calls.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "缺少 project_id 时不应执行任何子调用"
    );
}

#[tokio::test]
async fn batch_call_rejects_unknown_tool_without_silent_skip() {
    let spy = SpyTool::new("spy");
    let reg = registry_with_batch(&spy);
    let out = reg
        .execute_in_project(
            Uuid::new_v4(),
            "batch_call",
            serde_json::json!({"calls": [
                {"tool": "spy", "input": {"text": "ok"}},
                {"tool": "does_not_exist", "input": {}}
            ]}),
        )
        .await
        .unwrap();

    assert_eq!(out["ok"], serde_json::json!(false), "有失败项时整体不能报成功");
    assert_eq!(out["aborted_at"], serde_json::json!(1));
    assert_eq!(out["results"][1]["ok"], serde_json::json!(false));
    assert!(out["results"][1]["error"]
        .as_str()
        .unwrap()
        .contains("does_not_exist"));
}

#[tokio::test]
async fn batch_call_aborts_remaining_items_after_failure() {
    let spy = SpyTool::new("spy");
    let reg = registry_with_batch(&spy);
    // 第二项入参缺必填字段，会被 Schema 校验拦下；第三项不该被执行
    let out = reg
        .execute_in_project(
            Uuid::new_v4(),
            "batch_call",
            serde_json::json!({"calls": [
                {"tool": "spy", "input": {"text": "一"}},
                {"tool": "spy", "input": {}},
                {"tool": "spy", "input": {"text": "三"}}
            ]}),
        )
        .await
        .unwrap();

    assert_eq!(out["ok"], serde_json::json!(false));
    assert_eq!(out["aborted_at"], serde_json::json!(1));
    assert_eq!(
        spy.calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "失败后不应继续执行后续项"
    );
}

#[tokio::test]
async fn batch_call_rejects_nesting_and_oversize_and_empty() {
    let spy = SpyTool::new("spy");
    let reg = registry_with_batch(&spy);

    let nested = reg
        .execute_in_project(
            Uuid::new_v4(),
            "batch_call",
            serde_json::json!({"calls": [{"tool": "batch_call", "input": {"calls": []}}]}),
        )
        .await
        .unwrap_err();
    assert!(nested.to_string().contains("嵌套"), "{}", nested);

    let many: Vec<serde_json::Value> = (0..BATCH_CALL_MAX_ITEMS + 1)
        .map(|i| serde_json::json!({"tool": "spy", "input": {"text": format!("{}", i)}}))
        .collect();
    let oversize = reg
        .execute_in_project(
            Uuid::new_v4(),
            "batch_call",
            serde_json::json!({ "calls": many }),
        )
        .await
        .unwrap_err();
    assert!(oversize.to_string().contains("上限"), "{}", oversize);

    let empty = reg
        .execute_in_project(
            Uuid::new_v4(),
            "batch_call",
            serde_json::json!({"calls": []}),
        )
        .await
        .unwrap_err();
    assert!(empty.to_string().contains("空"), "{}", empty);
}

// ---------- 框架层体积护栏 ----------
//
// 背景：实测 list_projects 曾一次返回 425,018 字符、list_entities 36,555 字符，
// 一次 batch_call 把 3 个列表拼成 56,668 字符 —— 这些字符每轮请求都要重发，
// 最终把模型拖到思考 116 秒并撞上网关超时。护栏必须**报错**而不是截断。

/// 返回指定长度结果的假工具。
struct BigTool;

#[async_trait]
impl AgentTool for BigTool {
    fn name(&self) -> String {
        "big".into()
    }
    fn description(&self) -> String {
        "返回大结果（测试体积护栏用）".into()
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "chars": { "type": "number" } }
        })
    }
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        let n = input.get("chars").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        Ok(serde_json::json!({ "blob": "x".repeat(n) }))
    }
}

#[tokio::test]
async fn oversized_tool_result_is_rejected_loudly() {
    let reg = Arc::new(ToolRegistry::new());
    reg.register(Arc::new(BigTool));

    // 上限之内：正常返回
    let ok = reg
        .execute("big", serde_json::json!({ "chars": 100 }))
        .await
        .unwrap();
    assert!(ok["blob"].is_string());

    // 超过上限：报错，并明确告诉调用方怎么缩小范围
    let err = reg
        .execute(
            "big",
            serde_json::json!({ "chars": TOOL_RESULT_MAX_CHARS + 100 }),
        )
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("超过单次上限"), "{}", msg);
    assert!(msg.contains("limit"), "应提示怎么缩小范围：{}", msg);
}

#[tokio::test]
async fn batch_call_stops_when_result_budget_exceeded() {
    let reg = Arc::new(ToolRegistry::new());
    reg.register(Arc::new(BigTool));
    reg.register(Arc::new(BatchCallTool::new(reg.clone())));

    // 每项 8,000 字符（单次上限之内），第三项会把累计推过 20,000 预算
    let out = reg
        .execute_in_project(
            Uuid::new_v4(),
            "batch_call",
            serde_json::json!({"calls": [
                {"tool": "big", "input": {"chars": 8000}},
                {"tool": "big", "input": {"chars": 8000}},
                {"tool": "big", "input": {"chars": 8000}},
                {"tool": "big", "input": {"chars": 10}}
            ]}),
        )
        .await
        .unwrap();

    assert_eq!(out["ok"], serde_json::json!(false), "{}", out);
    assert_eq!(out["aborted_reason"], serde_json::json!("结果体积超预算"));
    assert_eq!(out["executed"], serde_json::json!(3), "第三项后应停止");
    assert_eq!(out["total"], serde_json::json!(4));
    let used = out["used_chars"].as_u64().unwrap();
    assert!(
        used <= BATCH_RESULT_BUDGET_CHARS as u64,
        "已用字数不应超过预算：{}",
        used
    );
    assert!(used > 15000, "前两项应当已经接近预算：{}", used);
    assert!(
        out["message"].as_str().unwrap().contains("拆成多次 batch_call"),
        "应给出可执行的建议：{}",
        out["message"]
    );
    let items = out["results"].as_array().unwrap();
    assert_eq!(items.len(), 3, "只应包含已执行的项");
    // 每项都带自己的字符数，方便定位是哪一项把预算吃掉的
    assert!(items[0]["chars"].as_u64().unwrap() > 7000);
    // 超预算那一项：执行了、但结果不返回，且明确说明（不是静默丢弃）
    assert_eq!(items[2]["result_omitted"], serde_json::json!(true));
    assert!(
        items[2]["note"].as_str().unwrap().contains("单独调用"),
        "应告诉模型怎么拿到它：{}",
        items[2]
    );
    // 返回体积必须真的被压住（否则上下文照样炸）
    assert!(
        agent::result_chars(&out) < BATCH_RESULT_MAX_CHARS,
        "返回体积应被压在上限内：{}",
        agent::result_chars(&out)
    );
}
