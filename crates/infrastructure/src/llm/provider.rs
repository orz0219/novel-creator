//! LLM Provider trait and implementations

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;
use async_stream::stream;

use super::types::{LlmRequest, LlmResponse, LlmUsage};
use domain::ports::{AiRuntimeConfig, AiSettingsPort, LlmStreamChunk};
use std::sync::Arc;

/// 写入原始 LLM 流式日志，用于事后核对“provider 到底发了什么”。
///
/// 默认路径：`tmp/llm_streams.jsonl`；可用环境变量 `NOVEL_LLM_STREAM_LOG` 覆盖。
/// 写入失败不影响请求（日志只是诊断用途）。
fn append_llm_stream_log(record: &serde_json::Value) {
    use std::io::Write;

    let path = std::env::var("NOVEL_LLM_STREAM_LOG")
        .unwrap_or_else(|_| "tmp/llm_streams.jsonl".to_string());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{}", record);
    }
}

/// 流式输出的流类型：逐段产出正文 token，并在末尾附带用量统计。
pub type TokenStream = Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>;

/// LLM Provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a response from the LLM
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse>;

    /// 流式生成：逐段产出 token 的流。默认实现直接报错，由具体 provider 覆盖。
    async fn stream_generate(&self, _request: LlmRequest) -> Result<TokenStream> {
        anyhow::bail!("该 provider 未实现 stream_generate")
    }

    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is available
    async fn health_check(&self) -> Result<bool>;
}

/// 固定配置的 [`AiSettingsPort`]：用于「测试连接」等一次性调用，
/// 让调用方直接指定一组（尚未保存的）网关参数。
pub struct StaticAiSettings {
    config: AiRuntimeConfig,
}

impl StaticAiSettings {
    pub fn new(config: AiRuntimeConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AiSettingsPort for StaticAiSettings {
    async fn load(&self) -> Result<AiRuntimeConfig> {
        Ok(self.config.clone())
    }
}

/// OpenAI 兼容 HTTP Provider（真实调用，提案 十 / 十一 接线）。
///
/// 适用于任何 OpenAI Chat Completions 兼容网关，例如 opencode.ai、
/// vLLM、LiteLLM、OpenRouter 等。base_url 为兼容端点前缀（不含 /chat/completions）。
///
/// 网关参数（base_url / api_key / model）**不在此固化**：每次请求前通过
/// [`AiSettingsPort`] 读取当前设置，因此设置页改完立即生效。
/// 请求自带的 `model` 非空时优先于配置中的模型名。
pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    settings: Arc<dyn AiSettingsPort>,
    /// opencode.ai「Console Go」网关要求的会话路由头 `x-opencode-session`；
    /// 缺失时网关直接返回 400 MissingSessionID。同一进程复用同一值以保持路由亲和。
    session_id: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(settings: Arc<dyn AiSettingsPort>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            settings,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// 解析本次请求实际使用的网关参数。
    async fn resolve(&self, requested_model: &str) -> Result<AiRuntimeConfig> {
        let mut config = self.settings.load().await?;
        if !requested_model.is_empty() {
            config.model = requested_model.to_string();
        }
        Ok(config)
    }

    /// 拉取网关可用模型列表（OpenAI 兼容 `GET /models`）。
    ///
    /// 只在当前配置的 base_url / api_key 上查询，与生成请求同一套参数。
    /// 网关未实现该端点时返回包含状态码与响应体的明确错误。
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let config = self.resolve("").await?;
        let url = format!("{}/models", config.base_url.trim_end_matches('/'));

        let mut req = self
            .client
            .get(&url)
            .header("x-opencode-session", &self.session_id);
        if let Some(key) = &config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("请求模型列表失败：{}", e))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("获取模型列表失败 {}: {}", status, body));
        }

        let parsed: ModelListResponse = resp.json().await.context("解析模型列表响应失败")?;
        Ok(parsed.data.into_iter().map(|m| m.id).collect())
    }
}

/// OpenAI 兼容 `GET /models` 响应体。
#[derive(serde::Deserialize)]
struct ModelListResponse {
    data: Vec<ModelEntry>,
}

#[derive(serde::Deserialize)]
struct ModelEntry {
    id: String,
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
    /// DeepSeek 系网关的缓存命中字段。
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
    /// OpenAI 风格的缓存命中字段。
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(serde::Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

impl ChatUsage {
    /// 统一取「命中缓存的 prompt token 数」——两种网关字段名不同。
    fn cached_tokens(&self) -> Option<u32> {
        self.prompt_cache_hit_tokens.or_else(|| {
            self.prompt_tokens_details
                .as_ref()
                .and_then(|d| d.cached_tokens)
        })
    }

    fn into_usage(self) -> LlmUsage {
        let cached = self.cached_tokens();
        LlmUsage {
            prompt_tokens: self.prompt_tokens,
            completion_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
            cached_tokens: cached,
        }
    }
}

/// 流式响应分片
#[derive(serde::Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    /// 用量统计只出现在最后一个分片（需请求时带 `stream_options.include_usage`）。
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(serde::Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
    /// `stop` / `length` / `content_filter` 等；`length` 说明输出被 max_tokens 截断。
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(serde::Deserialize, Default)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
        let config = self.resolve(&request.model).await?;
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
        // 请求体只需构建一次，重试时复用。
        let body = serde_json::json!({
            "model": config.model.clone(),
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 1..=3 {
            let mut req = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("x-opencode-session", &self.session_id)
                .json(&body);
            if let Some(key) = &config.api_key {
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
                        .map(|u| u.into_usage())
                        .unwrap_or(LlmUsage {
                            prompt_tokens: 0,
                            completion_tokens: 0,
                            total_tokens: 0,
                            cached_tokens: None,
                        });
                    return Ok(LlmResponse {
                        content,
                        usage,
                        model: parsed.model.unwrap_or_else(|| config.model.clone()),
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

    async fn stream_generate(&self, request: LlmRequest) -> Result<TokenStream> {
        use futures::StreamExt;
        let config = self.resolve(&request.model).await?;
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
        let stream_id = uuid::Uuid::new_v4().to_string();
        let gateway_session_id = self.session_id.clone();
        let log_model = config.model.clone();
        tracing::info!(
            model = %log_model,
            max_tokens = request.max_tokens,
            stream_id = %stream_id,
            "LLM stream request"
        );
        append_llm_stream_log(&serde_json::json!({
            "time": chrono::Utc::now().to_rfc3339(),
            "stream_id": stream_id,
            "kind": "request",
            "model": log_model,
            "max_tokens": request.max_tokens,
            "gateway_session_id": gateway_session_id,
        }));
        let body = serde_json::json!({
            "model": config.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": true,
            // 要求网关在流末尾附上用量统计（含缓存命中数）；不加这一项则拿不到。
            "stream_options": { "include_usage": true },
        });
        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-opencode-session", &self.session_id)
            .json(&body);
        if let Some(key) = &config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let resp = req.send().await.map_err(|e| anyhow::anyhow!("LLM stream request failed: {}", e))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM provider error {}: {}", status, body_text));
        }
        let mut byte_stream = resp.bytes_stream();
        let s = stream! {
            let stream_id = stream_id;
            let mut buf = String::new();
            while let Some(chunk) = byte_stream.next().await {
                let bytes = match chunk {
                    Ok(b) => b,
                    Err(e) => { yield Err(anyhow::anyhow!("LLM stream read error: {}", e)); return; }
                };
                buf.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(nl) = buf.find('\n') {
                    let line = buf[..nl].trim().to_string();
                    buf = buf[nl + 1..].to_string();
                    if line.is_empty() { continue; }
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        // 原始 SSE 数据先落日志：这是“provider 到底发了什么”的事实层。
                        append_llm_stream_log(&serde_json::json!({
                            "time": chrono::Utc::now().to_rfc3339(),
                            "stream_id": stream_id.clone(),
                            "kind": "data",
                            "data": data,
                        }));
                        if data == "[DONE]" {
                            append_llm_stream_log(&serde_json::json!({
                                "time": chrono::Utc::now().to_rfc3339(),
                                "stream_id": stream_id.clone(),
                                "kind": "done",
                            }));
                            return;
                        }
                        match serde_json::from_str::<StreamChunk>(data) {
                            Ok(parsed) => {
                                // 用量统计（含缓存命中）通常只在最后一个分片出现
                                if let Some(usage) = parsed.usage {
                                    yield Ok(LlmStreamChunk::Usage(usage.into_usage()));
                                }
                                if let Some(choice) = parsed.choices.into_iter().next() {
                                    if let Some(content) = choice.delta.content {
                                        if !content.is_empty() {
                                            yield Ok(LlmStreamChunk::Token(content));
                                        }
                                    }
                                    if let Some(reason) = choice.finish_reason {
                                        if !reason.is_empty() {
                                            yield Ok(LlmStreamChunk::Finish(reason));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                append_llm_stream_log(&serde_json::json!({
                                    "time": chrono::Utc::now().to_rfc3339(),
                                    "stream_id": stream_id.clone(),
                                    "kind": "parse_error",
                                    "error": e.to_string(),
                                    "data": data,
                                }));
                            }
                        }
                    }
                }
            }
            append_llm_stream_log(&serde_json::json!({
                "time": chrono::Utc::now().to_rfc3339(),
                "stream_id": stream_id.clone(),
                "kind": "eof",
            }));
        };
        Ok(Box::pin(s))
    }

    fn name(&self) -> &str {
        "opencode"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}