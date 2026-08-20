//! LlmPort 的 infrastructure 实现（提案 十 / 十一）。
//!
//! 包裹已有的 LlmClient，把领域端口 `domain::ports::LlmPort` 对接到具体 Provider。

use anyhow::Result;
use async_trait::async_trait;
use domain::ports::LlmPort;

use crate::llm::client::LlmClient;
use crate::llm::types::{LlmRequest, Message};

/// 基于 infrastructure LlmClient 的 LlmPort 实现。
pub struct InfraLlmPort {
    client: LlmClient,
}

impl InfraLlmPort {
    pub fn new(client: LlmClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LlmPort for InfraLlmPort {
    async fn complete(&self, system_prompt: &str, user_prompt: &str, model: &str) -> Result<String> {
        let request = LlmRequest {
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            max_tokens: 4096,
            temperature: 0.7,
        };
        let _ = model; // 具体 model 由 provider 配置决定；此处保留接口兼容
        let response = self.client.generate(request).await?;
        Ok(response.content)
    }
}
