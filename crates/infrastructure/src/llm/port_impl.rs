//! LlmPort 的 infrastructure 实现（提案 十 / 十一）。
//!
//! 包裹已有的 LlmClient，把领域端口 `domain::ports::LlmPort` 对接到具体 Provider。

use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;

use domain::ports::{AiSettingsPort, LlmPort, LlmStreamChunk};
use std::sync::Arc;

use crate::llm::client::LlmClient;
use crate::llm::types::{LlmRequest, Message};

/// 基于 infrastructure LlmClient 的 LlmPort 实现。
pub struct InfraLlmPort {
    client: LlmClient,
    settings: Arc<dyn AiSettingsPort>,
}

impl InfraLlmPort {
    pub fn new(client: LlmClient, settings: Arc<dyn AiSettingsPort>) -> Self {
        Self { client, settings }
    }

    /// 每次请求前读取运行时配置，使设置页保存后的 max_output_tokens 立即生效。
    async fn max_output_tokens(&self) -> u32 {
        self.settings
            .load()
            .await
            .map(|config| config.max_output_tokens)
            .unwrap_or(22_000)
    }
}

#[async_trait]
impl LlmPort for InfraLlmPort {
    async fn complete(&self, system_prompt: &str, user_prompt: &str, model: &str) -> Result<String> {
        let max_tokens = self.max_output_tokens().await;
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
            max_tokens,
            temperature: 0.7,
            model: model.to_string(),
        };
        let response = self.client.generate(request).await?;
        Ok(response.content)
    }

    async fn stream_complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        model: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>> {
        let max_tokens = self.max_output_tokens().await;
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
            max_tokens,
            temperature: 0.7,
            model: model.to_string(),
        };
        self.client.stream_generate(request).await
    }
}
