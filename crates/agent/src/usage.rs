//! 会话上下文用量估算（聊天页用量预警用）。
//!
//! 说明：网关的 `GET /models` 只返回 id / object / created / owned_by，
//! **不含上下文长度**，因此这里给出的是估算：
//!
//! - `used_tokens`：按字符类型近似（CJK 约 1 token/字，其余约 4 字符/token），
//!   另加每条消息的协议开销。用于「快超了」的提醒，不是精确计费值。
//! - `limit_tokens`：来自设置页配置的**预算**，而非模型硬上限的权威声明。
//!
//! 这样设计的原因：超限时网关会直接报错（整段会话历史会被拍平成一次请求），
//! 提前显示余量比事后看报错更有用。

use serde::Serialize;

/// 每条消息在请求中的固定协议开销（role / 分隔标记等），单位 token。
const PER_MESSAGE_OVERHEAD: usize = 4;

/// 单次对话的上下文用量。
#[derive(Debug, Clone, Serialize)]
pub struct ContextUsage {
    /// 已估算使用的 token 数。
    pub used_tokens: usize,
    /// 配置的上下文预算（token）。
    pub limit_tokens: usize,
    /// 会话消息条数。
    pub message_count: usize,
    /// 当前生效模型名。
    pub model: String,
}

/// 估算一段文本的 token 数（近似值，用于预警）。
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if is_cjk(ch) {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    cjk + other.div_ceil(4)
}

/// 估算一条消息（含协议开销）占用的 token。
pub fn estimate_message_tokens(content: &str) -> usize {
    estimate_tokens(content) + PER_MESSAGE_OVERHEAD
}

/// 是否为 CJK / 全角字符（这类字符的 token 密度接近 1 token/字）。
fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x2E80..=0x9FFF      // 部首扩展 + CJK 标点(0x3000 段) + CJK 统一表意文字
        | 0xF900..=0xFAFF    // CJK 兼容表意文字
        | 0xFF00..=0xFFEF    // 全角字符
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_counts_about_one_token_per_char() {
        assert_eq!(estimate_tokens("你好世界"), 4);
    }

    #[test]
    fn ascii_counts_about_four_chars_per_token() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 / 4 向上取整
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn mixed_text_adds_both_parts() {
        // 2 个汉字 + 4 个 ASCII = 2 + 1
        assert_eq!(estimate_tokens("你好abcd"), 3);
    }

    #[test]
    fn message_tokens_add_protocol_overhead() {
        assert_eq!(estimate_message_tokens("你好"), 2 + PER_MESSAGE_OVERHEAD);
    }
}
