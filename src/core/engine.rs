//! Core engine for `MiniMax` CLI.
//!
//! The engine handles all AI interactions in a background task,
//! communicating with the UI via channels. This enables:
//! - Non-blocking UI during API calls
//! - Real-time streaming updates
//! - Proper cancellation support
//! - Tool execution orchestration

use std::path::PathBuf;
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::{RwLock, mpsc};
use tokio_util::sync::CancellationToken;

use crate::client::AnthropicClient;
use crate::config::Config;
use crate::models::{
    ContentBlock, ContentBlockStart, Delta, Message, MessageRequest, StreamEvent, Tool, Usage,
};
use crate::prompts;
use crate::tools::plan::PlanState;
use crate::tools::subagent::{
    SharedSubAgentManager, SubAgentRuntime, SubAgentType, new_shared_subagent_manager,
};
use crate::tools::todo::TodoList;
use crate::tools::{ToolContext, ToolRegistryBuilder};
use crate::tui::app::AppMode;

use super::events::Event;
use super::ops::Op;
use super::session::Session;
use super::turn::{TurnContext, TurnToolCall};

// === Types ===

/// Configuration for the engine
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Model identifier to use for responses.
    pub model: String,
    /// Workspace root for tool execution and file operations.
    pub workspace: PathBuf,
    /// Allow shell tool execution when true.
    pub allow_shell: bool,
    /// Enable trust mode (skip approvals) when true.
    pub trust_mode: bool,
    /// Path to the notes file used by the notes tool.
    pub notes_path: PathBuf,
    /// Path to the MCP configuration file.
    pub mcp_config_path: PathBuf,
    /// Maximum number of assistant steps before stopping.
    pub max_steps: u32,
    /// Maximum number of concurrently active subagents.
    pub max_subagents: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            model: "MiniMax-M2.1".to_string(),
            workspace: PathBuf::from("."),
            allow_shell: false,
            trust_mode: false,
            notes_path: PathBuf::from("notes.txt"),
            mcp_config_path: PathBuf::from("mcp.json"),
            max_steps: 100,
            max_subagents: 5,
        }
    }
}

/// Handle to communicate with the engine
#[derive(Clone)]
pub struct EngineHandle {
    /// Send operations to the engine
    pub tx_op: mpsc::Sender<Op>,
    /// Receive events from the engine
    pub rx_event: Arc<RwLock<mpsc::Receiver<Event>>>,
    /// Cancellation token for the current request
    cancel_token: CancellationToken,
}

impl EngineHandle {
    /// Send an operation to the engine
    pub async fn send(&self, op: Op) -> Result<()> {
        self.tx_op.send(op).await?;
        Ok(())
    }

    /// Cancel the current request
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Check if a request is currently cancelled
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}

// === Engine ===

/// The core engine that processes operations and emits events
pub struct Engine {
    config: EngineConfig,
    anthropic_client: Option<AnthropicClient>,
    session: Session,
    subagent_manager: SharedSubAgentManager,
    rx_op: mpsc::Receiver<Op>,
    tx_event: mpsc::Sender<Event>,
    cancel_token: CancellationToken,
}

// === Internal stream helpers ===

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContentBlockKind {
    Text,
    Thinking,
    ToolUse,
}

#[derive(Debug, Clone)]
struct ToolUseState {
    id: String,
    name: String,
    input: serde_json::Value,
    input_buffer: String,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: EngineConfig, api_config: &Config) -> (Self, EngineHandle) {
        let (tx_op, rx_op) = mpsc::channel(32);
        let (tx_event, rx_event) = mpsc::channel(256);
        let cancel_token = CancellationToken::new();

        // Create clients for both providers
        let anthropic_client = AnthropicClient::new(api_config).ok();

        let mut session = Session::new(
            config.model.clone(),
            config.workspace.clone(),
            config.allow_shell,
            config.trust_mode,
            config.notes_path.clone(),
            config.mcp_config_path.clone(),
        );

        // Set up system prompt with project context (default to agent mode)
        let system_prompt =
            prompts::system_prompt_for_mode_with_context(AppMode::Agent, &config.workspace);
        session.system_prompt = Some(system_prompt);

        let subagent_manager =
            new_shared_subagent_manager(config.workspace.clone(), config.max_subagents);

        let engine = Engine {
            config,
            anthropic_client,
            session,
            subagent_manager,
            rx_op,
            tx_event,
            cancel_token: cancel_token.clone(),
        };

        let handle = EngineHandle {
            tx_op,
            rx_event: Arc::new(RwLock::new(rx_event)),
            cancel_token,
        };

        (engine, handle)
    }

    /// Run the engine event loop
    #[allow(clippy::too_many_lines)]
    pub async fn run(mut self) {
        while let Some(op) = self.rx_op.recv().await {
            match op {
                Op::SendMessage { content, mode } => {
                    self.handle_send_message(content, mode).await;
                }
                Op::CancelRequest => {
                    self.cancel_token.cancel();
                    // Create a new token for the next request
                    self.cancel_token = CancellationToken::new();
                }
                Op::ApproveToolCall { id } => {
                    // Tool approval handling will be implemented in tools module
                    let _ = self
                        .tx_event
                        .send(Event::status(format!("Approved tool call: {id}")))
                        .await;
                }
                Op::DenyToolCall { id } => {
                    let _ = self
                        .tx_event
                        .send(Event::status(format!("Denied tool call: {id}")))
                        .await;
                }
                Op::SpawnSubAgent { prompt } => {
                    let Some(client) = self.anthropic_client.clone() else {
                        let _ = self
                            .tx_event
                            .send(Event::error(
                                "Failed to spawn sub-agent: no Anthropic API client configured",
                                false,
                            ))
                            .await;
                        continue;
                    };

                    let runtime = SubAgentRuntime::new(
                        client,
                        self.session.model.clone(),
                        self.build_tool_context(),
                        self.session.allow_shell,
                        Some(self.tx_event.clone()),
                    );

                    let result = self
                        .subagent_manager
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Failed to lock sub-agent manager"))
                        .and_then(|mut manager| {
                            manager.spawn_background(
                                Arc::clone(&self.subagent_manager),
                                runtime,
                                SubAgentType::General,
                                prompt.clone(),
                                None,
                            )
                        });

                    match result {
                        Ok(snapshot) => {
                            let _ = self
                                .tx_event
                                .send(Event::status(format!(
                                    "Spawned sub-agent {}",
                                    snapshot.agent_id
                                )))
                                .await;
                        }
                        Err(err) => {
                            let _ = self
                                .tx_event
                                .send(Event::error(
                                    format!("Failed to spawn sub-agent: {err}"),
                                    false,
                                ))
                                .await;
                        }
                    }
                }
                Op::ListSubAgents => {
                    let result = self
                        .subagent_manager
                        .lock()
                        .map(|manager| manager.list())
                        .map_err(|_| anyhow::anyhow!("Failed to lock sub-agent manager"));

                    match result {
                        Ok(agents) => {
                            let _ = self.tx_event.send(Event::AgentList { agents }).await;
                        }
                        Err(err) => {
                            let _ = self
                                .tx_event
                                .send(Event::error(
                                    format!("Failed to list sub-agents: {err}"),
                                    true,
                                ))
                                .await;
                        }
                    }
                }
                Op::ChangeMode { mode } => {
                    let _ = self
                        .tx_event
                        .send(Event::status(format!("Mode changed to: {mode:?}")))
                        .await;
                }
                Op::SetModel { model } => {
                    self.session.model = model;
                    self.config.model.clone_from(&self.session.model);
                    let _ = self
                        .tx_event
                        .send(Event::status(format!(
                            "Model set to: {}",
                            self.session.model
                        )))
                        .await;
                }
                Op::Shutdown => {
                    break;
                }
            }
        }
    }

    /// Handle a send message operation
    async fn handle_send_message(&mut self, content: String, mode: AppMode) {
        // Emit turn started event
        let _ = self.tx_event.send(Event::TurnStarted).await;

        // Check if we have the appropriate client
        if self.anthropic_client.is_none() {
            let _ = self
                .tx_event
                .send(Event::error(
                    "Failed to send message: no Anthropic API client configured",
                    false,
                ))
                .await;
            return;
        }

        // Add user message to session
        let user_msg = Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: content,
                cache_control: None,
            }],
        };
        self.session.add_message(user_msg);

        // Create turn context
        let mut turn = TurnContext::new(self.config.max_steps);

        // Build tool registry and tools if in agent or RLM mode
        let todo_list = Arc::new(Mutex::new(TodoList::new()));
        let plan_state = Arc::new(Mutex::new(PlanState::default()));

        let tool_registry = if matches!(mode, AppMode::Agent | AppMode::Rlm) {
            let tool_context = self.build_tool_context();
            let runtime = if let Some(client) = self.anthropic_client.clone() {
                Some(SubAgentRuntime::new(
                    client,
                    self.session.model.clone(),
                    tool_context.clone(),
                    self.session.allow_shell,
                    Some(self.tx_event.clone()),
                ))
            } else {
                None
            };
            Some(
                ToolRegistryBuilder::new()
                    .with_full_agent_tools(
                        self.session.allow_shell,
                        todo_list.clone(),
                        plan_state.clone(),
                    )
                    .with_subagent_tools(
                        self.subagent_manager.clone(),
                        runtime.expect("sub-agent runtime should exist with active client"),
                    )
                    .build(tool_context),
            )
        } else {
            None
        };

        let tools = tool_registry
            .as_ref()
            .map(super::super::tools::registry::ToolRegistry::to_api_tools);

        // Main turn loop
        self.handle_anthropic_turn(&mut turn, tool_registry.as_ref(), tools, mode)
            .await;

        // Update session usage
        self.session.total_usage.add(&turn.usage);

        // Emit turn complete event
        let _ = self
            .tx_event
            .send(Event::TurnComplete { usage: turn.usage })
            .await;
    }

    fn build_tool_context(&self) -> ToolContext {
        ToolContext::with_options(
            self.session.workspace.clone(),
            self.session.trust_mode,
            self.session.notes_path.clone(),
            self.session.mcp_config_path.clone(),
        )
    }

    /// Handle a turn using the Anthropic API (original implementation)
    #[allow(clippy::too_many_lines)]
    async fn handle_anthropic_turn(
        &mut self,
        turn: &mut TurnContext,
        tool_registry: Option<&crate::tools::ToolRegistry>,
        tools: Option<Vec<Tool>>,
        _mode: AppMode,
    ) {
        let client = self.anthropic_client.as_ref().unwrap();

        loop {
            if self.cancel_token.is_cancelled() {
                let _ = self.tx_event.send(Event::status("Request cancelled")).await;
                break;
            }

            if turn.at_max_steps() {
                let _ = self
                    .tx_event
                    .send(Event::status("Reached maximum steps"))
                    .await;
                break;
            }

            // Build the request
            let request = MessageRequest {
                model: self.session.model.clone(),
                messages: self.session.messages.clone(),
                max_tokens: 4096,
                system: self.session.system_prompt.clone(),
                tools: tools.clone(),
                tool_choice: if tools.is_some() {
                    Some(json!({ "type": "auto" }))
                } else {
                    None
                },
                metadata: None,
                thinking: None,
                stream: Some(true),
                temperature: None,
                top_p: None,
            };

            // Stream the response
            let stream_result = client.create_message_stream(request).await;
            let stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = self.tx_event.send(Event::error(e.to_string(), true)).await;
                    break;
                }
            };
            let mut stream = pin!(stream);

            // Track content blocks
            let mut content_blocks: Vec<ContentBlock> = Vec::new();
            let mut current_text = String::new();
            let mut current_thinking = String::new();
            let mut tool_uses: Vec<ToolUseState> = Vec::new();
            let mut usage = Usage {
                input_tokens: 0,
                output_tokens: 0,
            };
            let mut current_block_kind: Option<ContentBlockKind> = None;
            let mut current_tool_index: Option<usize> = None;

            // Process stream events
            while let Some(event_result) = stream.next().await {
                if self.cancel_token.is_cancelled() {
                    break;
                }

                let event = match event_result {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = self.tx_event.send(Event::error(e.to_string(), true)).await;
                        break;
                    }
                };

                match event {
                    StreamEvent::MessageStart { message } => {
                        usage = message.usage;
                    }
                    StreamEvent::ContentBlockStart {
                        index,
                        content_block,
                    } => match content_block {
                        ContentBlockStart::Text { text } => {
                            current_text = text;
                            current_block_kind = Some(ContentBlockKind::Text);
                            let _ = self
                                .tx_event
                                .send(Event::MessageStarted {
                                    index: index as usize,
                                })
                                .await;
                        }
                        ContentBlockStart::Thinking { thinking } => {
                            current_thinking = thinking;
                            current_block_kind = Some(ContentBlockKind::Thinking);
                            let _ = self
                                .tx_event
                                .send(Event::ThinkingStarted {
                                    index: index as usize,
                                })
                                .await;
                        }
                        ContentBlockStart::ToolUse { id, name, input } => {
                            current_block_kind = Some(ContentBlockKind::ToolUse);
                            current_tool_index = Some(tool_uses.len());
                            let _ = self
                                .tx_event
                                .send(Event::ToolCallStarted {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: json!({}),
                                })
                                .await;
                            tool_uses.push(ToolUseState {
                                id,
                                name,
                                input,
                                input_buffer: String::new(),
                            });
                        }
                    },
                    StreamEvent::ContentBlockDelta { index, delta } => match delta {
                        Delta::TextDelta { text } => {
                            current_text.push_str(&text);
                            if !text.is_empty() {
                                let _ = self
                                    .tx_event
                                    .send(Event::MessageDelta {
                                        index: index as usize,
                                        content: text,
                                    })
                                    .await;
                            }
                        }
                        Delta::ThinkingDelta { thinking } => {
                            current_thinking.push_str(&thinking);
                            if !thinking.is_empty() {
                                let _ = self
                                    .tx_event
                                    .send(Event::ThinkingDelta {
                                        index: index as usize,
                                        content: thinking,
                                    })
                                    .await;
                            }
                        }
                        Delta::InputJsonDelta { partial_json } => {
                            if let Some(index) = current_tool_index
                                && let Some(tool_state) = tool_uses.get_mut(index)
                            {
                                tool_state.input_buffer.push_str(&partial_json);
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(
                                    &tool_state.input_buffer,
                                ) {
                                    tool_state.input = value;
                                }
                            }
                        }
                    },
                    StreamEvent::ContentBlockStop { index } => {
                        let stopped_kind = current_block_kind.take();
                        match stopped_kind {
                            Some(ContentBlockKind::Text) => {
                                let _ = self
                                    .tx_event
                                    .send(Event::MessageComplete {
                                        index: index as usize,
                                    })
                                    .await;
                            }
                            Some(ContentBlockKind::Thinking) => {
                                let _ = self
                                    .tx_event
                                    .send(Event::ThinkingComplete {
                                        index: index as usize,
                                    })
                                    .await;
                            }
                            Some(ContentBlockKind::ToolUse) | None => {}
                        }
                        if matches!(stopped_kind, Some(ContentBlockKind::ToolUse))
                            && let Some(index) = current_tool_index.take()
                            && let Some(tool_state) = tool_uses.get_mut(index)
                            && !tool_state.input_buffer.trim().is_empty()
                        {
                            if let Ok(value) =
                                serde_json::from_str::<serde_json::Value>(&tool_state.input_buffer)
                            {
                                tool_state.input = value;
                            } else {
                                let _ = self
                                    .tx_event
                                    .send(Event::status(format!(
                                        "Failed to parse tool input JSON for {}",
                                        tool_state.name
                                    )))
                                    .await;
                            }
                        }
                    }
                    StreamEvent::MessageDelta {
                        usage: delta_usage, ..
                    } => {
                        if let Some(u) = delta_usage {
                            usage = u;
                        }
                    }
                    StreamEvent::MessageStop | StreamEvent::Ping => {}
                }
            }

            // Update turn usage
            turn.add_usage(&usage);

            // Build content blocks
            if !current_thinking.is_empty() {
                content_blocks.push(ContentBlock::Thinking {
                    thinking: current_thinking.clone(),
                });
            }
            if !current_text.is_empty() {
                content_blocks.push(ContentBlock::Text {
                    text: current_text.clone(),
                    cache_control: None,
                });
            }
            for tool in &tool_uses {
                content_blocks.push(ContentBlock::ToolUse {
                    id: tool.id.clone(),
                    name: tool.name.clone(),
                    input: tool.input.clone(),
                });
            }

            // Add assistant message to session
            if !content_blocks.is_empty() {
                self.session.add_message(Message {
                    role: "assistant".to_string(),
                    content: content_blocks,
                });
            }

            // If no tool uses, we're done
            if tool_uses.is_empty() {
                break;
            }

            // Execute tools
            if let Some(registry) = tool_registry {
                for tool in &tool_uses {
                    let tool_id = &tool.id;
                    let tool_name = &tool.name;
                    let tool_input = tool.input.clone();
                    let start = Instant::now();

                    let result = registry.execute_full(tool_name, tool_input.clone()).await;
                    let duration = start.elapsed();

                    let mut tool_call =
                        TurnToolCall::new(tool_id.clone(), tool_name.clone(), tool_input.clone());

                    match result {
                        Ok(output) => {
                            tool_call.set_result(output.content.clone(), duration);
                            let _ = self
                                .tx_event
                                .send(Event::ToolCallComplete {
                                    id: tool_id.clone(),
                                    name: tool_name.clone(),
                                    result: Ok(output.clone()),
                                })
                                .await;

                            self.session.add_message(Message {
                                role: "user".to_string(),
                                content: vec![ContentBlock::ToolResult {
                                    tool_use_id: tool_id.clone(),
                                    content: output.content,
                                }],
                            });
                        }
                        Err(e) => {
                            let error = e.to_string();
                            tool_call.set_error(error.clone(), duration);
                            let _ = self
                                .tx_event
                                .send(Event::ToolCallComplete {
                                    id: tool_id.clone(),
                                    name: tool_name.clone(),
                                    result: Err(e),
                                })
                                .await;

                            self.session.add_message(Message {
                                role: "user".to_string(),
                                content: vec![ContentBlock::ToolResult {
                                    tool_use_id: tool_id.clone(),
                                    content: format!("Error: {error}"),
                                }],
                            });
                        }
                    }

                    turn.record_tool_call(tool_call);
                }
            }

            turn.next_step();
        }
    }

    /// Get a reference to the session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get a mutable reference to the session
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

/// Spawn the engine in a background task
pub fn spawn_engine(config: EngineConfig, api_config: &Config) -> EngineHandle {
    let (engine, handle) = Engine::new(config, api_config);

    tokio::spawn(async move {
        engine.run().await;
    });

    handle
}
