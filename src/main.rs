use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use std::path::PathBuf;

mod client;
mod config;
mod logging;
mod models;
mod models_catalog;
mod modules;
mod agent;
mod mcp;
mod skills;
mod memory;
mod session;
mod tui;
mod ui;
mod utils;
mod rlm;
mod compaction;

use crate::config::Config;

#[derive(Parser, Debug)]
#[command(
    name = "minimax",
    author,
    version,
    about = "MiniMax CLI - Chat with MiniMax M2.1",
    long_about = "Unofficial CLI for MiniMax M2.1 API.\n\nJust run 'minimax' to start chatting.\n\nNot affiliated with MiniMax Inc."
)]
struct Cli {
    /// Send a one-shot prompt (non-interactive)
    #[arg(short, long)]
    prompt: Option<String>,

    /// YOLO mode: enable agent tools + shell execution
    #[arg(long)]
    yolo: bool,

    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Config profile name
    #[arg(long)]
    profile: Option<String>,

    /// Workspace directory for file operations
    #[arg(short, long)]
    workspace: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli = Cli::parse();
    logging::set_verbose(cli.verbose);

    let profile = cli.profile.or_else(|| std::env::var("MINIMAX_PROFILE").ok());
    let config = Config::load(cli.config, profile)?;

    let workspace = cli.workspace.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    let model = config
        .default_text_model
        .clone()
        .unwrap_or_else(|| "MiniMax-M2.1".to_string());

    // One-shot prompt mode
    if let Some(prompt) = cli.prompt {
        return run_one_shot(&config, &model, &prompt).await;
    }

    // Default: Interactive TUI
    // --yolo enables agent mode with shell execution
    tui::run_tui(
        &config,
        tui::TuiOptions {
            model,
            workspace,
            allow_shell: cli.yolo || config.allow_shell(),
            skills_dir: config.skills_dir(),
            memory_path: config.memory_path(),
            notes_path: config.notes_path(),
            mcp_config_path: config.mcp_config_path(),
            use_memory: false,
            start_in_agent_mode: cli.yolo,
        },
    )
    .await
}

async fn run_one_shot(config: &Config, model: &str, prompt: &str) -> Result<()> {
    use crate::client::AnthropicClient;
    use crate::models::{ContentBlock, Message, MessageRequest};

    let client = AnthropicClient::new(config)?;

    let request = MessageRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: prompt.to_string(),
                cache_control: None,
            }],
        }],
        max_tokens: 4096,
        system: None,
        tools: None,
        tool_choice: None,
        metadata: None,
        thinking: None,
        stream: Some(false),
        temperature: None,
        top_p: None,
    };

    let response = client.create_message(request).await?;

    for block in response.content {
        if let ContentBlock::Text { text, .. } = block {
            println!("{}", text);
        }
    }

    Ok(())
}
