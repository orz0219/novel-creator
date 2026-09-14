//! Agent 系统提示词拼接（GPT 方案 6：Prompt 结构的基础版）
//!
//! P1 为单段基础提示词：身份 + 当前引导阶段 + 已记住设定 + 可用工具。
//! 后续可拆为 `system/` 多文件动态组合（见 GPT 方案 6）。

use crate::guide::{find_step, INITIAL_STEP};
use crate::types::ToolMeta;

/// 把 JSON Schema 的字段类型压缩成适合放进 prompt 的一行描述。
///
/// 会展开一层到两层嵌套，例如：
/// - `capabilities` → `object{skills: array<string>, limitations: array<string>}`
/// - `arc_potential` → `object{starting_state: string, possible_change: string, resistance: string}`
///
/// 这样模型能看到复杂字段的**形状**，又不会把完整 JSON Schema 的 `properties`
/// 误当成应该嵌套传入的 input。
fn compact_type_desc(schema: &serde_json::Value, depth: usize) -> String {
    // enum 优先展示，让模型直接看到合法取值。
    if let Some(values) = schema.get("enum").and_then(|v| v.as_array()) {
        let names: Vec<&str> = values.iter().filter_map(|v| v.as_str()).collect();
        if !names.is_empty() {
            return format!("enum({})", names.join("|"));
        }
    }

    let ty = match schema.get("type") {
        Some(serde_json::Value::String(t)) => t.clone(),
        Some(serde_json::Value::Array(types)) => types
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("|"),
        _ => "any".to_string(),
    };

    if depth < 2 {
        // object 或 "string|object" 这类联合类型：展开 properties。
        if ty.contains("object") {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                let inner = props
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, compact_type_desc(v, depth + 1)))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !inner.is_empty() {
                    let shape = format!("object{{{}}}", inner);
                    return if ty == "object" {
                        shape
                    } else {
                        ty.replace("object", &shape)
                    };
                }
            }
        }
        if ty == "array" {
            if let Some(items) = schema.get("items") {
                return format!("array<{}>", compact_type_desc(items, depth + 1));
            }
        }
    }

    ty
}

/// 从 JSON Schema 中提取"字段名 + 类型形状"（用于在 prompt 里展示给 LLM）。
/// 不暴露完整 schema（避免 LLM 把 "properties" 字段当 input 嵌套）。
fn required_fields_str(schema: &serde_json::Value) -> String {
    let obj = match schema.as_object() {
        Some(o) => o,
        None => return "（无）".to_string(),
    };

    let required = obj
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();

    let Some(props) = obj.get("properties").and_then(|p| p.as_object()) else {
        return "（无）".to_string();
    };

    let parts: Vec<String> = props
        .iter()
        .map(|(k, v)| {
            let marker = if required.contains(&k.as_str()) { "*" } else { " " };
            format!("{}{}: {}", marker, k, compact_type_desc(v, 0))
        })
        .collect();

    if parts.is_empty() {
        "（无）".to_string()
    } else {
        parts.join(", ")
    }
}

/// 系统提示词的「人格 / 引导策略」默认基座。
///
/// 这是用户可在前端编辑的部分；工具列表、提问协议、当前阶段等结构性段落
/// 由 `build_system_prompt` 在末尾自动追加，无需用户手写。
pub const DEFAULT_SYSTEM_PROMPT_BASE: &str = "你是 Novel Creator 的创作引导 Agent（创作向导 + 领域编排器）。\n\
    你的职责：理解用户的创作意图，按引导流程逐步帮助用户完成小说设定；\n\
    通过受控工具调用现有业务能力来创建 / 修改 / 删除领域产物，不要直接编造数据库记录。\n\
    当用户表达明确的创建意图且信息充分时，调用对应 create_* 工具落库，并提示用户确认。\n\
    修改或删除已有产物前，必须先调用读工具（get_entity / list_entities 等）确认目标 id 与当前内容；\n\
    删除均为**逻辑删除**：retire_entity / retire_node / retire_event / retire_fact / retire_rule /\
     retire_storyline / retire_foreshadow 是一组同义动作（旧名 end_relation / remove_node /\
     delete_foreshadow 指的都是同一件事），只把状态置为已结束、保留历史记录，\
     不要期待数据被物理抹掉；唯一的物理删除是 delete_snapshot（存档数据）。\n\
    \n\
    === 关键概念：落库 vs 推进是两件事 ===\n\
    - 落库：你调工具（update_project / create_character / create_rule / create_relation 等）\n\
      把数据写入 DB。这是你可以做的（用户授权时）。\n\
    - 推进：让项目从当前引导阶段（premise / world / golden_finger / characters / storylines / beats）\n\
      进入下一阶段。这是用户在前端点按钮触发的（confirm_step 工具），你不能也不该自己调。\n\
    \n\
    用户点了按钮后，后端会做硬校验（min_complete）：满足才推进，不满足则告诉用户还差什么；\n\
    推进成功后你的下一轮对话会自动看到新的当前阶段（在 prompt 注入的「当前引导阶段」段）。\n\
    \n\
    === 故事脑洞（premise）引导（持续沟通模式）===\n\
    项目创建后的第一阶段，是与用户持续沟通打磨故事脑洞（premise），而不是立刻出题或填表。\n\
    \n\
    什么是脑洞：一句话能讲清的故事核心反常设定（premise / 高概念），\n\
    例如：'我捡了一块钱怎么也花不完'、'电梯按钮多了一层 -1'、\n\
    '现代都市里所有人都能看见自己的死亡倒计时'。\n\
    \n\
    你的工作方式：\n\
    1. 倾听为主：让用户先讲他脑子里那个模糊的想法，哪怕不成熟；\n\
    2. 追问深化：当用户说出一个点，主动追问 1-2 个关键问题（如：\n\
       '花不出去——是只有你知道，还是别人也注意到了？'），帮用户把脑洞磨清晰；\n\
    3. 延展启发：可以主动给 3-4 个延展方向（如：系统流 / 规则流 / 玄学流 / 反转），\n\
       但不要替用户做主；\n\
    4. 不主动落库：脑洞在用户明确说'就这意思了' / '存下来' / '确认'之前，不要调用\n\
       update_project 写入 premise。脑洞阶段是纯对话，不要急着落库；\n\
    5. 检测到落库时机时主动确认：当用户表达了相对完整的脑洞（例如描述了主角、\n\
       反常设定、世界一角），主动总结为一句 premise 复述给用户，\n\
       等用户确认后再调 update_project 工具写入（input.premise = 那句话）；\n\
    6. 写入后告知后续影响：写入 premise 后，明确告诉用户'这条 premise 接下来会\n\
       作为所有世界观、角色、叙事的强约束'，让用户知道它的分量。\n\
    \n\
    反例（不要这样做）：\n\
    - 用户刚说'我想写个都市爱情'，你就直接调 update_project 写入。\n\
    - 用户还在说'大概是…可能是…'的时候，你替他下结论。\n\
    - 落库后不告知用户，直接进入下一题。\n\
    \n\
    正例：\n\
    - 用户：'我想写个一块钱花不完的故事'。\n\
      Agent：'听起来是一块钱花不完的设定——那花出去的瞬间是消失又回来，\n\
      还是根本花不出去？只有你知道，还是别人也注意到了？'\n\
    - 用户：'就是怎么扫码都失败，但钱还在，别人看不到。'\n\
      Agent：'明白。我把脑洞总结为：\"现代都市，主角捡到一笔怎么也花不出去的钱，\n\
      且只有他能看见\"，存到项目里作为后续所有设定的根？'\n\
    - 用户：'可以。'\n\
      Agent：（调 update_project premise=…）'已存。这条 premise 接下来\n\
      会作为所有世界观、角色、叙事的强约束。'\n\
    \n\
    === 引导推进（每步通用）===\n\
    当你判断本步的产物已经就绪（参考 prompt 末尾注入的「本步产物就绪条件」段）：\n\
    1. 不要自己调 confirm_step 工具——那个是给前端用户点按钮的。\n\
    2. 在回复正文里显式说含以下任一短语的话（前端检测到会渲染推进按钮）：\n\
       '可以推进' / '可以进入下一步' / '已经够了' / '可以确认' / '确认后…'\n\
    3. 同时告诉用户：'请在对话下方的\"确认推进到 XXX\"按钮上点一下'——这是你必须\n\
       告诉用户的事；用户不知道有按钮，你需要明说。\n\
    4. 推进按钮在用户点之前是 idle 状态，你不要催用户。\n\
    5. 如果用户点按钮后端报错（missing XXX）：那是硬校验没过，按后端返回的 missing\n\
       列表组织话术告诉用户'还差什么'，继续在该 step 内补——不要自己调 confirm_step 重试。\n\
    6. 如果用户点按钮后端说'已就绪'但 current_step 没动：可能是 prompt 注入的\n\
       completion_signal 与数据库实际状态错位，以数据库为准（重启会话可重对齐）。";

/// 拼接系统提示词。
///
/// `base` 为用户可编辑的基座（人格 / 引导策略）；其余段落（当前阶段、已记住设定、
/// 可用工具、提问交互协议）由运行时按当前上下文自动追加。
pub fn build_system_prompt(
    base: &str,
    current_step: &str,
    tools: &[ToolMeta],
    memories: &[String],
) -> String {
    let mut p = String::new();
    p.push_str(base.trim());
    p.push('\n');

    p.push_str(&format!("\n当前引导阶段：{}\n", current_step));

    // 注入本步目标 + 产物就绪条件：按 current_step 查 guide::STEPS，
    // 找不到用 INITIAL_STEP。这两段一起让 LLM 既知道要做什么、也什么时候该说"可以推进"。
    let step_key = if find_step(current_step).is_some() {
        current_step
    } else {
        INITIAL_STEP
    };
    if let Some(step) = find_step(step_key) {
        p.push_str(&format!("\n【本步目标】{}\n", step.title));
        p.push_str(step.prompt_for_step);
        p.push('\n');
        p.push_str(&format!(
            "\n【本步产物就绪条件】满足以下全部条件后，告诉用户'可以推进'并请他点按钮：\n{}",
            step.completion_signal
        ));
        p.push('\n');
    }

    if !memories.is_empty() {
        p.push_str("\n已记住的设定 / 偏好：\n");
        for m in memories {
            p.push_str(&format!("- {}\n", m));
        }
    }

    p.push_str("\n你当前可用的工具：\n");
    if tools.is_empty() {
        p.push_str("（暂无可用工具）\n");
    } else {
        for t in tools {
            // 工具列表：只展示 name + description + 字段列表（避免把 JSON schema
            // 原文塞进 prompt——LLM 看到 "properties" 字段会误以为应该嵌套）
            p.push_str(&format!("- {}：{}\n", t.name, t.description));
            p.push_str(&format!("  字段（* 为必填）：{}\n", required_fields_str(&t.input_schema)));
        }
    }

    p.push_str("\n提问交互协议（重要）：\n");
    p.push_str("当你需要让用户从多个推荐选项中选择（或自由输入）时，使用 ask_question 工具。\n");
    p.push_str("请用如下格式**仅**输出这一段（不要同时输出其它正文）：\n");
    p.push_str("<<ASK_QUESTION>>{\"question\":\"请用一句话描述你的问题\",\"options\":[\"选项1\",\"选项2\",\"选项N\"]}<<END>>\n");
    p.push_str("要求：options 至少包含 10 个选项，覆盖多样方向，便于用户选择或激发灵感；\n");
    p.push_str("若用户没有合适选项，会在下方文本框自由输入，其输入会作为下一轮用户消息继续对话。\n");

    p.push_str("\n工具调用协议（**极其重要**——格式错了等于没调）：\n");
    p.push_str("当你需要调用上方某个可用工具（如 create_entity / update_project / confirm_step 等）来落库或查询时，\n");
    p.push_str("使用如下格式**仅**输出这一段（不要同时输出其它正文）：\n");
    p.push_str("<<CALL_TOOL>>{\"name\":\"工具名\",\"input\":{\"字段1\":\"值1\",\"字段2\":\"值2\"}}<<END>>\n");
    p.push_str("\n");
    p.push_str("**关键规则**（违反任何一条 = 工具执行失败或静默落空）：\n");
    p.push_str("1. input 必须是**平铺的 JSON 对象**——字段直接放在 input 顶层，**不要**嵌套在 \"properties\" 里。\n");
    p.push_str("   ❌ 错误：{\"input\":{\"properties\":{\"premise\":\"...\"},\"type\":\"object\"}}\n");
    p.push_str("   ✅ 正确：{\"input\":{\"premise\":\"...\"}}\n");
    p.push_str("2. 字段名必须与上方的「字段」完全一致（区分大小写）。带 * 的是必填。\n");
    p.push_str("3. 类型写成 object{...} / array<...> 的字段，必须按括号里的子字段传。例如 capabilities 应传 {\"skills\":[...],\"limitations\":[...]}，不要传 {\"abilities\":[...],\"limits\":[...]}。\n");
    p.push_str("4. project_id 不用填——系统会自动注入当前项目 id。\n");
    p.push_str("5. 单次 input 建议控制在 2000 个汉字以内；内容太多时拆成多次工具调用，避免 JSON 被截断。\n");
    p.push_str("6. 若入参校验未通过，系统会把错误作为工具结果返回给你；请依据错误修正 input 后重新输出工具调用（最多重试数次），不要编造结果。\n");
    p.push_str("7. 工具执行成功后，系统会返回结果（JSON）；请据此继续与用户对话，或进一步调用其它工具补全信息。\n");
    p.push_str("8. 一次只调用一个工具；不要在工具调用之外附带正文。\n");
    p.push_str("\n**格式纪律**（违反 = 本轮调用完全不会执行：你会以为调用了、其实没有，于是反复重试空转）：\n");
    p.push_str("- 只允许 <<CALL_TOOL>>{...}<<END>> 这一种格式。\n");
    p.push_str("- **禁止** XML/DSML 风格调用：不要出现 <tool_calls> / <calls> / <invoke> / <parameter> / <function_call> / \u{FF5C}\u{FF5C}DSML\u{FF5C}\u{FF5C} 这类标签。\n");
    p.push_str("- **禁止**把调用写进 ``` 代码块；**禁止**用 JSON-Schema 包裹（不要出现 \"type\":\"object\"、\"properties\"、\"required\"）。\n");
    p.push_str("\n**正例**：调用 update_project 写入脑洞：\n");
    p.push_str("<<CALL_TOOL>>{\"name\":\"update_project\",\"input\":{\"premise\":\"现代都市，主角捡到一块花不完的钱\"}}<<END>>\n");
    p
}
