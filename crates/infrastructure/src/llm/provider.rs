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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
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
        // 请求体只需构建一次，重试时复用。
        let body = serde_json::json!({
            "model": self.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 1..=3 {
            let mut req = self.client.post(&url).header("Content-Type", "application/json").json(&body);
            if let Some(key) = &self.api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }
            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let resp_body = resp.text().await.unwrap_or_default();
                        // 5xx 视为上游瞬时故障，重试。
                        if status.is_server_error() && attempt < 3 {
                            last_err = Some(anyhow::anyhow!("LLM provider error {}: {}", status, resp_body));
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            continue;
                        }
                        return Err(anyhow::anyhow!("LLM provider error {}: {}", status, resp_body));
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
                    return Ok(LlmResponse {
                        content,
                        usage,
                        model: parsed.model.unwrap_or_else(|| self.model.clone()),
                    });
                }
                Err(e) => {
                    // 网络/超时等瞬时错误，重试。
                    last_err = Some(anyhow::anyhow!("LLM request failed: {}", e));
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("LLM generation failed after retries")))
    }

    fn name(&self) -> &str {
        "opencode"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}