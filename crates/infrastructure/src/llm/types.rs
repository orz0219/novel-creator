//! LLM types

use serde::{Deserialize, Serialize};

/// LLM Request
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub model: String,
}

/// Message in LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// LLM Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: LlmUsage,
    pub model: String,
}

/// LLM Usage statistics
///
/// 直接复用领域层的类型，避免同一份用量数据存在两个定义、字段逐渐漂移。
pub use domain::ports::LlmUsage;
