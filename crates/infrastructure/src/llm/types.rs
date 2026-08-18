//! LLM types

use serde::{Deserialize, Serialize};

/// LLM Request
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: f32,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
