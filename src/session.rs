use crate::models::{Message, SystemPrompt};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentSession {
    pub model: String,
    pub workspace: String,
    pub system_prompt: Option<SystemPrompt>,
    pub messages: Vec<Message>,
    pub created_at: u64,
}

impl AgentSession {
    pub fn new(model: String, workspace: String, system_prompt: Option<SystemPrompt>) -> Self {
        Self {
            model,
            workspace,
            system_prompt,
            messages: Vec::new(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

pub fn save(path: &Path, session: &AgentSession) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(session)?;
    fs::write(path, contents)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub fn load(path: &Path) -> Result<AgentSession> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let session: AgentSession = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(session)
}

pub fn export_markdown(path: &Path, session: &AgentSession) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lines = Vec::new();
    lines.push(format!("# MiniMax CLI Session"));
    lines.push(format!("Model: {}", session.model));
    lines.push(format!("Workspace: {}", session.workspace));
    lines.push(String::new());

    for message in &session.messages {
        lines.push(format!("## {}", message.role));
        for block in &message.content {
            match block {
                crate::models::ContentBlock::Text { text, .. } => {
                    lines.push(text.clone());
                }
                crate::models::ContentBlock::Thinking { thinking } => {
                    lines.push(format!("> {}", thinking.replace('\n', "\n> ")));
                }
                crate::models::ContentBlock::ToolUse { name, input, .. } => {
                    lines.push(format!(
                        "Tool Call: {} {}",
                        name,
                        serde_json::to_string_pretty(input).unwrap_or_default()
                    ));
                }
                crate::models::ContentBlock::ToolResult { content, .. } => {
                    lines.push(format!("Tool Result: {}", content));
                }
            }
        }
        lines.push(String::new());
    }

    fs::write(path, lines.join("\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
