//! LLM Provider trait and implementations

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;

use super::types::{LlmRequest, LlmResponse, LlmUsage};

/// LLM Provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a response from the LLM
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse>;

    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is available
    async fn health_check(&self) -> Result<bool>;
}

/// OpenAI provider
#[allow(dead_code)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse> {
        // TODO: Implement actual OpenAI API call
        Ok(LlmResponse {
            content: "Mock response".to_string(),
            usage: super::types::LlmUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            model: self.model.clone(),
        })
    }

    fn name(&self) -> &str {
        "openai"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// DeepSeek provider
#[allow(dead_code)]
pub struct DeepSeekProvider {
    api_key: String,
    model: String,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse> {
        // TODO: Implement actual DeepSeek API call
        Ok(LlmResponse {
            content: "Mock response".to_string(),
            usage: super::types::LlmUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            model: self.model.clone(),
        })
    }

    fn name(&self) -> &str {
        "deepseek"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Local model provider (for testing)
#[allow(dead_code)]
pub struct LocalProvider {
    model_path: String,
}

impl LocalProvider {
    pub fn new(model_path: String) -> Self {
        Self { model_path }
    }
}

#[async_trait]
impl LlmProvider for LocalProvider {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse> {
        // TODO: Implement actual local model inference
        Ok(LlmResponse {
            content: "Local mock response".to_string(),
            usage: super::types::LlmUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            model: "local".to_string(),
        })
    }

    fn name(&self) -> &str {
        "local"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// OpenAI 兼容 HTTP Provider（真实调用，提案 十 / 十一 接线）。
///
/// 适用于任何 OpenAI Chat Completions 兼容网关，例如 opencode.ai、
/// vLLM、LiteLLM、OpenRouter 等。base_url 为兼容端点前缀（不含 /chat/completions）。
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            model,
        }
    }
}

#[derive(serde::Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
    model: Option<String>,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(serde::Deserialize)]
struct ChatMessage {
    content: Option<String>,
    /// 推理模型（如 mimo-v2.5）在最终 content 之前会把思考过程放在 reasoning 字段。
    #[serde(default)]
    reasoning: Option<String>,
}

#[derive(serde::Deserialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut req = self.client.post(&url).header("Content-Type", "application/json").json(
            &serde_json::json!({
                "model": self.model,
                "messages": request.messages,
                "max_tokens": request.max_tokens,
                "temperature": request.temperature,
            }),
        );
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.context("LLM request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM provider error {}: {}", status, body));
        }

        let parsed: ChatCompletionResponse =
            resp.json().await.context("Failed to parse LLM response")?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| {
                c.message
                    .content
                    .filter(|s| !s.is_empty())
                    .or_else(|| c.message.reasoning.filter(|s| !s.is_empty()))
            })
            .unwrap_or_default();
        let usage = parsed
            .usage
            .map(|u| LlmUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or(LlmUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            });

        Ok(LlmResponse {
            content,
            usage,
            model: parsed.model.unwrap_or_else(|| self.model.clone()),
        })
    }

    fn name(&self) -> &str {
        "opencode"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}