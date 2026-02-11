//! HTTP clients for LLM providers.
//!
//! This module centralizes retry behavior, base URLs, and streaming helpers
//! for the Axiom CLI's network requests.

use std::pin::Pin;

use anyhow::Result;
use futures_util::StreamExt;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};

use crate::config::{Config, RetryPolicy};
use crate::llm_client::{LlmClient, StreamEventBox};
use crate::logging;
use crate::models::{
    ContentBlock, Message, MessageRequest, MessageResponse, ModelListResponse, StreamEvent,
};

// === Types ===

pub fn test_connection_sync(base_url: &str, api_key: &str) -> Result<()> {
    let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

    let request = MessageRequest {
        model: "model-01".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: "test".to_string(),
                cache_control: None,
            }],
        }],
        system: None,
        tools: None,
        tool_choice: None,
        metadata: None,
        thinking: None,
        stream: Some(false),
        temperature: None,
        top_p: None,
        max_tokens: 10,
    };

    let response = client.post(&url).json(&request).send()?;

    if !response.status().is_success() {
        let status = response.status();
        let _text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Connection test failed: HTTP {}", status.as_u16());
    }

    Ok(())
}

/// Client for Anthropic-compatible API requests.
#[derive(Clone)]
#[must_use]
pub struct AnthropicClient {
    http_client: reqwest::Client,
    base_url: String,
    retry: RetryPolicy,
    #[allow(dead_code)]
    default_model: String,
}

// === AnthropicClient ===

impl AnthropicClient {
    /// Create an Anthropic-compatible client using the default model.
    pub fn new(config: &Config) -> Result<Self> {
        let model = config
            .default_model
            .clone()
            .unwrap_or_else(|| "anthropic/claude-3-5-sonnet-20241022".to_string());
        Self::with_model(config, model)
    }

    /// Create an Anthropic-compatible client pinned to a specific model.
    pub fn with_model(config: &Config, model: String) -> Result<Self> {
        let base_url = config.anthropic_base_url();
        let api_key = config.anthropic_api_key()?;
        let retry = config.retry_policy();

        logging::info(format!("Compatible base URL: {base_url}"));
        logging::info(format!(
            "Retry policy: enabled={}, max_retries={}, initial_delay={}s, max_delay={}s",
            retry.enabled, retry.max_retries, retry.initial_delay, retry.max_delay
        ));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("x-api-key", HeaderValue::from_str(&api_key)?);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http_client,
            base_url,
            retry,
            default_model: model,
        })
    }

    /// Get the default model name
    #[allow(dead_code)] // For future model selection
    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    /// Create a non-streaming Anthropic-compatible message request.
    pub async fn create_message(&self, request: MessageRequest) -> Result<MessageResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut request = request;
        request.stream = Some(false);

        let response =
            send_with_retry(&self.retry, || self.http_client.post(&url).json(&request)).await?;
        Ok(response.json::<MessageResponse>().await?)
    }

    /// Create a streaming Anthropic-compatible message request.
    pub async fn create_message_stream(
        &self,
        request: MessageRequest,
    ) -> Result<impl futures_util::Stream<Item = Result<StreamEvent>>> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut request = request;
        request.stream = Some(true);

        let response =
            send_with_retry(&self.retry, || self.http_client.post(&url).json(&request)).await?;

        Ok(parse_sse_stream(response.bytes_stream()))
    }

    /// List available models from the API
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list models: HTTP {}: {}", status, text);
        }

        let result: ModelListResponse = response.json().await?;

        Ok(result.models.into_iter().map(|m| m.id).collect())
    }
}

// === Retry + Streaming Helpers ===

async fn send_with_retry<F>(policy: &RetryPolicy, mut build: F) -> Result<reqwest::Response>
where
    F: FnMut() -> reqwest::RequestBuilder,
{
    let mut attempt: u32 = 0;

    loop {
        let result = build().send().await;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(response);
                }

                let status = response.status();
                let retryable = status.as_u16() == 429 || status.is_server_error();

                if !policy.enabled || !retryable || attempt >= policy.max_retries {
                    let text = response
                        .text()
                        .await
                        .unwrap_or_else(|e| format!("(failed to read body: {e})"));
                    anyhow::bail!("Failed to send API request: HTTP {status}: {text}");
                }
                logging::warn(format!(
                    "Retryable HTTP {} (attempt {} of {})",
                    status.as_u16(),
                    attempt + 1,
                    policy.max_retries + 1
                ));
            }
            Err(err) => {
                if !policy.enabled || attempt >= policy.max_retries {
                    return Err(err.into());
                }
                logging::warn(format!(
                    "Request error: {} (attempt {} of {})",
                    err,
                    attempt + 1,
                    policy.max_retries + 1
                ));
            }
        }

        let delay = policy.delay_for_attempt(attempt);
        attempt += 1;
        logging::info(format!("Retrying after {:.2}s", delay.as_secs_f64()));
        tokio::time::sleep(delay).await;
    }
}

/// Parse an SSE stream into structured stream events.
fn parse_sse_stream(
    stream: impl futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
) -> impl futures_util::Stream<Item = Result<StreamEvent>> {
    async_stream::try_stream! {
        let mut buffer = String::new();
        let mut stream = stream;

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(chunk) => chunk,
                Err(err) => {
                    logging::warn(format!("SSE stream chunk error: {err}"));
                    continue;
                }
            };
            let s = String::from_utf8_lossy(&chunk);
            buffer.push_str(&s);

            while let Some(pos) = buffer.find("\n\n") {
                let block = buffer[..pos].to_string();
                buffer.drain(..pos + 2);

                for line in block.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return;
                        }
                        // Log raw SSE data for debugging
                        if data.contains("tool_use") || data.contains("input_json") {
                            logging::info(format!("SSE tool event: {}", data));
                        }
                        match serde_json::from_str::<StreamEvent>(data) {
                            Ok(event) => yield event,
                            Err(err) => {
                                logging::warn(format!("Failed to parse SSE event: {err}"));
                                logging::warn(format!("Raw SSE data: {data}"));
                            }
                        }
                    }
                }
            }
        }
    }
}

// === Trait Implementations ===

impl LlmClient for AnthropicClient {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.default_model
    }

    async fn create_message(&self, request: MessageRequest) -> Result<MessageResponse> {
        // Delegate to existing method
        AnthropicClient::create_message(self, request).await
    }

    async fn create_message_stream(&self, request: MessageRequest) -> Result<StreamEventBox> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut request = request;
        request.stream = Some(true);

        let response =
            send_with_retry(&self.retry, || self.http_client.post(&url).json(&request)).await?;

        let stream = parse_sse_stream(response.bytes_stream());
        Ok(Pin::from(Box::new(stream)))
    }
}
