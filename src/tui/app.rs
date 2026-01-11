use crate::config::{has_api_key, save_api_key, Config};
use crate::models::{Message, SystemPrompt};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingState {
    Welcome,
    EnteringKey,
    Success,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Edit,
    Agent,
    Plan,
    Rlm,
}

impl AppMode {
    pub fn label(&self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Edit => "EDIT",
            AppMode::Agent => "AGENT",
            AppMode::Plan => "PLAN",
            AppMode::Rlm => "RLM",
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            AppMode::Normal => "Chat mode - ask questions, get answers",
            AppMode::Edit => "Edit mode - modify files with AI assistance",
            AppMode::Agent => "Agent mode - autonomous task execution with tools",
            AppMode::Plan => "Plan mode - design before implementing",
            AppMode::Rlm => "RLM mode - recursive language model sandbox",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub is_thinking: bool,
    pub is_tool_call: bool,
    pub tool_name: Option<String>,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
            is_thinking: false,
            is_tool_call: false,
            tool_name: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            is_thinking: false,
            is_tool_call: false,
            tool_name: None,
        }
    }

    pub fn thinking(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            is_thinking: true,
            is_tool_call: false,
            tool_name: None,
        }
    }

    pub fn tool_call(name: &str, content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            is_thinking: false,
            is_tool_call: true,
            tool_name: Some(name.to_string()),
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
            is_thinking: false,
            is_tool_call: false,
            tool_name: None,
        }
    }
}

pub struct TuiOptions {
    pub model: String,
    pub workspace: PathBuf,
    pub allow_shell: bool,
    #[allow(dead_code)]
    pub skills_dir: PathBuf,
    #[allow(dead_code)]
    pub memory_path: PathBuf,
    #[allow(dead_code)]
    pub notes_path: PathBuf,
    #[allow(dead_code)]
    pub mcp_config_path: PathBuf,
    #[allow(dead_code)]
    pub use_memory: bool,
    /// Start in agent mode (--yolo flag)
    pub start_in_agent_mode: bool,
}

pub struct App {
    pub mode: AppMode,
    pub input: String,
    pub cursor_position: usize,
    pub messages: Vec<ChatMessage>,
    pub api_messages: Vec<Message>,
    pub scroll_offset: usize,
    pub is_loading: bool,
    pub status_message: Option<String>,
    pub model: String,
    pub workspace: PathBuf,
    pub system_prompt: Option<SystemPrompt>,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    pub show_help: bool,
    pub auto_compact: bool,
    #[allow(dead_code)]
    pub compact_threshold: usize,
    pub total_tokens: u32,
    pub allow_shell: bool,
    // Onboarding
    pub onboarding: OnboardingState,
    pub api_key_input: String,
    pub api_key_cursor: usize,
}

impl App {
    pub fn new(options: TuiOptions, config: &Config) -> Self {
        // Check if API key exists
        let needs_onboarding = !has_api_key(config);

        // Start in agent mode if --yolo flag was passed
        let initial_mode = if options.start_in_agent_mode {
            AppMode::Agent
        } else {
            AppMode::Normal
        };

        let messages = if needs_onboarding {
            Vec::new() // No welcome message during onboarding
        } else {
            let mode_msg = if options.start_in_agent_mode {
                " | YOLO MODE (agent + shell enabled)"
            } else {
                ""
            };
            vec![ChatMessage::system(&format!(
                "Welcome to MiniMax! Model: {} | Workspace: {}{}",
                options.model,
                options.workspace.display(),
                mode_msg
            ))]
        };

        Self {
            mode: initial_mode,
            input: String::new(),
            cursor_position: 0,
            messages,
            api_messages: Vec::new(),
            scroll_offset: 0,
            is_loading: false,
            status_message: None,
            model: options.model,
            workspace: options.workspace,
            system_prompt: None,
            input_history: Vec::new(),
            history_index: None,
            show_help: false,
            auto_compact: false,
            compact_threshold: 50000,
            total_tokens: 0,
            allow_shell: options.allow_shell,
            onboarding: if needs_onboarding { OnboardingState::Welcome } else { OnboardingState::None },
            api_key_input: String::new(),
            api_key_cursor: 0,
        }
    }

    pub fn submit_api_key(&mut self) -> Result<PathBuf, String> {
        let key = self.api_key_input.trim().to_string();
        if key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }

        match save_api_key(&key) {
            Ok(path) => {
                self.onboarding = OnboardingState::Success;
                self.api_key_input.clear();
                self.api_key_cursor = 0;
                // Add welcome message after successful setup
                self.messages.push(ChatMessage::system(&format!(
                    "Welcome to MiniMax CLI! Model: {} | Workspace: {}",
                    self.model,
                    self.workspace.display()
                )));
                Ok(path)
            }
            Err(e) => Err(format!("Failed to save API key: {}", e))
        }
    }

    pub fn finish_onboarding(&mut self) {
        self.onboarding = OnboardingState::None;
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
        self.status_message = Some(format!("Switched to {} mode", mode.label()));
    }

    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        self.scroll_to_bottom();
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input.remove(self.cursor_position);
        }
    }

    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.input.len() {
            self.input.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.input.len();
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_position = 0;
    }

    pub fn submit_input(&mut self) -> Option<String> {
        if self.input.trim().is_empty() {
            return None;
        }
        let input = self.input.clone();
        if !input.starts_with('/') {
            self.input_history.push(input.clone());
        }
        self.history_index = None;
        self.clear_input();
        Some(input)
    }

    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            None => self.input_history.len().saturating_sub(1),
            Some(i) => i.saturating_sub(1),
        };
        self.history_index = Some(new_index);
        self.input = self.input_history[new_index].clone();
        self.cursor_position = self.input.len();
    }

    pub fn history_down(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        match self.history_index {
            None => {}
            Some(i) => {
                if i + 1 < self.input_history.len() {
                    self.history_index = Some(i + 1);
                    self.input = self.input_history[i + 1].clone();
                    self.cursor_position = self.input.len();
                } else {
                    self.history_index = None;
                    self.clear_input();
                }
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn handle_command(&mut self, cmd: &str) -> Option<AppAction> {
        let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim());

        match command.as_str() {
            "/exit" | "/quit" | "/q" => Some(AppAction::Quit),
            "/clear" => {
                self.messages.clear();
                self.api_messages.clear();
                self.status_message = Some("Conversation cleared".to_string());
                None
            }
            "/help" | "/?" => {
                self.show_help = true;
                None
            }
            "/mode" => {
                if let Some(mode_str) = arg {
                    match mode_str.to_lowercase().as_str() {
                        "normal" | "n" => self.set_mode(AppMode::Normal),
                        "edit" | "e" => self.set_mode(AppMode::Edit),
                        "agent" | "a" => self.set_mode(AppMode::Agent),
                        "plan" | "p" => self.set_mode(AppMode::Plan),
                        "rlm" | "r" => self.set_mode(AppMode::Rlm),
                        _ => {
                            self.status_message =
                                Some("Unknown mode. Use: normal, edit, agent, plan, rlm".to_string());
                        }
                    }
                } else {
                    self.status_message = Some(format!("Current mode: {}", self.mode.label()));
                }
                None
            }
            "/compact" => {
                self.auto_compact = !self.auto_compact;
                self.status_message = Some(format!(
                    "Auto-compact: {}",
                    if self.auto_compact { "ON" } else { "OFF" }
                ));
                None
            }
            "/model" => {
                if let Some(model) = arg {
                    self.model = model.to_string();
                    self.status_message = Some(format!("Model set to: {}", model));
                } else {
                    self.status_message = Some(format!("Current model: {}", self.model));
                }
                None
            }
            "/save" => {
                if let Some(path) = arg {
                    Some(AppAction::SaveSession(PathBuf::from(path)))
                } else {
                    self.status_message = Some("Usage: /save <path>".to_string());
                    None
                }
            }
            "/load" => {
                if let Some(path) = arg {
                    Some(AppAction::LoadSession(PathBuf::from(path)))
                } else {
                    self.status_message = Some("Usage: /load <path>".to_string());
                    None
                }
            }
            "/yolo" => {
                self.allow_shell = true;
                self.set_mode(AppMode::Agent);
                self.status_message = Some("YOLO mode enabled - shell execution allowed!".to_string());
                None
            }
            _ => {
                self.status_message = Some(format!("Unknown command: {}. Type /help for commands.", command));
                None
            }
        }
    }
}

pub enum AppAction {
    Quit,
    SaveSession(PathBuf),
    LoadSession(PathBuf),
    #[allow(dead_code)]
    SendMessage(String),
}
