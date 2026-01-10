#![allow(dead_code)]

use crate::client::AnthropicClient;
use crate::models::{CacheControl, ContentBlock, Message, MessageRequest, SystemBlock, SystemPrompt};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub enabled: bool,
    pub token_threshold: usize,
    pub message_threshold: usize,
    pub model: String,
    pub cache_summary: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_threshold: 50000,
            message_threshold: 50,
            model: "MiniMax-M2.1".to_string(),
            cache_summary: true,
        }
    }
}

pub fn estimate_tokens(messages: &[Message]) -> usize {
    // Rough estimate: ~4 chars per token
    messages
        .iter()
        .map(|m| {
            m.content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text, .. } => text.len() / 4,
                    ContentBlock::Thinking { thinking } => thinking.len() / 4,
                    ContentBlock::ToolUse { input, .. } => {
                        serde_json::to_string(input).map(|s| s.len() / 4).unwrap_or(100)
                    }
                    ContentBlock::ToolResult { content, .. } => content.len() / 4,
                })
                .sum::<usize>()
        })
        .sum()
}

pub fn should_compact(messages: &[Message], config: &CompactionConfig) -> bool {
    if !config.enabled {
        return false;
    }

    let token_estimate = estimate_tokens(messages);
    let message_count = messages.len();

    token_estimate > config.token_threshold || message_count > config.message_threshold
}

pub async fn compact_messages(
    client: &AnthropicClient,
    messages: &[Message],
    config: &CompactionConfig,
) -> Result<(Vec<Message>, Option<SystemPrompt>)> {
    if messages.is_empty() {
        return Ok((Vec::new(), None));
    }

    // Keep the last few messages as-is
    let keep_recent = 4;
    let (to_summarize, recent) = if messages.len() <= keep_recent {
        return Ok((messages.to_vec(), None));
    } else {
        let split_point = messages.len() - keep_recent;
        (&messages[..split_point], &messages[split_point..])
    };

    // Create a summary of older messages
    let summary = create_summary(client, to_summarize, &config.model).await?;

    // Build new message list with summary as system block
    let summary_block = SystemBlock {
        block_type: "text".to_string(),
        text: format!(
            "## Conversation Summary\n\nThe following is a summary of the earlier conversation:\n\n{}\n\n---\nRecent messages follow:",
            summary
        ),
        cache_control: if config.cache_summary {
            Some(CacheControl {
                cache_type: "ephemeral".to_string(),
            })
        } else {
            None
        },
    };

    Ok((
        recent.to_vec(),
        Some(SystemPrompt::Blocks(vec![summary_block])),
    ))
}

async fn create_summary(
    client: &AnthropicClient,
    messages: &[Message],
    model: &str,
) -> Result<String> {
    // Format messages for summarization
    let mut conversation_text = String::new();
    for msg in messages {
        let role = if msg.role == "user" { "User" } else { "Assistant" };
        for block in &msg.content {
            match block {
                ContentBlock::Text { text, .. } => {
                    conversation_text.push_str(&format!("{}: {}\n\n", role, text));
                }
                ContentBlock::ToolUse { name, .. } => {
                    conversation_text.push_str(&format!("{}: [Used tool: {}]\n\n", role, name));
                }
                ContentBlock::ToolResult { content, .. } => {
                    conversation_text.push_str(&format!("Tool result: {}\n\n", &content[..500.min(content.len())]));
                }
                ContentBlock::Thinking { .. } => {
                    // Skip thinking blocks in summary
                }
            }
        }
    }

    let request = MessageRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: format!(
                    "Summarize the following conversation in a concise but comprehensive way. \
                     Preserve key information, decisions made, and any important context. \
                     Keep it under 500 words.\n\n---\n\n{}",
                    conversation_text
                ),
                cache_control: None,
            }],
        }],
        max_tokens: 1024,
        system: Some(SystemPrompt::Text(
            "You are a helpful assistant that creates concise conversation summaries.".to_string(),
        )),
        tools: None,
        tool_choice: None,
        metadata: None,
        thinking: None,
        stream: Some(false),
        temperature: Some(0.3),
        top_p: None,
    };

    let response = client.create_message(request).await?;

    // Extract text from response
    let summary = response
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(summary)
}

pub fn merge_system_prompts(
    original: Option<&SystemPrompt>,
    summary: Option<SystemPrompt>,
) -> Option<SystemPrompt> {
    match (original, summary) {
        (None, None) => None,
        (Some(orig), None) => Some(orig.clone()),
        (None, Some(sum)) => Some(sum),
        (Some(SystemPrompt::Text(orig_text)), Some(SystemPrompt::Blocks(mut sum_blocks))) => {
            // Prepend original system prompt
            sum_blocks.insert(
                0,
                SystemBlock {
                    block_type: "text".to_string(),
                    text: orig_text.clone(),
                    cache_control: None,
                },
            );
            Some(SystemPrompt::Blocks(sum_blocks))
        }
        (Some(SystemPrompt::Blocks(orig_blocks)), Some(SystemPrompt::Blocks(mut sum_blocks))) => {
            // Prepend original blocks
            for (i, block) in orig_blocks.iter().enumerate() {
                sum_blocks.insert(i, block.clone());
            }
            Some(SystemPrompt::Blocks(sum_blocks))
        }
        (Some(orig), Some(SystemPrompt::Text(sum_text))) => {
            let mut blocks = match orig {
                SystemPrompt::Text(t) => vec![SystemBlock {
                    block_type: "text".to_string(),
                    text: t.clone(),
                    cache_control: None,
                }],
                SystemPrompt::Blocks(b) => b.clone(),
            };
            blocks.push(SystemBlock {
                block_type: "text".to_string(),
                text: sum_text,
                cache_control: None,
            });
            Some(SystemPrompt::Blocks(blocks))
        }
    }
}
