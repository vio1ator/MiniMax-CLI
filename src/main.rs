use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;
use serde_json;
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

use crate::client::{AnthropicClient, MiniMaxClient};
use crate::config::Config;
use crate::modules::{
    audio, files, image, music, text, video,
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Official Unofficial MiniMax Platform CLI")]
struct Cli {
    /// Path to config file (default: ~/.minimax/config.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// Output directory for generated media
    #[arg(long, global = true)]
    output_dir: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(long, global = true)]
    verbose: bool,

    /// Config profile name
    #[arg(long, global = true)]
    profile: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Text generation (chat)
    #[command(subcommand)]
    Text(TextCommand),
    /// Agentic mode (tool loop)
    #[command(subcommand)]
    Agent(AgentCommand),
    /// Interactive TUI (ratatui interface)
    Tui(TuiArgs),
    /// RLM sandbox mode (local REPL for large contexts)
    #[command(subcommand)]
    Rlm(RlmCommand),
    /// Image generation
    #[command(subcommand)]
    Image(ImageCommand),
    /// Video generation
    #[command(subcommand)]
    Video(VideoCommand),
    /// Speech and voice features
    #[command(subcommand)]
    Audio(AudioCommand),
    /// Music generation
    #[command(subcommand)]
    Music(MusicCommand),
    /// File management
    #[command(subcommand)]
    Files(FilesCommand),
    /// Model registry
    #[command(subcommand)]
    Models(ModelsCommand),
    /// Memory management
    #[command(subcommand)]
    Memory(MemoryCommand),
    /// MCP server management
    #[command(subcommand)]
    Mcp(McpCommand),
    /// Skills management
    #[command(subcommand)]
    Skills(SkillsCommand),
}

#[derive(Subcommand, Debug)]
enum TextCommand {
    /// Chat with M2.1 models (Anthropic or official API)
    Chat(TextChatArgs),
}

#[derive(Subcommand, Debug)]
enum AgentCommand {
    /// Run an agentic session with tools
    Run(AgentRunArgs),
}

#[derive(Parser, Debug)]
struct TextChatArgs {
    /// API mode to use for chat
    #[arg(long, value_enum, default_value_t = TextApi::Anthropic)]
    api: TextApi,

    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Initial prompt
    #[arg(short, long)]
    prompt: Option<String>,

    /// System prompt
    #[arg(long)]
    system: Option<String>,

    /// Disable streaming
    #[arg(long)]
    no_stream: bool,

    /// Temperature
    #[arg(long)]
    temperature: Option<f32>,

    /// Top-p
    #[arg(long)]
    top_p: Option<f32>,

    /// Max tokens
    #[arg(long, default_value_t = 4096)]
    max_tokens: u32,

    /// Enable ephemeral prompt caching for the user message
    #[arg(long)]
    cache: bool,

    /// Cache the system prompt (Anthropic API only)
    #[arg(long)]
    cache_system: bool,

    /// Cache tool definitions (Anthropic API only)
    #[arg(long)]
    cache_tools: bool,

    /// Path to tools JSON file
    #[arg(long)]
    tools_file: Option<PathBuf>,

    /// Inline tools JSON (array)
    #[arg(long)]
    tools_json: Option<String>,

    /// Tool choice: JSON, auto, none, or tool name
    #[arg(long)]
    tool_choice: Option<String>,
}

#[derive(Parser, Debug)]
struct AgentRunArgs {
    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Initial prompt
    #[arg(short, long)]
    prompt: Option<String>,

    /// System prompt
    #[arg(long)]
    system: Option<String>,

    /// Max steps before stopping
    #[arg(long, default_value_t = 12)]
    max_steps: u32,

    /// Enable shell tool for this run
    #[arg(long)]
    allow_shell: bool,

    /// Workspace directory for file tools
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Skills to load (repeatable)
    #[arg(long)]
    skill: Vec<String>,

    /// Include long-term memory in the system prompt
    #[arg(long)]
    memory: bool,

    /// Cache system prompt blocks
    #[arg(long)]
    cache_system: bool,

    /// Cache tool definitions
    #[arg(long)]
    cache_tools: bool,

    /// Cache memory block
    #[arg(long)]
    cache_memory: bool,
}

#[derive(Parser, Debug)]
struct TuiArgs {
    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Workspace directory
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Enable shell execution
    #[arg(long)]
    allow_shell: bool,

    /// Include long-term memory
    #[arg(long)]
    memory: bool,
}

#[derive(Subcommand, Debug)]
enum RlmCommand {
    /// Load a file into the RLM context
    Load(RlmLoadArgs),
    /// Search the loaded context with regex
    Search(RlmSearchArgs),
    /// Execute Python-like code on the context
    Exec(RlmExecArgs),
    /// Show RLM session status
    Status(RlmStatusArgs),
    /// Save RLM session
    SaveSession(RlmSaveSessionArgs),
    /// Load RLM session
    LoadSession(RlmLoadSessionArgs),
    /// Interactive RLM REPL
    Repl(RlmReplArgs),
}

#[derive(Parser, Debug)]
struct RlmLoadArgs {
    /// Path to file to load
    #[arg(long)]
    path: PathBuf,

    /// Context ID (default: "default")
    #[arg(long, default_value = "default")]
    context_id: String,
}

#[derive(Parser, Debug)]
struct RlmSearchArgs {
    /// Context ID
    #[arg(long, default_value = "default")]
    context_id: String,

    /// Regex pattern to search
    #[arg(long)]
    pattern: String,

    /// Context lines around matches
    #[arg(long, default_value = "2")]
    context_lines: usize,

    /// Max results
    #[arg(long, default_value = "20")]
    max_results: usize,
}

#[derive(Parser, Debug)]
struct RlmExecArgs {
    /// Context ID
    #[arg(long, default_value = "default")]
    context_id: String,

    /// Code to execute
    #[arg(long)]
    code: String,
}

#[derive(Parser, Debug)]
struct RlmStatusArgs {
    /// Context ID (if not set, shows all contexts)
    #[arg(long)]
    context_id: Option<String>,
}

#[derive(Parser, Debug)]
struct RlmSaveSessionArgs {
    /// Path to save session
    #[arg(long)]
    path: PathBuf,

    /// Context ID
    #[arg(long, default_value = "default")]
    context_id: String,
}

#[derive(Parser, Debug)]
struct RlmLoadSessionArgs {
    /// Path to load session from
    #[arg(long)]
    path: PathBuf,
}

#[derive(Parser, Debug)]
struct RlmReplArgs {
    /// Context ID
    #[arg(long, default_value = "default")]
    context_id: String,

    /// Optional file to load at start
    #[arg(long)]
    load: Option<PathBuf>,
}

#[derive(ValueEnum, Debug, Clone)]
enum TextApi {
    Anthropic,
    Official,
}

#[derive(Subcommand, Debug)]
enum ImageCommand {
    /// Generate images from text
    Generate(ImageGenerateArgs),
}

#[derive(Parser, Debug)]
struct ImageGenerateArgs {
    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Prompt
    #[arg(long)]
    prompt: String,

    /// Negative prompt
    #[arg(long)]
    negative_prompt: Option<String>,

    /// Aspect ratio (e.g. 1:1, 16:9)
    #[arg(long)]
    aspect_ratio: Option<String>,

    /// Width
    #[arg(long)]
    width: Option<u32>,

    /// Height
    #[arg(long)]
    height: Option<u32>,

    /// Style
    #[arg(long)]
    style: Option<String>,

    /// Response format (url or b64_json)
    #[arg(long)]
    response_format: Option<String>,

    /// Random seed
    #[arg(long)]
    seed: Option<u32>,

    /// Number of images
    #[arg(long)]
    n: Option<u32>,

    /// Enable prompt optimizer
    #[arg(long)]
    prompt_optimizer: Option<bool>,

    /// Subject reference entries (I2I)
    #[arg(long)]
    subject_reference: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum VideoCommand {
    /// Generate a video
    Generate(VideoGenerateArgs),
    /// Query video status
    Query(VideoQueryArgs),
    /// Create a video via template agent
    AgentCreate(VideoAgentCreateArgs),
    /// Query video template agent status
    AgentQuery(VideoAgentQueryArgs),
}

#[derive(Parser, Debug)]
struct VideoGenerateArgs {
    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Prompt
    #[arg(long)]
    prompt: String,

    /// First frame (URL or local path)
    #[arg(long)]
    first_frame: Option<String>,

    /// Last frame (URL or local path)
    #[arg(long)]
    last_frame: Option<String>,

    /// Subject reference (repeatable)
    #[arg(long)]
    subject_reference: Vec<String>,

    /// Subject reference JSON (advanced)
    #[arg(long)]
    subject_reference_json: Option<String>,

    /// Duration in seconds
    #[arg(long)]
    duration: Option<u32>,

    /// Resolution (e.g. 720P, 1080P)
    #[arg(long)]
    resolution: Option<String>,

    /// Callback URL
    #[arg(long)]
    callback_url: Option<String>,

    /// Prompt optimizer
    #[arg(long)]
    prompt_optimizer: Option<bool>,

    /// Fast pretreatment
    #[arg(long)]
    fast_pretreatment: Option<bool>,

    /// Wait for completion and download
    #[arg(long, default_value_t = true)]
    wait: bool,
}

#[derive(Parser, Debug)]
struct VideoQueryArgs {
    /// Task ID to query
    #[arg(long)]
    task_id: String,
}

#[derive(Parser, Debug)]
struct VideoAgentCreateArgs {
    /// Template ID
    #[arg(long)]
    template_id: String,

    /// Text inputs JSON
    #[arg(long)]
    text_inputs_json: Option<String>,

    /// Media inputs JSON
    #[arg(long)]
    media_inputs_json: Option<String>,

    /// Callback URL
    #[arg(long)]
    callback_url: Option<String>,
}

#[derive(Parser, Debug)]
struct VideoAgentQueryArgs {
    /// Task ID to query
    #[arg(long)]
    task_id: String,
}

#[derive(Subcommand, Debug)]
enum AudioCommand {
    /// Text-to-audio (sync)
    T2a(AudioT2aArgs),
    /// Text-to-audio (async create)
    T2aAsyncCreate(AudioT2aAsyncCreateArgs),
    /// Text-to-audio (async query)
    T2aAsyncQuery(AudioT2aAsyncQueryArgs),
    /// Voice management
    #[command(subcommand)]
    Voice(VoiceCommand),
}

#[derive(Parser, Debug)]
struct AudioT2aArgs {
    /// Model to use (Speech-01, Speech-02)
    #[arg(long)]
    model: Option<String>,

    /// Text to synthesize
    #[arg(long)]
    text: String,

    /// Stream audio bytes
    #[arg(long)]
    stream: bool,

    /// Output format (wav, mp3)
    #[arg(long)]
    output_format: Option<String>,

    /// Voice ID
    #[arg(long)]
    voice_id: Option<String>,

    /// Voice settings JSON
    #[arg(long)]
    voice_setting_json: Option<String>,

    /// Audio settings JSON
    #[arg(long)]
    audio_setting_json: Option<String>,

    /// Pronunciation dict JSON
    #[arg(long)]
    pronunciation_dict_json: Option<String>,

    /// Timber weights JSON
    #[arg(long)]
    timber_weights_json: Option<String>,

    /// Language boost JSON
    #[arg(long)]
    language_boost_json: Option<String>,

    /// Voice modify JSON
    #[arg(long)]
    voice_modify_json: Option<String>,

    /// Enable subtitles
    #[arg(long)]
    subtitle_enable: Option<bool>,
}

#[derive(Parser, Debug)]
struct AudioT2aAsyncCreateArgs {
    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Text to synthesize
    #[arg(long)]
    text: Option<String>,

    /// Text file ID (uploaded)
    #[arg(long)]
    text_file_id: Option<String>,

    /// Voice ID
    #[arg(long)]
    voice_id: Option<String>,

    /// Voice settings JSON
    #[arg(long)]
    voice_setting_json: Option<String>,

    /// Audio settings JSON
    #[arg(long)]
    audio_setting_json: Option<String>,

    /// Pronunciation dict JSON
    #[arg(long)]
    pronunciation_dict_json: Option<String>,

    /// Language boost JSON
    #[arg(long)]
    language_boost_json: Option<String>,

    /// Voice modify JSON
    #[arg(long)]
    voice_modify_json: Option<String>,
}

#[derive(Parser, Debug)]
struct AudioT2aAsyncQueryArgs {
    /// Task ID to query
    #[arg(long)]
    task_id: String,
}

#[derive(Subcommand, Debug)]
enum VoiceCommand {
    /// Clone a voice from audio
    Clone(VoiceCloneArgs),
    /// List voices
    List(VoiceListArgs),
    /// Delete a voice
    Delete(VoiceDeleteArgs),
    /// Design a voice
    Design(VoiceDesignArgs),
}

#[derive(Parser, Debug)]
struct VoiceCloneArgs {
    /// Clone audio file (required)
    #[arg(long)]
    clone_audio: PathBuf,

    /// Prompt audio file
    #[arg(long)]
    prompt_audio: Option<PathBuf>,

    /// Voice ID to create/overwrite
    #[arg(long)]
    voice_id: Option<String>,

    /// Clone prompt text
    #[arg(long)]
    clone_prompt_text: Option<String>,

    /// Optional TTS text to synthesize immediately
    #[arg(long)]
    text: Option<String>,

    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Language boost JSON
    #[arg(long)]
    language_boost_json: Option<String>,

    /// Apply noise reduction
    #[arg(long)]
    need_noise_reduction: Option<bool>,

    /// Apply volume normalization
    #[arg(long)]
    need_volume_normalization: Option<bool>,
}

#[derive(Parser, Debug)]
struct VoiceListArgs {
    /// Voice type (system/custom)
    #[arg(long)]
    voice_type: Option<String>,
}

#[derive(Parser, Debug)]
struct VoiceDeleteArgs {
    /// Voice type (system/custom)
    #[arg(long)]
    voice_type: Option<String>,

    /// Voice ID
    #[arg(long)]
    voice_id: String,
}

#[derive(Parser, Debug)]
struct VoiceDesignArgs {
    /// Design prompt
    #[arg(long)]
    prompt: String,

    /// Preview text
    #[arg(long)]
    preview_text: String,

    /// Voice ID (optional)
    #[arg(long)]
    voice_id: Option<String>,
}

#[derive(Subcommand, Debug)]
enum MusicCommand {
    /// Generate music
    Generate(MusicGenerateArgs),
}

#[derive(Parser, Debug)]
struct MusicGenerateArgs {
    /// Model to use
    #[arg(long)]
    model: Option<String>,

    /// Prompt
    #[arg(long)]
    prompt: String,

    /// Lyrics
    #[arg(long)]
    lyrics: Option<String>,

    /// Stream audio bytes
    #[arg(long)]
    stream: bool,

    /// Output format (wav, mp3)
    #[arg(long)]
    output_format: Option<String>,

    /// Audio settings JSON
    #[arg(long)]
    audio_setting_json: Option<String>,
}

#[derive(Subcommand, Debug)]
enum FilesCommand {
    /// Upload a file
    Upload(FilesUploadArgs),
    /// List files
    List(FilesListArgs),
    /// Retrieve a file URL
    Retrieve(FilesRetrieveArgs),
    /// Download file contents
    RetrieveContent(FilesRetrieveContentArgs),
    /// Delete a file
    Delete(FilesDeleteArgs),
}

#[derive(Subcommand, Debug)]
enum McpCommand {
    /// List MCP servers
    List,
    /// Add an MCP server
    Add(McpAddArgs),
    /// Remove an MCP server
    Remove(McpRemoveArgs),
}

#[derive(Subcommand, Debug)]
enum ModelsCommand {
    /// List available models by modality
    List(ModelsListArgs),
}

#[derive(Parser, Debug)]
struct ModelsListArgs {
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Parser, Debug)]
struct McpAddArgs {
    /// Server name
    #[arg(long)]
    name: String,

    /// Command to execute
    #[arg(long)]
    command: String,

    /// Arguments (repeatable)
    #[arg(long)]
    arg: Vec<String>,

    /// Environment variables (KEY=VALUE)
    #[arg(long)]
    env: Vec<String>,
}

#[derive(Parser, Debug)]
struct McpRemoveArgs {
    /// Server name
    #[arg(long)]
    name: String,
}

#[derive(Subcommand, Debug)]
enum SkillsCommand {
    /// List available skills
    List,
    /// Show a skill's README
    Show(SkillsShowArgs),
}

#[derive(Subcommand, Debug)]
enum MemoryCommand {
    /// Show long-term memory
    Show,
    /// Append memory entry
    Add(MemoryAddArgs),
    /// Clear memory
    Clear,
}

#[derive(Parser, Debug)]
struct MemoryAddArgs {
    /// Memory content
    #[arg(long)]
    content: String,
}

#[derive(Parser, Debug)]
struct SkillsShowArgs {
    /// Skill name (directory under skills dir)
    #[arg(long)]
    name: String,
}

#[derive(Parser, Debug)]
struct FilesUploadArgs {
    /// Path to file
    #[arg(long)]
    path: PathBuf,

    /// Purpose
    #[arg(long)]
    purpose: String,
}

#[derive(Parser, Debug)]
struct FilesListArgs {
    /// Purpose filter
    #[arg(long)]
    purpose: String,
}

#[derive(Parser, Debug)]
struct FilesDeleteArgs {
    /// File ID
    #[arg(long)]
    file_id: String,

    /// Purpose
    #[arg(long)]
    purpose: Option<String>,
}

#[derive(Parser, Debug)]
struct FilesRetrieveArgs {
    /// File ID
    #[arg(long)]
    file_id: String,

    /// Purpose
    #[arg(long)]
    purpose: Option<String>,
}

#[derive(Parser, Debug)]
struct FilesRetrieveContentArgs {
    /// File ID
    #[arg(long)]
    file_id: String,

    /// Output path
    #[arg(long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli = Cli::parse();
    logging::set_verbose(cli.verbose);
    let profile = cli
        .profile
        .or_else(|| std::env::var("MINIMAX_PROFILE").ok());
    let config = Config::load(cli.config, profile)?;
    let output_dir = cli.output_dir.unwrap_or_else(|| config.output_dir());

    // Default to TUI if no command specified
    let command = cli.command.unwrap_or(Command::Tui(TuiArgs {
        model: None,
        workspace: None,
        allow_shell: false,
        memory: false,
    }));

    match command {
        Command::Text(command) => match command {
            TextCommand::Chat(args) => {
                let model = args
                    .model
                    .or_else(|| config.default_text_model.clone())
                    .unwrap_or_else(|| "MiniMax-M2.1".to_string());

                let tools_path = args
                    .tools_file
                    .or_else(|| config.tools_file.clone().map(PathBuf::from));
                let tools = text::load_tools(tools_path.as_deref(), args.tools_json.as_deref())?;
                let tool_choice = text::parse_tool_choice(args.tool_choice.as_deref())?;

                let options = text::TextChatOptions {
                    model,
                    prompt: args.prompt,
                    system: args.system,
                    stream: !args.no_stream,
                    temperature: args.temperature,
                    top_p: args.top_p,
                    max_tokens: args.max_tokens,
                    cache_prompt: args.cache,
                    cache_system: args.cache_system,
                    cache_tools: args.cache_tools,
                    tools,
                    tool_choice,
                };

                match args.api {
                    TextApi::Anthropic => {
                        let client = AnthropicClient::new(&config)?;
                        text::run_anthropic_chat(&client, options).await?;
                    }
                    TextApi::Official => {
                        let client = MiniMaxClient::new(&config)?;
                        text::run_official_chat(&client, options).await?;
                    }
                }
            }
        },
        Command::Agent(command) => match command {
            AgentCommand::Run(args) => {
                let client = AnthropicClient::new(&config)?;
                let model = args
                    .model
                    .or_else(|| config.default_text_model.clone())
                    .unwrap_or_else(|| "MiniMax-M2.1".to_string());
                let workspace = args.workspace.unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
                agent::run(
                    &client,
                    agent::AgentOptions {
                        model,
                        prompt: args.prompt,
                        system: args.system,
                        max_steps: args.max_steps,
                        allow_shell: args.allow_shell || config.allow_shell(),
                        workspace,
                        skills: args.skill,
                        skills_dir: config.skills_dir(),
                        notes_path: config.notes_path(),
                        memory_path: config.memory_path(),
                        use_memory: args.memory,
                        cache_system: args.cache_system,
                        cache_tools: args.cache_tools,
                        cache_memory: args.cache_memory,
                        mcp_config_path: config.mcp_config_path(),
                    },
                )
                .await?;
            }
        },
        Command::Tui(args) => {
            let model = args
                .model
                .or_else(|| config.default_text_model.clone())
                .unwrap_or_else(|| "MiniMax-M2.1".to_string());
            let workspace = args.workspace.unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });
            tui::run_tui(
                &config,
                tui::TuiOptions {
                    model,
                    workspace,
                    allow_shell: args.allow_shell || config.allow_shell(),
                    skills_dir: config.skills_dir(),
                    memory_path: config.memory_path(),
                    notes_path: config.notes_path(),
                    mcp_config_path: config.mcp_config_path(),
                    use_memory: args.memory,
                },
            )
            .await?;
        }
        Command::Rlm(command) => {
            rlm::handle_command(command, &config)?;
        }
        Command::Image(command) => match command {
            ImageCommand::Generate(args) => {
                let client = MiniMaxClient::new(&config)?;
                let model = args
                    .model
                    .or_else(|| config.default_image_model.clone())
                    .unwrap_or_else(|| "image-01".to_string());
                let options = image::ImageGenerateOptions {
                    model,
                    prompt: args.prompt,
                    negative_prompt: args.negative_prompt,
                    aspect_ratio: args.aspect_ratio,
                    width: args.width,
                    height: args.height,
                    style: args.style,
                    response_format: args.response_format,
                    seed: args.seed,
                    n: args.n,
                    prompt_optimizer: args.prompt_optimizer,
                    subject_reference: args.subject_reference,
                    output_dir,
                };
                image::generate(&client, options).await?;
            }
        },
        Command::Video(command) => match command {
            VideoCommand::Generate(args) => {
                let client = MiniMaxClient::new(&config)?;
                let model = args
                    .model
                    .or_else(|| config.default_video_model.clone())
                    .unwrap_or_else(|| "video-01".to_string());
                let options = video::VideoGenerateOptions {
                    model,
                    prompt: args.prompt,
                    first_frame: args.first_frame,
                    last_frame: args.last_frame,
                    subject_reference: args.subject_reference,
                    subject_reference_json: args.subject_reference_json,
                    duration: args.duration,
                    resolution: args.resolution,
                    callback_url: args.callback_url,
                    prompt_optimizer: args.prompt_optimizer,
                    fast_pretreatment: args.fast_pretreatment,
                    wait: args.wait,
                    output_dir,
                };
                video::generate(&client, options).await?;
            }
            VideoCommand::Query(args) => {
                let client = MiniMaxClient::new(&config)?;
                video::query(
                    &client,
                    video::VideoQueryOptions {
                        task_id: args.task_id,
                    },
                )
                .await?;
            }
            VideoCommand::AgentCreate(args) => {
                let client = MiniMaxClient::new(&config)?;
                video::agent_create(
                    &client,
                    video::VideoAgentCreateOptions {
                        template_id: args.template_id,
                        text_inputs_json: args.text_inputs_json,
                        media_inputs_json: args.media_inputs_json,
                        callback_url: args.callback_url,
                    },
                )
                .await?;
            }
            VideoCommand::AgentQuery(args) => {
                let client = MiniMaxClient::new(&config)?;
                video::agent_query(&client, &args.task_id).await?;
            }
        },
        Command::Audio(command) => match command {
            AudioCommand::T2a(args) => {
                let client = MiniMaxClient::new(&config)?;
                let model = args
                    .model
                    .or_else(|| config.default_audio_model.clone())
                    .unwrap_or_else(|| "speech-01".to_string());
                let options = audio::T2aOptions {
                    model,
                    text: args.text,
                    stream: args.stream,
                    output_format: args.output_format,
                    voice_id: args.voice_id,
                    voice_setting_json: args.voice_setting_json,
                    audio_setting_json: args.audio_setting_json,
                    pronunciation_dict_json: args.pronunciation_dict_json,
                    timber_weights_json: args.timber_weights_json,
                    language_boost_json: args.language_boost_json,
                    voice_modify_json: args.voice_modify_json,
                    subtitle_enable: args.subtitle_enable,
                    output_dir,
                };
                audio::t2a(&client, options).await?;
            }
            AudioCommand::T2aAsyncCreate(args) => {
                let client = MiniMaxClient::new(&config)?;
                let model = args
                    .model
                    .or_else(|| config.default_audio_model.clone())
                    .unwrap_or_else(|| "speech-01".to_string());
                let options = audio::T2aAsyncCreateOptions {
                    model,
                    text: args.text,
                    text_file_id: args.text_file_id,
                    voice_id: args.voice_id,
                    voice_setting_json: args.voice_setting_json,
                    audio_setting_json: args.audio_setting_json,
                    pronunciation_dict_json: args.pronunciation_dict_json,
                    language_boost_json: args.language_boost_json,
                    voice_modify_json: args.voice_modify_json,
                };
                audio::t2a_async_create(&client, options).await?;
            }
            AudioCommand::T2aAsyncQuery(args) => {
                let client = MiniMaxClient::new(&config)?;
                audio::t2a_async_query(
                    &client,
                    audio::T2aAsyncQueryOptions {
                        task_id: args.task_id,
                    },
                )
                .await?;
            }
            AudioCommand::Voice(command) => {
                let client = MiniMaxClient::new(&config)?;
                match command {
                    VoiceCommand::Clone(args) => {
                        let options = audio::VoiceCloneOptions {
                            clone_audio: args.clone_audio,
                            prompt_audio: args.prompt_audio,
                            voice_id: args.voice_id,
                            clone_prompt_text: args.clone_prompt_text,
                            text: args.text,
                            model: args.model,
                            language_boost_json: args.language_boost_json,
                            need_noise_reduction: args.need_noise_reduction,
                            need_volume_normalization: args.need_volume_normalization,
                        };
                        audio::voice_clone(&client, options).await?;
                    }
                    VoiceCommand::List(args) => {
                        audio::voice_list(
                            &client,
                            audio::VoiceListOptions {
                                voice_type: args.voice_type,
                            },
                        )
                        .await?;
                    }
                    VoiceCommand::Delete(args) => {
                        audio::voice_delete(
                            &client,
                            audio::VoiceDeleteOptions {
                                voice_type: args.voice_type,
                                voice_id: args.voice_id,
                            },
                        )
                        .await?;
                    }
                    VoiceCommand::Design(args) => {
                        audio::voice_design(
                            &client,
                            audio::VoiceDesignOptions {
                                prompt: args.prompt,
                                preview_text: args.preview_text,
                                voice_id: args.voice_id,
                            },
                        )
                        .await?;
                    }
                }
            }
        },
        Command::Music(command) => match command {
            MusicCommand::Generate(args) => {
                let client = MiniMaxClient::new(&config)?;
                let model = args
                    .model
                    .or_else(|| config.default_music_model.clone())
                    .unwrap_or_else(|| "music-01".to_string());
                let options = music::MusicGenerateOptions {
                    model,
                    prompt: args.prompt,
                    lyrics: args.lyrics,
                    stream: args.stream,
                    output_format: args.output_format,
                    audio_setting_json: args.audio_setting_json,
                    output_dir,
                };
                music::generate(&client, options).await?;
            }
        },
        Command::Files(command) => match command {
            FilesCommand::Upload(args) => {
                let client = MiniMaxClient::new(&config)?;
                files::upload(
                    &client,
                    files::FileUploadOptions {
                        path: args.path,
                        purpose: args.purpose,
                    },
                )
                .await?;
            }
            FilesCommand::List(args) => {
                let client = MiniMaxClient::new(&config)?;
                files::list(
                    &client,
                    files::FileListOptions {
                        purpose: args.purpose,
                    },
                )
                .await?;
            }
            FilesCommand::Retrieve(args) => {
                let client = MiniMaxClient::new(&config)?;
                files::retrieve(
                    &client,
                    files::FileRetrieveOptions {
                        file_id: args.file_id,
                        purpose: args.purpose,
                    },
                )
                .await?;
            }
            FilesCommand::RetrieveContent(args) => {
                let client = MiniMaxClient::new(&config)?;
                files::retrieve_content(
                    &client,
                    files::FileRetrieveContentOptions {
                        file_id: args.file_id,
                        output: args.output,
                        output_dir: output_dir.clone(),
                    },
                )
                .await?;
            }
            FilesCommand::Delete(args) => {
                let client = MiniMaxClient::new(&config)?;
                files::delete(
                    &client,
                    files::FileDeleteOptions {
                        file_id: args.file_id,
                        purpose: args.purpose,
                    },
                )
                .await?;
            }
        },
        Command::Mcp(command) => match command {
            McpCommand::List => {
                mcp::list(config.mcp_config_path())?;
            }
            McpCommand::Add(args) => {
                mcp::add(
                    config.mcp_config_path(),
                    mcp::McpServerInput {
                        name: args.name,
                        command: args.command,
                        args: args.arg,
                        env: args.env,
                    },
                )?;
            }
            McpCommand::Remove(args) => {
                mcp::remove(config.mcp_config_path(), &args.name)?;
            }
        },
        Command::Skills(command) => match command {
            SkillsCommand::List => {
                skills::list(config.skills_dir())?;
            }
            SkillsCommand::Show(args) => {
                skills::show(config.skills_dir(), &args.name)?;
            }
        },
        Command::Models(command) => match command {
            ModelsCommand::List(args) => {
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&models_catalog::as_json())?);
                } else {
                    for category in models_catalog::categories() {
                        println!("{}:", category.name);
                        for model in category.models {
                            println!("  - {}", model);
                        }
                    }
                }
            }
        },
        Command::Memory(command) => match command {
            MemoryCommand::Show => {
                memory::show(config.memory_path())?;
            }
            MemoryCommand::Add(args) => {
                memory::add(config.memory_path(), &args.content)?;
            }
            MemoryCommand::Clear => {
                memory::clear(config.memory_path())?;
            }
        },
    }

    Ok(())
}
