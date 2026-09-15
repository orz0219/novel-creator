//! LlmPort 的 infrastructure 实现（提案 十 / 十一）。
//!
//! 包裹已有的 LlmClient，把领域端口 `domain::ports::LlmPort` 对接到具体 Provider。

use anyhow::{Context, Result};
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
    ///
    /// 读不到配置**直接报错**：这里原先兜底成 22000，于是设置页里写的 50000
    /// 会在读取失败时被静默忽略，用 22000 去发请求——请求「成功」但输出莫名变短，
    /// 排查时完全看不出是配置没读到。
    async fn max_output_tokens(&self) -> Result<u32> {
        let config = self
            .settings
            .load()
            .await
            .context("读取运行时 AI 配置失败（max_output_tokens）")?;
        Ok(config.max_output_tokens)
    }
}

#[async_trait]
impl LlmPort for InfraLlmPort {
    async fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        model: &str,
        temperature: f32,
    ) -> Result<String> {
        let max_tokens = self.max_output_tokens().await?;
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
            // 温度由调用方按用途给出（逻辑类低、文学类高），此处不再写死 0.7：
            // 同一个值伺候"严密的细纲"和"有文采的正文"，两头都不讨好。
            temperature,
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
        temperature: f32,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>> {
        let max_tokens = self.max_output_tokens().await?;
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
            // 温度由调用方按用途给出（逻辑类低、文学类高），此处不再写死 0.7：
            // 同一个值伺候"严密的细纲"和"有文采的正文"，两头都不讨好。
            temperature,
            model: model.to_string(),
        };
        self.client.stream_generate(request).await
    }
}
