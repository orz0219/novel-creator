//! LLM Client for managing providers and requests

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

use super::provider::LlmProvider;
use super::types::{LlmRequest, LlmResponse};

/// LLM Client - manages provider selection and request handling
pub struct LlmClient {
    providers: Vec<Arc<dyn LlmProvider>>,
    default_provider: String,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(default_provider: String) -> Self {
        Self {
            providers: Vec::new(),
            default_provider,
        }
    }

    /// Add a provider
    pub fn add_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.providers.push(provider);
    }

    /// Generate a response using the default provider
    pub async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
        let provider = self.find_provider(&self.default_provider)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", self.default_provider))?;

        info!("Generating with provider: {}", provider.name());
        provider.generate(request).await
    }

    /// Generate a response using a specific provider
    pub async fn generate_with_provider(&self, provider_name: &str, request: LlmRequest) -> Result<LlmResponse> {
        let provider = self.find_provider(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider_name))?;

        info!("Generating with provider: {}", provider.name());
        provider.generate(request).await
    }

    /// Find a provider by name
    fn find_provider(&self, name: &str) -> Option<Arc<dyn LlmProvider>> {
        self.providers.iter().find(|p| p.name() == name).cloned()
    }

    /// Check health of all providers
    pub async fn health_check(&self) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();
        for provider in &self.providers {
            let healthy = provider.health_check().await.unwrap_or(false);
            results.push((provider.name().to_string(), healthy));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::LlmUsage;
    use async_trait::async_trait;

    /// 测试用假 Provider：实现 LlmProvider 但不触网，替换已删除的 LocalProvider。
    struct MockProvider {
        name: String,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse> {
            Ok(LlmResponse {
                content: "mock".to_string(),
                usage: LlmUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: self.name.clone(),
            })
        }

        fn name(&self) -> &str {
            &self.name
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_llm_client() {
        let mut client = LlmClient::new("mock".to_string());
        client.add_provider(Arc::new(MockProvider {
            name: "mock".to_string(),
        }));

        let request = LlmRequest {
            messages: vec![],
            max_tokens: 100,
            temperature: 0.7,
        };

        let response = client.generate(request).await.unwrap();
        assert_eq!(response.model, "mock");
    }

    #[tokio::test]
    async fn test_llm_client_health_check() {
        let mut client = LlmClient::new("mock".to_string());
        client.add_provider(Arc::new(MockProvider {
            name: "mock".to_string(),
        }));

        let results = client.health_check().await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1);
    }
}