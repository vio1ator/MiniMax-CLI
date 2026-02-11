//! Generic LLM provider abstraction layer.
//!
//! This module provides a unified interface for different LLM providers
//! (Anthropic, OpenAI, and compatible APIs).

use anyhow::Result;
use futures_util::StreamExt;
use std::pin::Pin;

use crate::llm_client::{LlmClient, StreamEventBox};
use crate::models::{MessageRequest, MessageResponse, StreamEvent};

/// Generic trait for any LLM provider.
///
/// This trait abstracts over different LLM API providers, allowing the application
/// to work with any provider that implements this interface.
pub trait LlmProvider: Send + Sync {
     /// Returns the provider name (e.g., "anthropic", "openai")
    fn provider_name(&self) -> &'static str;

    /// Returns the model identifier being used
    fn model(&self) -> &str;

    /// Creates a non-streaming message completion
    fn create_message(&self, request: MessageRequest) -> impl std::future::Future<Output = Result<MessageResponse>> + Send;

    /// Creates a streaming message completion
    fn create_message_stream(&self, request: MessageRequest) -> impl std::future::Future<Output = Result<StreamEventBox>> + Send;
}

/// A generic provider that implements the LlmProvider trait for Anthropic-compatible APIs.
///
/// This provider can work with any API that follows the Anthropic message API spec,
/// including OpenAI-compatible endpoints.
#[derive(Clone)]
pub struct GenericProvider<C: LlmClient> {
    client: C,
}

impl<C: LlmClient> GenericProvider<C> {
    /// Create a new generic provider from an existing LlmClient
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Get a reference to the underlying client
    pub fn client(&self) -> &C {
        &self.client
    }

    /// Get a mutable reference to the underlying client
    pub fn client_mut(&mut self) -> &mut C {
        &mut self.client
    }
}

impl<C: LlmClient> LlmProvider for GenericProvider<C> {
    fn provider_name(&self) -> &'static str {
        self.client.provider_name()
    }

    fn model(&self) -> &str {
        self.client.model()
    }

    async fn create_message(&self, request: MessageRequest) -> Result<MessageResponse> {
        self.client.create_message(request).await
    }

    async fn create_message_stream(&self, request: MessageRequest) -> Result<StreamEventBox> {
        self.client.create_message_stream(request).await
    }
}
