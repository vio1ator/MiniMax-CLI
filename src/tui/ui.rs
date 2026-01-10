use super::app::{App, AppAction, AppMode, ChatMessage, OnboardingState, TuiOptions};
use crate::client::AnthropicClient;
use crate::config::Config;
use crate::models::{ContentBlock, Message, MessageRequest};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use serde_json::json;
use std::io::{self, Stdout};

const MINIMAX_RED: Color = Color::Rgb(220, 80, 80);
const MINIMAX_CORAL: Color = Color::Rgb(240, 128, 100);
const MINIMAX_ORANGE: Color = Color::Rgb(255, 165, 80);

const LOGO: &str = r#"
 ███╗   ███╗██╗███╗   ██╗██╗███╗   ███╗ █████╗ ██╗  ██╗
 ████╗ ████║██║████╗  ██║██║████╗ ████║██╔══██╗╚██╗██╔╝
 ██╔████╔██║██║██╔██╗ ██║██║██╔████╔██║███████║ ╚███╔╝
 ██║╚██╔╝██║██║██║╚██╗██║██║██║╚██╔╝██║██╔══██║ ██╔██╗
 ██║ ╚═╝ ██║██║██║ ╚████║██║██║ ╚═╝ ██║██║  ██║██╔╝ ██╗
 ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
"#;

const LOGO_SMALL: &str = "MiniMax CLI";

fn mode_color(mode: AppMode) -> Color {
    match mode {
        AppMode::Normal => Color::Green,
        AppMode::Edit => Color::Yellow,
        AppMode::Agent => Color::Cyan,
        AppMode::Plan => Color::Magenta,
        AppMode::Rlm => MINIMAX_CORAL,
    }
}

pub async fn run_tui(config: &Config, options: TuiOptions) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(options, config);
    let result = run_event_loop(&mut terminal, &mut app, config).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    config: &Config,
) -> Result<()> {
    // Track if we need to reload config after onboarding
    let mut config_with_key = config.clone();

    loop {
        terminal.draw(|f| render(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Handle onboarding flow
                if app.onboarding != OnboardingState::None {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Esc => {
                            if app.onboarding == OnboardingState::EnteringKey {
                                app.onboarding = OnboardingState::Welcome;
                                app.api_key_input.clear();
                                app.api_key_cursor = 0;
                            }
                        }
                        KeyCode::Enter => {
                            match app.onboarding {
                                OnboardingState::Welcome => {
                                    app.onboarding = OnboardingState::EnteringKey;
                                }
                                OnboardingState::EnteringKey => {
                                    match app.submit_api_key() {
                                        Ok(path) => {
                                            app.status_message = Some(format!("API key saved to {}", path.display()));
                                            // Reload config with new API key
                                            if let Ok(new_config) = crate::config::Config::load(None, None) {
                                                config_with_key = new_config;
                                            }
                                        }
                                        Err(e) => {
                                            app.status_message = Some(e);
                                        }
                                    }
                                }
                                OnboardingState::Success => {
                                    app.finish_onboarding();
                                }
                                OnboardingState::None => {}
                            }
                        }
                        KeyCode::Backspace if app.onboarding == OnboardingState::EnteringKey => {
                            if app.api_key_cursor > 0 {
                                app.api_key_cursor -= 1;
                                app.api_key_input.remove(app.api_key_cursor);
                            }
                        }
                        KeyCode::Char(c) if app.onboarding == OnboardingState::EnteringKey => {
                            app.api_key_input.insert(app.api_key_cursor, c);
                            app.api_key_cursor += 1;
                        }
                        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) && app.onboarding == OnboardingState::EnteringKey => {
                            // Note: Ctrl+V paste doesn't work in raw terminal mode
                            // User needs to use terminal's paste (Cmd+V / right-click)
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle help popup
                if app.show_help {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter) {
                        app.show_help = false;
                    }
                    continue;
                }

                // Global keybindings
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::F(1) => {
                        app.toggle_help();
                    }
                    KeyCode::Esc => {
                        if !app.input.is_empty() {
                            app.clear_input();
                        } else {
                            app.set_mode(AppMode::Normal);
                        }
                    }
                    KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
                        app.scroll_up(3);
                    }
                    KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
                        app.scroll_down(3);
                    }
                    KeyCode::PageUp => {
                        app.scroll_up(10);
                    }
                    KeyCode::PageDown => {
                        app.scroll_down(10);
                    }
                    // Input handling
                    KeyCode::Enter => {
                        if let Some(input) = app.submit_input() {
                            if input.starts_with('/') {
                                match app.handle_command(&input) {
                                    Some(AppAction::Quit) => return Ok(()),
                                    Some(AppAction::SaveSession(path)) => {
                                        app.status_message = Some(format!("Session saved to {}", path.display()));
                                    }
                                    Some(AppAction::LoadSession(path)) => {
                                        app.status_message = Some(format!("Session loaded from {}", path.display()));
                                    }
                                    Some(AppAction::SendMessage(_)) | None => {}
                                }
                            } else {
                                app.add_message(ChatMessage::user(&input));
                                app.is_loading = true;

                                // Create client on demand (lazy)
                                match AnthropicClient::new(&config_with_key) {
                                    Ok(client) => {
                                        match send_message(&client, app, &input).await {
                                            Ok(response) => {
                                                for msg in response {
                                                    app.add_message(msg);
                                                }
                                            }
                                            Err(e) => {
                                                app.add_message(ChatMessage::system(&format!("Error: {}", e)));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app.add_message(ChatMessage::system(&format!(
                                            "API key not configured. Set MINIMAX_API_KEY or edit ~/.minimax/config.toml\n\nError: {}",
                                            e
                                        )));
                                    }
                                }
                                app.is_loading = false;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        app.delete_char();
                    }
                    KeyCode::Delete => {
                        app.delete_char_forward();
                    }
                    KeyCode::Left => {
                        app.move_cursor_left();
                    }
                    KeyCode::Right => {
                        app.move_cursor_right();
                    }
                    KeyCode::Home | KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.move_cursor_start();
                    }
                    KeyCode::End | KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.move_cursor_end();
                    }
                    KeyCode::Up => {
                        app.history_up();
                    }
                    KeyCode::Down => {
                        app.history_down();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.clear_input();
                    }
                    KeyCode::Char(c) => {
                        app.insert_char(c);
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn send_message(
    client: &AnthropicClient,
    app: &mut App,
    input: &str,
) -> Result<Vec<ChatMessage>> {
    // Add user message to API messages
    app.api_messages.push(Message {
        role: "user".to_string(),
        content: vec![ContentBlock::Text {
            text: input.to_string(),
            cache_control: None,
        }],
    });

    // Build tools based on mode
    let tools = if matches!(app.mode, AppMode::Agent) {
        Some(crate::agent::build_agent_tools(app.allow_shell))
    } else {
        None
    };

    let request = MessageRequest {
        model: app.model.clone(),
        messages: app.api_messages.clone(),
        max_tokens: 4096,
        system: app.system_prompt.clone(),
        tools,
        tool_choice: if app.mode == AppMode::Agent {
            Some(json!({ "type": "auto" }))
        } else {
            None
        },
        metadata: None,
        thinking: None,
        stream: Some(false),
        temperature: None,
        top_p: None,
    };

    let response = client.create_message(request).await?;

    // Update token count
    app.total_tokens += response.usage.input_tokens + response.usage.output_tokens;

    let mut chat_messages = Vec::new();
    let mut assistant_text = String::new();

    for block in &response.content {
        match block {
            ContentBlock::Thinking { thinking } => {
                chat_messages.push(ChatMessage::thinking(thinking));
            }
            ContentBlock::Text { text, .. } => {
                assistant_text.push_str(text);
            }
            ContentBlock::ToolUse { id: _, name, input } => {
                chat_messages.push(ChatMessage::tool_call(
                    name,
                    &serde_json::to_string_pretty(input).unwrap_or_default(),
                ));
            }
            _ => {}
        }
    }

    if !assistant_text.is_empty() {
        chat_messages.push(ChatMessage::assistant(&assistant_text));
    }

    // Add assistant message to API messages
    app.api_messages.push(Message {
        role: "assistant".to_string(),
        content: response.content,
    });

    Ok(chat_messages)
}

fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    // Show onboarding screen if needed
    if app.onboarding != OnboardingState::None {
        render_onboarding(f, size, app);
        return;
    }

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if size.height > 30 { 8 } else { 3 }),  // Header/Logo
            Constraint::Min(10),                                       // Chat area
            Constraint::Length(3),                                     // Input
            Constraint::Length(1),                                     // Status bar
        ])
        .split(size);

    render_header(f, chunks[0], app);
    render_chat(f, chunks[1], app);
    render_input(f, chunks[2], app);
    render_status_bar(f, chunks[3], app);

    if app.show_help {
        render_help_popup(f, size);
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(MINIMAX_RED));

    if area.height >= 8 {
        // Show full logo
        let logo_lines: Vec<Line> = LOGO
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let color = match i % 3 {
                    0 => MINIMAX_RED,
                    1 => MINIMAX_CORAL,
                    _ => MINIMAX_ORANGE,
                };
                Line::from(Span::styled(line, Style::default().fg(color).bold()))
            })
            .collect();

        let mut lines = logo_lines;
        lines.push(Line::from(vec![
            Span::styled("Unofficial CLI ", Style::default().fg(Color::DarkGray)),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled("Not affiliated with MiniMax", Style::default().fg(Color::DarkGray).italic()),
        ]));

        let paragraph = Paragraph::new(lines).block(block).centered();
        f.render_widget(paragraph, area);
    } else {
        // Show compact header
        let header = Line::from(vec![
            Span::styled(LOGO_SMALL, Style::default().fg(MINIMAX_RED).bold()),
            Span::raw(" "),
            Span::styled(
                format!("[{}]", app.mode.label()),
                Style::default().fg(mode_color(app.mode)).bold(),
            ),
            Span::raw(" "),
            Span::styled(&app.model, Style::default().fg(Color::DarkGray)),
        ]);
        let paragraph = Paragraph::new(header).block(block).centered();
        f.render_widget(paragraph, area);
    }
}

fn render_chat(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" Chat ", Style::default().fg(MINIMAX_CORAL).bold()),
            Span::styled(
                format!("[{}] ", app.mode.label()),
                Style::default().fg(mode_color(app.mode)),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Calculate visible messages based on scroll
    let visible_height = inner.height as usize;
    let total_lines: usize = app.messages.iter().map(|m| count_lines(&m.content, inner.width as usize) + 1).sum();

    let skip_lines = if total_lines > visible_height {
        (total_lines - visible_height).saturating_sub(app.scroll_offset)
    } else {
        0
    };

    let mut items: Vec<ListItem> = Vec::new();
    let mut accumulated_lines = 0;

    for msg in &app.messages {
        let msg_lines = count_lines(&msg.content, inner.width as usize) + 1;

        if accumulated_lines + msg_lines <= skip_lines {
            accumulated_lines += msg_lines;
            continue;
        }

        let style = message_style(msg);
        let prefix = message_prefix(msg);

        let mut lines = vec![Line::from(vec![
            Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
        ])];

        for line in msg.content.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                style,
            )));
        }

        items.push(ListItem::new(lines));
        accumulated_lines += msg_lines;
    }

    let list = List::new(items);
    f.render_widget(list, inner);

    // Loading indicator
    if app.is_loading {
        let loading = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("● ", Style::default().fg(MINIMAX_CORAL)),
            Span::styled("Thinking...", Style::default().fg(Color::DarkGray).italic()),
        ]));
        let loading_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        f.render_widget(loading, loading_area);
    }
}

fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let mode_indicator = format!(" {} ", app.mode.label());
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(mode_indicator, Style::default().fg(Color::Black).bg(mode_color(app.mode)).bold()),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(mode_color(app.mode)));

    let inner = block.inner(area);

    // Input text with cursor
    let input_text = if app.input.is_empty() {
        Span::styled("Type a message or /help for commands...", Style::default().fg(Color::DarkGray).italic())
    } else {
        Span::raw(&app.input)
    };

    let input = Paragraph::new(input_text).block(block);
    f.render_widget(input, area);

    // Position cursor
    let cursor_x = inner.x + app.cursor_position as u16;
    let cursor_y = inner.y;
    if cursor_x < inner.x + inner.width {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled(" F1", Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled(" Help ", Style::default().fg(Color::DarkGray)),
        Span::styled(" /mode", Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled(" Switch ", Style::default().fg(Color::DarkGray)),
        Span::styled(" /yolo", Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled(" Agent+Shell ", Style::default().fg(Color::DarkGray)),
    ];

    // Add status message or token count
    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(msg, Style::default().fg(Color::Yellow)));
    } else {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!("Tokens: {}", app.total_tokens),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Add compact indicator
    if app.auto_compact {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled("COMPACT", Style::default().fg(Color::Green)));
    }

    let status = Paragraph::new(Line::from(spans));
    f.render_widget(status, area);
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_width = 60.min(area.width - 4);
    let popup_height = 20.min(area.height - 4);

    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![
            Span::styled("MiniMax CLI Help", Style::default().fg(MINIMAX_RED).bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Modes:", Style::default().fg(MINIMAX_CORAL).bold()),
        ]),
        Line::from("  /mode normal  - Chat mode (default)"),
        Line::from("  /mode edit    - Edit mode (file modification)"),
        Line::from("  /mode agent   - Agent mode (tool execution)"),
        Line::from("  /mode plan    - Plan mode (design first)"),
        Line::from("  /mode rlm     - RLM sandbox mode"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Commands:", Style::default().fg(MINIMAX_CORAL).bold()),
        ]),
        Line::from("  /help         - Show this help"),
        Line::from("  /clear        - Clear conversation"),
        Line::from("  /model <name> - Change model"),
        Line::from("  /yolo         - Enable agent + shell"),
        Line::from("  /compact      - Toggle auto-compaction"),
        Line::from("  /save <path>  - Save session"),
        Line::from("  /load <path>  - Load session"),
        Line::from("  /exit         - Exit application"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press Esc or Enter to close", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled(" Help ", Style::default().fg(MINIMAX_RED).bold()),
                ]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINIMAX_CORAL)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(help, popup_area);
}

fn render_onboarding(f: &mut Frame, area: Rect, app: &App) {
    // Clear the entire screen with a dark background
    let block = Block::default()
        .style(Style::default().bg(Color::Black));
    f.render_widget(block, area);

    // Center the content
    let content_width = 70.min(area.width.saturating_sub(4));
    let content_height = 24.min(area.height.saturating_sub(4));
    let content_area = Rect {
        x: (area.width - content_width) / 2,
        y: (area.height - content_height) / 2,
        width: content_width,
        height: content_height,
    };

    match app.onboarding {
        OnboardingState::Welcome => {
            let mut lines = vec![];

            // Logo
            for (i, line) in LOGO.lines().enumerate() {
                let color = match i % 3 {
                    0 => MINIMAX_RED,
                    1 => MINIMAX_CORAL,
                    _ => MINIMAX_ORANGE,
                };
                lines.push(Line::from(Span::styled(line, Style::default().fg(color).bold())));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Welcome to ", Style::default().fg(Color::White)),
                Span::styled("MiniMax CLI", Style::default().fg(MINIMAX_RED).bold()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Unofficial CLI for MiniMax M2.1 API",
                Style::default().fg(Color::DarkGray).italic(),
            )));
            lines.push(Line::from(Span::styled(
                "Not affiliated with MiniMax Inc.",
                Style::default().fg(Color::DarkGray).italic(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Chat API ", Style::default().fg(Color::Green).bold()),
                Span::styled("(Normal Mode) - ", Style::default().fg(Color::White)),
                Span::styled("Ready", Style::default().fg(Color::Green)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Coding API ", Style::default().fg(Color::Yellow).bold()),
                Span::styled("(Plan/Edit Mode) - ", Style::default().fg(Color::White)),
                Span::styled("Coming Soon", Style::default().fg(Color::Yellow).italic()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "To get started, you'll need a MiniMax API key.",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(Span::styled(
                "Get yours at: https://platform.minimax.chat",
                Style::default().fg(MINIMAX_CORAL),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::White).bold()),
                Span::styled(" to enter your API key", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("Ctrl+C", Style::default().fg(Color::White).bold()),
                Span::styled(" to exit", Style::default().fg(Color::DarkGray)),
            ]));

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MINIMAX_RED)))
                .centered();
            f.render_widget(paragraph, content_area);
        }
        OnboardingState::EnteringKey => {
            let mut lines = vec![];

            lines.push(Line::from(Span::styled(
                "Enter Your API Key",
                Style::default().fg(MINIMAX_RED).bold(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Paste your MiniMax API key below:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));

            // API key input field (masked)
            let masked_key = if app.api_key_input.is_empty() {
                Span::styled("(paste your key here)", Style::default().fg(Color::DarkGray).italic())
            } else {
                // Show first 8 chars, mask the rest
                let visible = app.api_key_input.chars().take(8).collect::<String>();
                let hidden = "*".repeat(app.api_key_input.len().saturating_sub(8));
                Span::styled(format!("{}{}", visible, hidden), Style::default().fg(Color::Green))
            };
            lines.push(Line::from(masked_key));
            lines.push(Line::from(""));
            lines.push(Line::from(""));

            // Status message
            if let Some(ref msg) = app.status_message {
                lines.push(Line::from(Span::styled(msg, Style::default().fg(Color::Yellow))));
                lines.push(Line::from(""));
            }

            lines.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::White).bold()),
                Span::styled(" to save", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::White).bold()),
                Span::styled(" to go back", Style::default().fg(Color::DarkGray)),
            ]));

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MINIMAX_CORAL)))
                .centered();
            f.render_widget(paragraph, content_area);
        }
        OnboardingState::Success => {
            let mut lines = vec![];

            lines.push(Line::from(Span::styled(
                "API Key Saved!",
                Style::default().fg(Color::Green).bold(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Your API key has been saved to:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(Span::styled(
                "~/.minimax/config.toml",
                Style::default().fg(MINIMAX_CORAL),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "You're all set! Start chatting with MiniMax M2.1",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::White).bold()),
                Span::styled(" to continue", Style::default().fg(Color::DarkGray)),
            ]));

            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)))
                .centered();
            f.render_widget(paragraph, content_area);
        }
        OnboardingState::None => {}
    }
}

fn message_style(msg: &ChatMessage) -> Style {
    if msg.is_thinking {
        Style::default().fg(Color::Yellow).italic()
    } else if msg.is_tool_call {
        Style::default().fg(Color::Cyan)
    } else {
        match msg.role.as_str() {
            "user" => Style::default().fg(Color::Green),
            "assistant" => Style::default().fg(Color::White),
            "system" => Style::default().fg(Color::DarkGray).italic(),
            _ => Style::default(),
        }
    }
}

fn message_prefix(msg: &ChatMessage) -> String {
    if msg.is_thinking {
        "💭 Thinking:".to_string()
    } else if msg.is_tool_call {
        format!("🔧 Tool: {}", msg.tool_name.as_deref().unwrap_or("unknown"))
    } else {
        match msg.role.as_str() {
            "user" => "You:".to_string(),
            "assistant" => "MiniMax:".to_string(),
            "system" => "System:".to_string(),
            _ => format!("{}:", msg.role),
        }
    }
}

fn count_lines(text: &str, width: usize) -> usize {
    text.lines()
        .map(|line| (line.len() / width.max(1)) + 1)
        .sum()
}
