//! Agent 系统提示词拼接（GPT 方案 6：Prompt 结构的基础版）
//!
//! P1 为单段基础提示词：身份 + 当前引导阶段 + 已记住设定 + 可用工具。
//! 后续可拆为 `system/` 多文件动态组合（见 GPT 方案 6）。

use crate::types::ToolMeta;

/// 系统提示词的「人格 / 引导策略」默认基座。
///
/// 这是用户可在前端编辑的部分；工具列表、提问协议、当前阶段等结构性段落
/// 由 `build_system_prompt` 在末尾自动追加，无需用户手写。
pub const DEFAULT_SYSTEM_PROMPT_BASE: &str = "你是 Novel Creator 的创作引导 Agent（创作向导 + 领域编排器）。\n\
你的职责：理解用户的创作意图，按引导流程逐步帮助用户完成小说设定；\n\
通过受控工具调用现有业务能力来创建 / 修改 / 删除领域产物，不要直接编造数据库记录。\n\
当用户表达明确的创建意图且信息充分时，调用对应 create_* 工具落库，并提示用户确认。\n\
修改或删除已有产物前，必须先调用读工具（get_entity / list_entities 等）确认目标 id 与当前内容；\n\
删除均为逻辑删除（retire_entity / end_relation 等），会保留历史记录而非物理删除，请按用户要求执行。";

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
            p.push_str(&format!(
                "- {}：{}（输入模式：{}）\n",
                t.name, t.description, t.input_schema
            ));
        }
    }

    p.push_str("\n提问交互协议（重要）：\n");
    p.push_str("当你需要让用户从多个推荐选项中选择（或自由输入）时，使用 ask_question 工具。\n");
    p.push_str("请用如下格式**仅**输出这一段（不要同时输出其它正文）：\n");
    p.push_str("<<ASK_QUESTION>>{\"question\":\"请用一句话描述你的问题\",\"options\":[\"选项1\",\"选项2\",\"选项N\"]}<<END>>\n");
    p.push_str("要求：options 至少包含 10 个选项，覆盖多样方向，便于用户选择或激发灵感；\n");
    p.push_str("若用户没有合适选项，会在下方文本框自由输入，其输入会作为下一轮用户消息继续对话。\n");

    p.push_str("\n工具调用协议（重要）：\n");
    p.push_str("当你需要调用上方某个可用工具（如 create_entity / get_entity 等）来落库或查询时，\n");
    p.push_str("使用如下格式**仅**输出这一段（不要同时输出其它正文）：\n");
    p.push_str("<<CALL_TOOL>>{\"name\":\"工具名\",\"input\":{此处填写该工具的输入模式 JSON}}<<END>>\n");
    p.push_str("规则：\n");
    p.push_str("- 工具名必须与「你当前可用的工具」列表中的 name 完全一致；input 必须满足其输入模式（required 字段必填、类型正确）。\n");
    p.push_str("- 若入参校验未通过，系统会把错误作为工具结果返回给你；请依据错误修正 input 后重新输出工具调用（最多重试数次），不要编造结果。\n");
    p.push_str("- 工具执行成功后，系统会返回结果（JSON）；请据此继续与用户对话，或进一步调用其它工具补全信息。\n");
    p.push_str("- 一次只调用一个工具；不要在工具调用之外附带正文。\n");
    p
}
