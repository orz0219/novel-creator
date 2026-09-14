//! 会话收尾摘要：把「故事现在到哪了」沉淀成一份结构化滚动摘要。
//!
//! 与 `prompt.rs` 的区别：这里是**专用的一次性任务提示词**，不掺用户自定义基座
//! 与引导阶段规则——摘要只负责如实归纳对话，不该被创作人设影响措辞。

use anyhow::{Context, Result};

use domain::session_summary::SessionSummary;

/// 摘要写入 `agent_memory` 时使用的 memory_type。
///
/// 这是「注入通道」的标记：现有 `build_system_prompt` 会注入项目全部记忆，
/// 因此新开的会话能自动读到上一次收尾的摘要。
pub const SUMMARY_MEMORY_TYPE: &str = "session_summary";

/// 摘要生成用的系统提示词（固定，不可由用户配置）。
pub const SUMMARY_SYSTEM_PROMPT: &str = "\
你是小说创作项目的会话记录员。你的唯一任务：把这轮会话归纳成一份结构化摘要，
供作者之后新开会话时快速回想起「故事到哪了、还有什么没办」。

严格遵守：
1. 只输出一个 JSON 对象，不要输出任何解释文字，不要用 ``` 代码围栏。
2. 字段固定为：
   - story_state：字符串，故事现在推进到哪里（一到两句）。
   - confirmed：字符串数组，本次会话谈定的事（每条一句话）。
   - open_threads：字符串数组，尚未收口的伏笔、待办、用户提过但还没处理的问题。
   - next_step：字符串，下次接着做什么。
3. 如果某字段确实没有内容，用空字符串或空数组，不要编造。
4. 归纳时以「上一位记录员的摘要」为基线：它里面仍然成立的内容要保留在
   story_state / open_threads 中，不要因为本轮没提到就丢掉。
5. 不要复述寒暄、试错过程和被否掉的方案。
6. 不要在这里写人物设定、世界观规则等世界事实——那些必须落在世界库里；
   摘要只记「谈了什么、定了什么、还差什么」。";

/// 构造本轮摘要任务的用户输入：旧摘要（基线）+ 本次会话正文。
///
/// 旧摘要必须喂进去：否则第二次收尾只看得见本次会话，前几轮谈定的内容会静默消失。
pub fn build_summary_prompt(previous: Option<&SessionSummary>, transcript: &str) -> Result<String> {
    let baseline = match previous {
        Some(s) => serde_json::to_string_pretty(s).context("序列化上一位记录员的摘要失败")?,
        None => "（这是本项目第一次收尾，没有旧摘要）".to_string(),
    };

    Ok(format!(
        "## 上一位记录员的摘要（基线）\n{}\n\n\
         ## 本次会话正文\n{}\n\n\
         现在输出更新后的 JSON 摘要：",
        baseline, transcript
    ))
}

/// 解析模型返回的摘要 JSON。
///
/// 只容忍「``` 围栏包裹」这一种格式偏差（模型常见行为），剥壳后仍解析失败就报错，
/// 并把原始响应附在错误里——不静默返回空摘要，否则会悄悄清空作者的记录。
pub fn parse_summary_response(raw: &str) -> Result<SessionSummary> {
    let trimmed = raw.trim();
    let body = strip_code_fence(trimmed);

    serde_json::from_str::<SessionSummary>(body).with_context(|| {
        format!(
            "摘要不是合法 JSON（原始响应：{}）",
            truncate_for_error(trimmed)
        )
    })
}

/// 剥掉 ```json ... ``` / ``` ... ``` 围栏；没有围栏时原样返回。
fn strip_code_fence(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    // 去掉语言标记行（json / JSON / 空）
    let rest = match rest.find('\n') {
        Some(idx) => &rest[idx + 1..],
        None => rest,
    };
    match rest.trim_end().strip_suffix("```") {
        Some(inner) => inner.trim(),
        None => rest.trim(),
    }
}

/// 截断过长的原始响应，避免错误信息本身膨胀。
fn truncate_for_error(text: &str) -> String {
    const MAX: usize = 400;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    let head: String = text.chars().take(MAX).collect();
    format!("{}…（已截断）", head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let raw = r#"{"story_state":"主角刚拿到钥匙","confirmed":["确定用三人结局"],
            "open_threads":["反派动机未定"],"next_step":"设计第二章冲突"}"#;
        let s = parse_summary_response(raw).expect("应能解析");
        assert_eq!(s.story_state, "主角刚拿到钥匙");
        assert_eq!(s.confirmed, vec!["确定用三人结局"]);
        assert_eq!(s.open_threads, vec!["反派动机未定"]);
        assert_eq!(s.next_step, "设计第二章冲突");
    }

    #[test]
    fn parses_json_wrapped_in_code_fence() {
        let raw = "```json\n{\"story_state\":\"x\",\"confirmed\":[],\"open_threads\":[],\"next_step\":\"y\"}\n```";
        let s = parse_summary_response(raw).expect("围栏应被剥掉");
        assert_eq!(s.story_state, "x");
        assert_eq!(s.next_step, "y");
    }

    #[test]
    fn missing_fields_default_to_empty_not_error() {
        // 模型只给了 story_state：其余字段按空处理（不编造，但也不该整条失败）
        let s = parse_summary_response(r#"{"story_state":"只有状态"}"#).expect("应能解析");
        assert_eq!(s.story_state, "只有状态");
        assert!(s.confirmed.is_empty());
        assert!(s.next_step.is_empty());
    }

    #[test]
    fn invalid_json_reports_raw_response() {
        let err = parse_summary_response("我觉得故事进展得不错").expect_err("应报错");
        let msg = format!("{:#}", err);
        assert!(msg.contains("不是合法 JSON"), "错误应说明原因：{}", msg);
        assert!(msg.contains("我觉得故事进展得不错"), "错误应附原始响应：{}", msg);
    }

    #[test]
    fn prompt_contains_baseline_and_transcript() {
        let prev = SessionSummary {
            story_state: "上一轮的状态".into(),
            confirmed: vec!["旧结论".into()],
            open_threads: vec![],
            next_step: "旧下一步".into(),
        };
        let prompt = build_summary_prompt(Some(&prev), "用户：继续写").expect("构造提示词失败");
        assert!(prompt.contains("上一轮的状态"));
        assert!(prompt.contains("用户：继续写"));
    }

    #[test]
    fn prompt_marks_first_run_without_summary() {
        let prompt = build_summary_prompt(None, "用户：开写").expect("构造提示词失败");
        assert!(prompt.contains("第一次收尾"));
    }
}
