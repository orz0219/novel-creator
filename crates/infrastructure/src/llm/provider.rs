//! LLM Provider trait and implementations

use anyhow::Result;
use async_trait::async_trait;

use super::types::{LlmRequest, LlmResponse};

/// LLM Provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a response from the LLM
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse>;

    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is available
    async fn health_check(&self) -> Result<bool>;
}

/// OpenAI provider
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
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
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
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
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
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
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
