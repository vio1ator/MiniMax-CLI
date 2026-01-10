use crate::config::{Config, RetryPolicy};
use crate::logging;
use crate::models::{MessageRequest, MessageResponse, StreamEvent};
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

pub struct MiniMaxClient {
    http_client: reqwest::Client,
    base_url: String,
    retry: RetryPolicy,
}

pub struct AnthropicClient {
    http_client: reqwest::Client,
    base_url: String,
    retry: RetryPolicy,
}

impl MiniMaxClient {
    pub fn new(config: &Config) -> Result<Self> {
        let api_key = config.minimax_api_key()?;
        let base_url = config.minimax_base_url();
        let retry = config.retry_policy();

        logging::info(format!("MiniMax base URL: {}", base_url));
        logging::info(format!(
            "Retry policy: enabled={}, max_retries={}, initial_delay={}s, max_delay={}s",
            retry.enabled, retry.max_retries, retry.initial_delay, retry.max_delay
        ));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http_client,
            base_url,
            retry,
        })
    }

    pub async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T> {
        let url = self.url(path);
        let response = send_with_retry(&self.retry, || {
            self.http_client.post(&url).json(body)
        })
        .await?;
        self.parse_json_response(response).await
    }

    pub async fn post_json_raw(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<reqwest::Response> {
        let url = self.url(path);
        let response = send_with_retry(&self.retry, || {
            self.http_client.post(&url).json(body)
        })
        .await?;
        Ok(response)
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: Option<&[(&str, &str)]>,
    ) -> Result<T> {
        let url = self.url(path);
        let response = if let Some(query) = query {
            let mut url = reqwest::Url::parse(&url)?;
            url.query_pairs_mut().extend_pairs(query.iter().cloned());
            send_with_retry(&self.retry, || self.http_client.get(url.clone())).await?
        } else {
            send_with_retry(&self.retry, || self.http_client.get(&url)).await?
        };
        self.parse_json_response(response).await
    }

    pub async fn post_multipart<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<T> {
        let url = self.url(path);
        let response = self.http_client.post(&url).multipart(form).send().await?;
        self.parse_json_response(response).await
    }

    pub async fn get_bytes(&self, url: &str) -> Result<bytes::Bytes> {
        let response = send_with_retry(&self.retry, || self.http_client.get(url)).await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, text);
        }
        Ok(response.bytes().await?)
    }

    pub async fn get_bytes_with_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<(bytes::Bytes, Option<String>)> {
        let url = self.url(path);
        let mut url = reqwest::Url::parse(&url)?;
        url.query_pairs_mut().extend_pairs(query.iter().cloned());
        let response = send_with_retry(&self.retry, || self.http_client.get(url.clone())).await?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());
        Ok((response.bytes().await?, content_type))
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!(
                "{}/{}",
                self.base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            )
        }
    }

    async fn parse_json_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed with status {}: {}", status, text);
        }
        Ok(response.json::<T>().await?)
    }
}

impl AnthropicClient {
    pub fn new(config: &Config) -> Result<Self> {
        let base_url = config.anthropic_base_url();
        let api_key = config.anthropic_api_key()?;
        let retry = config.retry_policy();

        logging::info(format!("Anthropic base URL: {}", base_url));
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
        })
    }

    pub async fn create_message(&self, request: MessageRequest) -> Result<MessageResponse> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut request = request;
        request.stream = Some(false);

        let response = send_with_retry(&self.retry, || {
            self.http_client.post(&url).json(&request)
        })
        .await?;
        Ok(response.json::<MessageResponse>().await?)
    }

    pub async fn create_message_stream(
        &self,
        request: MessageRequest,
    ) -> Result<impl futures_util::Stream<Item = Result<StreamEvent>>> {
        let url = format!("{}/v1/messages", self.base_url);
        let mut request = request;
        request.stream = Some(true);

        let response = send_with_retry(&self.retry, || {
            self.http_client.post(&url).json(&request)
        })
        .await?;

        Ok(parse_sse_stream(response.bytes_stream()))
    }
}

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
                    let text = response.text().await.unwrap_or_default();
                    anyhow::bail!("API request failed with status {}: {}", status, text);
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

fn parse_sse_stream(
    stream: impl futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
) -> impl futures_util::Stream<Item = Result<StreamEvent>> {
    async_stream::try_stream! {
        let mut buffer = String::new();
        let mut stream = stream;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let s = String::from_utf8_lossy(&chunk);
            buffer.push_str(&s);

            while let Some(pos) = buffer.find("\n\n") {
                let block = buffer[..pos].to_string();
                buffer.drain(..pos + 2);

                for line in block.lines() {
                    if line.starts_with("data: ") {
                        let data = &line["data: ".len()..];
                        if data == "[DONE]" {
                            break;
                        }
                        match serde_json::from_str::<StreamEvent>(data) {
                            Ok(event) => yield event,
                            Err(_e) => {}
                        }
                    }
                }
            }
        }
    }
}
