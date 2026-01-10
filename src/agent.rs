use crate::client::AnthropicClient;
use crate::models::{
    CacheControl, ContentBlock, Message, MessageRequest, SystemBlock, SystemPrompt, Tool,
};
use crate::session as session;
use crate::mcp;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AgentOptions {
    pub model: String,
    pub prompt: Option<String>,
    pub system: Option<String>,
    pub max_steps: u32,
    pub allow_shell: bool,
    pub workspace: PathBuf,
    pub skills: Vec<String>,
    pub skills_dir: PathBuf,
    pub notes_path: PathBuf,
    pub memory_path: PathBuf,
    pub use_memory: bool,
    pub cache_system: bool,
    pub cache_tools: bool,
    pub cache_memory: bool,
    pub mcp_config_path: PathBuf,
}

pub async fn run(client: &AnthropicClient, options: AgentOptions) -> Result<()> {
    let mut system_prompt = build_system_prompt(&options)?;
    let workspace_display = options.workspace.display().to_string();
    let mut current_session = session::AgentSession::new(
        options.model.clone(),
        workspace_display,
        system_prompt.clone(),
    );

    println!("{}", "MiniMax Agent".bold().cyan());
    println!("Model: {}", options.model);
    println!("Workspace: {}", options.workspace.display());
    println!("Type /exit to quit, /clear to reset.\n");

    if let Some(prompt) = options.prompt.as_deref() {
        run_turn(
            client,
            &options,
            &system_prompt,
            &mut current_session.messages,
            prompt,
        )
        .await?;
        return Ok(());
    }

    let mut editor = Editor::<(), DefaultHistory>::new()?;
    loop {
        match editor.readline("You> ") {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                editor.add_history_entry(input)?;
                if matches_exit(input) {
                    break;
                }
                if input == "/clear" {
                    current_session.messages.clear();
                    println!("{}", "Session cleared.".yellow());
                    continue;
                }
                if handle_session_command(
                    input,
                    &mut current_session,
                    &mut system_prompt,
                )? {
                    continue;
                }
                run_turn(
                    client,
                    &options,
                    &system_prompt,
                    &mut current_session.messages,
                    input,
                )
                .await?;
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

async fn run_turn(
    client: &AnthropicClient,
    options: &AgentOptions,
    system_prompt: &Option<SystemPrompt>,
    messages: &mut Vec<Message>,
    user_input: &str,
) -> Result<()> {
    let tools = build_tools(&options)?;
    let mut tool_registry = ToolRegistry::new(options);

    messages.push(Message {
        role: "user".to_string(),
        content: vec![ContentBlock::Text {
            text: user_input.to_string(),
            cache_control: None,
        }],
    });

    for _step in 0..options.max_steps {
        let request = MessageRequest {
            model: options.model.clone(),
            messages: messages.clone(),
            max_tokens: 4096,
            system: system_prompt.clone(),
            tools: Some(tools.clone()),
            tool_choice: Some(json!({ "type": "auto" })),
            metadata: None,
            thinking: None,
            stream: Some(false),
            temperature: None,
            top_p: None,
        };

        let response = client.create_message(request).await?;

        let mut tool_uses = Vec::new();
        for block in &response.content {
            match block {
                ContentBlock::Thinking { thinking } => {
                    println!("{}", "\nThinking 💭".yellow().dimmed());
                    println!("{}", thinking.yellow().dimmed());
                }
                ContentBlock::Text { text, .. } => {
                    if !text.trim().is_empty() {
                        println!("{}", text);
                    }
                }
                ContentBlock::ToolUse { id, name, input } => {
                    println!(
                        "{} {} {}",
                        "Tool Call:".blue().bold(),
                        name.blue().bold(),
                        format!("id={}", id).dimmed()
                    );
                    tool_uses.push((id.clone(), name.clone(), input.clone()));
                }
                _ => {}
            }
        }

        messages.push(Message {
            role: "assistant".to_string(),
            content: response.content.clone(),
        });

        if tool_uses.is_empty() {
            return Ok(());
        }

        for (tool_id, tool_name, tool_input) in tool_uses {
            let result = tool_registry.execute(&tool_name, tool_input)?;
            messages.push(Message {
                role: "user".to_string(),
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: tool_id,
                    content: result,
                }],
            });
        }
    }

    println!("{}", "Reached max steps.".yellow());
    Ok(())
}

fn matches_exit(input: &str) -> bool {
    let normalized = input.trim().to_lowercase();
    matches!(normalized.as_str(), "/exit" | "exit" | "quit" | "q")
}

fn handle_session_command(
    input: &str,
    current_session: &mut session::AgentSession,
    system_prompt: &mut Option<SystemPrompt>,
) -> Result<bool> {
    if let Some(rest) = input.strip_prefix("/save ") {
        let path = PathBuf::from(rest.trim());
        current_session.system_prompt = system_prompt.clone();
        session::save(&path, current_session)?;
        println!("Saved session to {}", path.display());
        return Ok(true);
    }
    if let Some(rest) = input.strip_prefix("/load ") {
        let path = PathBuf::from(rest.trim());
        let loaded = session::load(&path)?;
        *current_session = loaded;
        *system_prompt = current_session.system_prompt.clone();
        println!("Loaded session from {}", path.display());
        return Ok(true);
    }
    if let Some(rest) = input.strip_prefix("/export ") {
        let path = PathBuf::from(rest.trim());
        session::export_markdown(&path, current_session)?;
        println!("Exported session to {}", path.display());
        return Ok(true);
    }
    Ok(false)
}

fn build_system_prompt(options: &AgentOptions) -> Result<Option<SystemPrompt>> {
    let mut blocks = Vec::new();
    let mut base = options.system.clone().unwrap_or_default();

    if !options.skills.is_empty() {
        let skills_text = load_skills(&options.skills_dir, &options.skills)?;
        if !base.is_empty() {
            base.push_str("\n\n");
        }
        base.push_str("Skills:\n");
        base.push_str(&skills_text);
    }

    if !base.trim().is_empty() {
        blocks.push(SystemBlock {
            block_type: "text".to_string(),
            text: base,
            cache_control: if options.cache_system {
                Some(CacheControl {
                    cache_type: "ephemeral".to_string(),
                })
            } else {
                None
            },
        });
    }

    if options.use_memory {
        let memory = load_memory(&options.memory_path)?;
        if !memory.trim().is_empty() {
            blocks.push(SystemBlock {
                block_type: "text".to_string(),
                text: format!("Long-term memory:\n{}", memory),
                cache_control: if options.cache_memory {
                    Some(CacheControl {
                        cache_type: "ephemeral".to_string(),
                    })
                } else {
                    None
                },
            });
        }
    }

    if blocks.is_empty() {
        Ok(None)
    } else {
        Ok(Some(SystemPrompt::Blocks(blocks)))
    }
}

fn load_skills(skills_dir: &Path, skills: &[String]) -> Result<String> {
    let mut compiled = Vec::new();
    for skill in skills {
        let path = skills_dir.join(skill).join("SKILL.md");
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read skill: {}", path.display()))?;
        compiled.push(format!("# {}\n{}", skill, contents));
    }
    Ok(compiled.join("\n\n"))
}

pub fn build_agent_tools(allow_shell: bool) -> Vec<Tool> {
    let mut tools = vec![
        Tool {
            name: "list_dir".to_string(),
            description: "List entries in a directory relative to the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path (default: .)" }
                },
                "required": []
            }),
            cache_control: None,
        },
        Tool {
            name: "read_file".to_string(),
            description: "Read a UTF-8 file from the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            cache_control: None,
        },
        Tool {
            name: "write_file".to_string(),
            description: "Write a UTF-8 file to the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
            cache_control: None,
        },
        Tool {
            name: "edit_file".to_string(),
            description: "Replace text in a file (simple search/replace).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "search": { "type": "string" },
                    "replace": { "type": "string" }
                },
                "required": ["path", "search", "replace"]
            }),
            cache_control: None,
        },
    ];

    if allow_shell {
        tools.push(Tool {
            name: "exec_shell".to_string(),
            description: "Run a shell command inside the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"]
            }),
            cache_control: None,
        });
    }

    tools.push(Tool {
        name: "note".to_string(),
        description: "Append a note to the agent notes file.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "content": { "type": "string" }
            },
            "required": ["content"]
        }),
        cache_control: None,
    });

    tools
}

fn build_tools(options: &AgentOptions) -> Result<Vec<Tool>> {
    let mut tools = vec![
        Tool {
            name: "list_dir".to_string(),
            description: "List entries in a directory relative to the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path (default: .)" }
                },
                "required": []
            }),
            cache_control: None,
        },
        Tool {
            name: "read_file".to_string(),
            description: "Read a UTF-8 file from the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }),
            cache_control: None,
        },
        Tool {
            name: "write_file".to_string(),
            description: "Write a UTF-8 file to the workspace.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
            cache_control: None,
        },
        Tool {
            name: "edit_file".to_string(),
            description: "Replace text in a file (simple search/replace).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "search": { "type": "string" },
                    "replace": { "type": "string" }
                },
                "required": ["path", "search", "replace"]
            }),
            cache_control: None,
        },
        Tool {
            name: "exec_shell".to_string(),
            description: "Run a shell command inside the workspace. Disabled unless allow_shell=true.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"]
            }),
            cache_control: None,
        },
        Tool {
            name: "note".to_string(),
            description: "Append a note to the agent notes file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string" }
                },
                "required": ["content"]
            }),
            cache_control: None,
        },
        Tool {
            name: "mcp_call".to_string(),
            description: "Call an MCP server tool (stdio). Requires server name and tool name.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" },
                    "tool": { "type": "string" },
                    "arguments": { "type": "object" }
                },
                "required": ["server", "tool"]
            }),
            cache_control: None,
        },
    ];

    if options.cache_tools {
        if let Some(last) = tools.last_mut() {
            last.cache_control = Some(CacheControl {
                cache_type: "ephemeral".to_string(),
            });
        }
    }

    Ok(tools)
}

struct ToolRegistry {
    workspace: PathBuf,
    allow_shell: bool,
    notes_path: PathBuf,
    mcp_config_path: PathBuf,
}

impl ToolRegistry {
    fn new(options: &AgentOptions) -> Self {
        Self {
            workspace: options.workspace.clone(),
            allow_shell: options.allow_shell,
            notes_path: options.notes_path.clone(),
            mcp_config_path: options.mcp_config_path.clone(),
        }
    }

    fn execute(&mut self, name: &str, input: Value) -> Result<String> {
        match name {
            "list_dir" => {
                let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                let dir = self.resolve_path(path)?;
                let mut entries = Vec::new();
                for entry in std::fs::read_dir(&dir)
                    .with_context(|| format!("Failed to read dir: {}", dir.display()))?
                {
                    let entry = entry?;
                    let file_type = entry.file_type()?;
                    entries.push(json!({
                        "name": entry.file_name().to_string_lossy().to_string(),
                        "is_dir": file_type.is_dir(),
                    }));
                }
                Ok(serde_json::to_string_pretty(&entries)?)
            }
            "read_file" => {
                let path = required_str(&input, "path")?;
                let file_path = self.resolve_path(path)?;
                let contents = std::fs::read_to_string(&file_path)
                    .with_context(|| format!("Failed to read {}", file_path.display()))?;
                Ok(contents)
            }
            "write_file" => {
                let path = required_str(&input, "path")?;
                let content = required_str(&input, "content")?;
                let file_path = self.resolve_path(path)?;
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&file_path, content)?;
                Ok(format!("Wrote {}", file_path.display()))
            }
            "edit_file" => {
                let path = required_str(&input, "path")?;
                let search = required_str(&input, "search")?;
                let replace = required_str(&input, "replace")?;
                let file_path = self.resolve_path(path)?;
                let contents = std::fs::read_to_string(&file_path)
                    .with_context(|| format!("Failed to read {}", file_path.display()))?;
                let updated = contents.replace(search, replace);
                let count = contents.matches(search).count();
                std::fs::write(&file_path, updated)?;
                Ok(format!("Replaced {} occurrence(s) in {}", count, file_path.display()))
            }
            "exec_shell" => {
                if !self.allow_shell {
                    return Err(anyhow!("Shell execution disabled. Use --allow-shell to enable."));
                }
                let command = required_str(&input, "command")?;
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&self.workspace)
                    .output()
                    .with_context(|| format!("Failed to execute: {}", command))?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(format!(
                    "exit_code={}\nstdout:\n{}\nstderr:\n{}",
                    output.status.code().unwrap_or(-1),
                    stdout,
                    stderr
                ))
            }
            "note" => {
                let content = required_str(&input, "content")?;
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let line = format!("[{}] {}\n", timestamp, content);
                if let Some(parent) = self.notes_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.notes_path)?
                    .write_all(line.as_bytes())?;
                Ok(format!("Noted in {}", self.notes_path.display()))
            }
            "mcp_call" => {
                let server = required_str(&input, "server")?;
                let tool = required_str(&input, "tool")?;
                let args = input
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let response = mcp::call_tool(self.mcp_config_path.clone(), server, tool, args)?;
                Ok(response)
            }
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    fn resolve_path(&self, raw: &str) -> Result<PathBuf> {
        let candidate = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.workspace.join(raw)
        };
        let canonical = candidate
            .canonicalize()
            .unwrap_or(candidate.clone());
        let workspace = self.workspace.canonicalize().unwrap_or(self.workspace.clone());
        if !canonical.starts_with(&workspace) {
            return Err(anyhow!(
                "Path escapes workspace. Use paths under {}",
                workspace.display()
            ));
        }
        Ok(canonical)
    }
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing or invalid field: {}", key))
}

fn load_memory(path: &Path) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(std::fs::read_to_string(path)?)
}
