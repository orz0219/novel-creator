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
use uuid::Uuid;

use crate::types::ToolMeta;

/// 单个工具结果的字符上限：超过就**报错**，让调用方缩小范围。
///
/// 为什么要有这条硬线：实测 `list_projects` 曾一次返回 **425,018 字符**、
/// `list_entities` 36,555 字符。这些字符进入上下文后**每轮请求都要重发**，
/// 最终把模型拖到"思考 116 秒"并撞上网关超时（`error decoding response body`）。
/// 有了它，任何工具（包括以后新增的）都不可能再把上下文炸掉。
pub const TOOL_RESULT_MAX_CHARS: usize = 12_000;

/// `batch_call` 单次调用的**累计**结果预算：超了就停止后续项并说明原因。
pub const BATCH_RESULT_BUDGET_CHARS: usize = 20_000;

/// `batch_call` **结果本身**的字符预算 = 累计预算 + 信封余量。
///
/// 为什么不直接用累计预算：批量返回里除各项结果外还有 index / tool / chars 等
/// 信封字段；两者相等的话，正常停止也会被外层护栏再拦一次（实测踩到）。
/// 余量只需覆盖信封（几百字符），因为超预算的那一项**不会**把结果放进返回。
pub const BATCH_RESULT_MAX_CHARS: usize = BATCH_RESULT_BUDGET_CHARS + 2_000;

/// 结果进入上下文时的实际字符数（runtime 用 pretty JSON 写进消息）。
pub fn result_chars(value: &serde_json::Value) -> usize {
    serde_json::to_string_pretty(value)
        .map(|s| s.chars().count())
        .unwrap_or(0)
}

/// 框架注入的字段名：项目级工具靠它做物理隔离（见 [`inject_project_id`]）。
pub const PROJECT_ID_FIELD: &str = "project_id";

/// 把会话的 `project_id` 注入（**覆盖**模型传入值）到工具入参对象里。
///
/// 顶层单次调用与 `batch_call` 的子调用都调用本函数，保证注入规则只有一份：
/// 谁执行工具，谁就先把 `project_id` 注入好，不存在"某条路径忘了注入"。
///
/// 入参不是 JSON 对象时直接报错——不静默改写成 `{}`，否则工具会带着错误入参继续跑。
pub fn inject_project_id(input: &mut serde_json::Value, project_id: Uuid) -> Result<()> {
    let obj = input
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("tool input must be a JSON object"))?;
    obj.insert(PROJECT_ID_FIELD.to_string(), json!(project_id.to_string()));
    Ok(())
}

/// 长文本写入：`append` 为 true 时把 `incoming` 拼到原值后面，否则整体替换。
///
/// 为什么需要：单次输出有上限，长描述 / 长章节常被截断，模型只能把整段重发一遍；
/// 有了追加语义就能分两三次写完，每次只发新增部分。
///
/// 语义刻意保持"笨"：**直接拼接**，不自动加换行或分隔符——需要分段时由调用方
/// 在文本里自己带（框架不猜作者的排版意图）。
pub fn append_or_replace(existing: Option<&str>, incoming: &str, append: bool) -> String {
    if !append {
        return incoming.to_string();
    }
    match existing {
        Some(e) => format!("{}{}", e, incoming),
        None => incoming.to_string(),
    }
}

/// 校验入参的**顶层字段名**是否都在工具 schema 的 `properties` 里。
///
/// 与 [`ensure_known_subfields`] 是一对：前者管字段名，后者管字段里的子字段名。
/// 字段名写错却静默忽略，会变成"值写进了陌生键、落库时被 `serde` 丢掉、
/// 读回来是 null，工具却回 ok: true"——正是要杜绝的静默丢数据。
///
/// 框架注入的 `project_id` 永远放行；工具自己的控制字段（如 `arc_stages_mode`）
/// 由调用方通过 `extra_allowed` 传入。
pub fn ensure_known_fields(
    input: &serde_json::Value,
    tool_schema: &serde_json::Value,
    extra_allowed: &[&str],
) -> Result<()> {
    let Some(obj) = input.as_object() else {
        return Ok(());
    };
    let allowed: Vec<String> = tool_schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    if allowed.is_empty() {
        anyhow::bail!("内部错误：工具 schema 未声明 properties，无法校验字段名");
    }

    let mut unknown: Vec<String> = obj
        .keys()
        .filter(|k| {
            !allowed.iter().any(|a| a == *k)
                && !extra_allowed.contains(&k.as_str())
                && k.as_str() != PROJECT_ID_FIELD
        })
        .cloned()
        .collect();
    if !unknown.is_empty() {
        unknown.sort();
        anyhow::bail!(
            "字段名不存在：{}。可写字段只有：{}",
            unknown.join(" / "),
            allowed.join(" / ")
        );
    }
    Ok(())
}

/// 校验**结构字段的子字段名**（对象型与结构化数组型都覆盖）。
///
/// 为什么必须有：顶层字段名写错会报「未知字段」，但子字段写错原先完全无声——
/// 例如 `arc_potential: {"initial_state": ...}`（真实字段名是 `starting_state`）
/// 会被合并进档案，落库时被 `serde` 丢掉，工具却回 `ok: true`，读回来全是 null。
///
/// 允许的子字段清单**直接从工具 schema 派生**（对象取 `properties`，数组取
/// `items.properties`），不另写一份常量，避免契约与校验清单各自漂移。
///
/// 该字段在 schema 里没声明子字段时（字符串、或刻意保留的自由对象）不校验——
/// 没有契约就没有可校验的内容；元素写成字符串的简写同样跳过。
pub fn ensure_known_subfields(
    field: &str,
    payload: &serde_json::Value,
    tool_schema: &serde_json::Value,
) -> Result<()> {
    let Some(node) = tool_schema.get("properties").and_then(|p| p.get(field)) else {
        return Ok(());
    };

    let (items, is_array): (Vec<&serde_json::Value>, bool) = match payload {
        serde_json::Value::Object(_) => (vec![payload], false),
        serde_json::Value::Array(a) => (a.iter().collect(), true),
        // 字符串简写：没有子字段可校验
        _ => return Ok(()),
    };

    let container = if is_array { node.get("items") } else { Some(node) };
    let allowed: Vec<String> = container
        .and_then(|c| c.get("properties"))
        .and_then(|p| p.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    if allowed.is_empty() {
        return Ok(());
    }

    let mut unknown: Vec<String> = Vec::new();
    for item in items {
        if let serde_json::Value::Object(obj) = item {
            for k in obj.keys() {
                if !allowed.iter().any(|a| a == k) && !unknown.iter().any(|u| u == k) {
                    unknown.push(k.clone());
                }
            }
        }
    }
    if !unknown.is_empty() {
        anyhow::bail!(
            "{} 里的子字段名不存在：{}。该字段允许的子字段只有：{}",
            field,
            unknown.join(" / "),
            allowed.join(" / ")
        );
    }
    Ok(())
}

/// Agent 工具接口。
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// 工具名（唯一键）
    fn name(&self) -> String;
    /// 自然语言描述（进入提示词，也用于前端展示）
    fn description(&self) -> String;
    /// 输入 JSON Schema（用于提示词拼接，P2 用于 Schema 校验）
    fn input_schema(&self) -> serde_json::Value;
    /// 本工具单次结果的字符预算（默认 [`TOOL_RESULT_MAX_CHARS`]）。
    ///
    /// 聚合型工具（如 `batch_call`）可以调高——它一次代替多次调用，
    /// 但必须自带与之匹配的累计预算控制，否则就是把上下文炸点搬了个地方。
    fn result_budget(&self) -> usize {
        TOOL_RESULT_MAX_CHARS
    }
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

    /// 按名字执行一个工具（查表 + Schema 校验 + 调用），**不做任何注入**。
    ///
    /// 真实调用路径一律走 [`Self::execute_in_project`]：本方法保留给「入参已由调用方
    /// 组织好、不需要项目隔离」的场景（如单元测试里的假工具）。
    pub async fn execute(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("tool not found: {}", name))?;
        if let Err(e) = crate::runtime::validate_input(&input, &tool.input_schema()) {
            anyhow::bail!("工具 '{}' 入参校验失败: {}", name, e);
        }
        // 预算要在 execute 之前取：工具可能被 move
        let budget = tool.result_budget();
        let value = tool.execute(input).await?;
        // 体积护栏：超上限**报错**而不是截断——截断会让模型以为"就这些内容"，
        // 报错才能让它主动缩小范围（列表用 limit/offset，单个对象用 get_*）。
        let chars = result_chars(&value);
        if chars > budget {
            anyhow::bail!(
                "工具 '{}' 的结果 {} 字符，超过单次上限 {} 字符：请缩小范围后重试\
                 （列表类用 limit / offset 翻页；单个对象用 get_* 按需读取），不要一次拉全量",
                name,
                chars,
                budget
            );
        }
        Ok(value)
    }

    /// 带项目隔离的执行：先注入 `project_id`（覆盖模型传入值），再走 [`Self::execute`]。
    ///
    /// 单次调用（`runtime::tool_execute`）与 `batch_call` 的子调用都调用本方法，
    /// 因此批量子调用与顶层单次调用享有**同一套注入与校验**，没有旁路。
    pub async fn execute_in_project(
        &self,
        project_id: Uuid,
        name: &str,
        mut input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        inject_project_id(&mut input, project_id)?;
        self.execute(name, input).await
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

/// 批量调用：一次执行多个工具，把多轮往返压成一轮。
///
/// 典型场景：补 7 条关系边、给 6 个新角色逐个补档案——原先每条边一轮对话。
/// **顺序执行**，因此后一项能用上前一项的产物（例如刚建出来的实体 id）。
///
/// 四条硬约束：
/// 1. 禁止嵌套 `batch_call`：否则可以无限自套，日志与配额都会失控；
/// 2. 单次上限 [`BATCH_CALL_MAX_ITEMS`] 项：模型一次塞几百项会同时撞上
///    JSON 长度与超时，宁可让它自己分批；
/// 3. 任一项失败**不静默跳过**：整体返回 `ok: false` 并逐项标记结果，
///    同时中止后续项——后续项常常依赖前面刚建出来的 id，
///    在错误前提下继续跑只会产出一堆垃圾。
/// 4. 子调用与顶层单次调用走**同一条注入路径**（[`ToolRegistry::execute_in_project`]）：
///    框架注入的 `project_id` 会被逐项写进子调用入参。项目级工具在批处理里
///    同样不需要（也不应该）自己传 `project_id`。
/// 5. **累计结果体积有预算**（[`BATCH_RESULT_BUDGET_CHARS`]）：超了就停止后续项并说明
///    原因与已用字数。批量读是"读放大"最容易失控的地方——实测一次 batch_call 把
///    3 个列表拼成 56,668 字符，把模型拖到思考 116 秒并撞上网关超时。
pub const BATCH_CALL_MAX_ITEMS: usize = 20;

pub struct BatchCallTool {
    registry: Arc<ToolRegistry>,
}

impl BatchCallTool {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl AgentTool for BatchCallTool {
    fn name(&self) -> String {
        "batch_call".into()
    }

    fn description(&self) -> String {
        format!(
            "一次调用按顺序执行多个工具（最多 {} 项），把多轮往返压成一轮。\
             适用于批量补关系边、批量补档案；「建完后要拿新 id 继续操作」也适用，\
             因为顺序执行、后一项能用前一项的产物。\
             任一项失败会中止后续项并逐项标明原因（不会静默跳过）。不允许嵌套 batch_call。\
             子调用与单次调用一样由框架注入 project_id：不要在 input 里手写 project_id。\
             整批结果的累计体积有上限（{} 字符），超了就停止后续项并报明已用字数——\
             批量读请给每项加 limit / offset，不要指望一次拉全量。",
            BATCH_CALL_MAX_ITEMS,
            BATCH_RESULT_BUDGET_CHARS
        )
    }

    /// 聚合器：一次代替多次调用，所以预算按累计口径给（而不是单工具上限）。
    /// 超预算时会在 execute 内部停止后续项并说明，见 [`BATCH_RESULT_BUDGET_CHARS`]。
    fn result_budget(&self) -> usize {
        BATCH_RESULT_MAX_CHARS
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "calls": {
                    "type": "array",
                    "description": "按顺序执行的工具调用列表",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": { "type": "string", "description": "工具名，如 create_relation / update_character_profile" },
                            "input": { "type": "object", "description": "该工具的入参对象" }
                        },
                        "required": ["tool", "input"]
                    }
                }
            },
            "required": ["calls"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value> {
        // project_id 是前置条件：由 `runtime::tool_execute` 注入（会话级，模型无法伪造）。
        // 缺失或非法时直接报错，不降级为「不过滤项目」继续跑。
        let project_id = input
            .get(PROJECT_ID_FIELD)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{} 缺失：batch_call 需要框架注入的 {} 才能执行子调用（调用方应走 ToolRegistry::execute_in_project）",
                    PROJECT_ID_FIELD,
                    PROJECT_ID_FIELD
                )
            })?;
        let project_id = Uuid::parse_str(project_id)
            .map_err(|_| anyhow::anyhow!("{} 不是合法 UUID：{}", PROJECT_ID_FIELD, project_id))?;

        let calls = input
            .get("calls")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("calls 缺失或不是数组"))?;
        if calls.is_empty() {
            anyhow::bail!("calls 为空：至少要给一项，否则这次调用没有意义");
        }
        if calls.len() > BATCH_CALL_MAX_ITEMS {
            anyhow::bail!(
                "calls 有 {} 项，超过单次上限 {} 项：请拆成多次 batch_call",
                calls.len(),
                BATCH_CALL_MAX_ITEMS
            );
        }

        let mut results: Vec<serde_json::Value> = Vec::with_capacity(calls.len());
        // 累计结果体积：批量读是"读放大"最容易失控的地方（一次 batch_call
        // 曾把 3 个列表拼成 56,668 字符），所以这里也设一条硬线。
        let mut used_chars: usize = 0;
        for (index, call) in calls.iter().enumerate() {
            let tool_name = call
                .get("tool")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("calls[{}] 缺少 tool 名称", index))?
                .to_string();

            if tool_name == "batch_call" {
                anyhow::bail!("不允许嵌套 batch_call（calls[{}]）", index);
            }

            let call_input = call
                .get("input")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("calls[{}] 缺少 input", index))?;

            // 未注册的工具名由 execute 报「未知工具」，错误里带上名字。
            // 子调用走 execute_in_project：与顶层单次调用同一套注入 + 校验。
            match self
                .registry
                .execute_in_project(project_id, &tool_name, call_input)
                .await
            {
                Ok(value) => {
                    let chars = result_chars(&value);
                    // 预算在**放入返回之前**判断：这一项可能已经执行完（副作用已发生），
                    // 但把它的结果塞进返回会把上下文撑爆。此时只报告"跑了、多大、
                    // 结果没返回"，让模型自己决定要不要单独再调一次——不静默丢弃。
                    if used_chars + chars > BATCH_RESULT_BUDGET_CHARS {
                        results.push(json!({
                            "index": index,
                            "tool": tool_name,
                            "ok": true,
                            "chars": chars,
                            "result_omitted": true,
                            "note": format!(
                                "本项已执行，但结果 {} 字符，放进返回会超过批量预算 {} 字符，故只报告不返回；\
                                 需要它的内容请单独调用这一项，并加上 limit / offset 缩小范围",
                                chars, BATCH_RESULT_BUDGET_CHARS
                            ),
                        }));
                        return Ok(json!({
                            "ok": false,
                            "executed": index + 1,
                            "total": calls.len(),
                            "aborted_at": index,
                            "aborted_reason": "结果体积超预算",
                            "budget_chars": BATCH_RESULT_BUDGET_CHARS,
                            "used_chars": used_chars,
                            "results": results,
                            "message": format!(
                                "前 {} 项的结果累计已 {} 字符，再加这一项就会超过单次批量预算 {} 字符，\
                                 已停止后续 {} 项。请用 limit / offset 缩小每项的读取范围，\
                                 或把这次批量拆成多次 batch_call。",
                                index,
                                used_chars,
                                BATCH_RESULT_BUDGET_CHARS,
                                calls.len() - index - 1
                            ),
                        }));
                    }
                    used_chars += chars;
                    results.push(json!({
                        "index": index,
                        "tool": tool_name,
                        "ok": true,
                        "chars": chars,
                        "result": value,
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "index": index,
                        "tool": tool_name,
                        "ok": false,
                        "error": e.to_string(),
                    }));
                    return Ok(json!({
                        "ok": false,
                        "executed": index + 1,
                        "total": calls.len(),
                        "aborted_at": index,
                        "results": results,
                        "message": format!(
                            "第 {} 项（{}）失败，已中止后续 {} 项",
                            index,
                            tool_name,
                            calls.len() - index - 1
                        ),
                    }));
                }
            }
        }

        Ok(json!({
            "ok": true,
            "executed": results.len(),
            "total": calls.len(),
            "results": results,
        }))
    }
}
