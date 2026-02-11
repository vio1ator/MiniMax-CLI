//! Interactive model picker for switching between models.
//!
//! Provides a simple list-based picker for available models with descriptions.

use crate::config::Config;
use crate::models::ModelListResponse;
use crate::palette;
use crate::tui::views::{ModalKind, ModalView, ViewAction, ViewEvent};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Information about a model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capabilities: String,
}

/// Available models as owned Strings (fallback when API fetch fails)
pub fn available_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "model-01".to_string(),
            name: "Model 01".to_string(),
            description: "General-purpose large language model with strong reasoning".to_string(),
            capabilities: "Text generation, reasoning, analysis".to_string(),
        },
        ModelInfo {
            id: "text-01".to_string(),
            name: "Text 01".to_string(),
            description: "Text-optimized model for natural language tasks".to_string(),
            capabilities: "Text generation, summarization, Q&A".to_string(),
        },
        ModelInfo {
            id: "coding-01".to_string(),
            name: "Coding 01".to_string(),
            description: "Code-specialized model for programming tasks".to_string(),
            capabilities: "Code generation, debugging, review".to_string(),
        },
    ]
}

/// Result of a model selection
#[derive(Debug, Clone)]
pub enum ModelPickerResult {
    /// User selected a model
    Selected(String),
    /// User cancelled
    Cancelled,
}

/// Interactive picker for selecting a model
pub struct ModelPicker {
    /// Currently selected index
    selected: usize,
    /// ID of the currently active model (to highlight)
    current_model: String,
    /// List of available models
    models: Vec<ModelInfo>,
    /// Error message if API fetch failed (empty if no error)
    error_message: String,
    /// Whether models were fetched from API (false = fallback mode)
    from_api: bool,
}

impl ModelPicker {
    /// Create a new model picker
    pub fn new(current_model: String, config: &Config) -> Self {
        let base_url = config.anthropic_base_url();
        let api_key = config.anthropic_api_key().unwrap_or_default();
        let mut error_message = String::new();
        let mut from_api = false;

        // Fetch models synchronously using std::thread
        let models_result = std::thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            let url = format!("{}/v1/models", base_url);

            let mut request = client.get(&url);
            request = request.header("x-api-key", &api_key);
            request = request.header("anthropic-version", "2023-06-01");
            request = request.header("content-type", "application/json");

            match request.send() {
                Ok(response) if response.status().is_success() => {
                    let response_text = response.text().unwrap_or_default();
                    match serde_json::from_str::<ModelListResponse>(&response_text) {
                        Ok(result) => {
                            // Try models first (Axiom format), then data (vLLM/OpenAI format)
                            let models_vec = if !result.models.is_empty() {
                                result.models
                            } else {
                                result.data
                            };
                            let models: Vec<ModelInfo> = models_vec
                                .into_iter()
                                .map(|m| {
                                    let model_id = m.id.clone();
                                    ModelInfo {
                                        id: m.id,
                                        name: model_id,
                                        description: "Model from API".to_string(),
                                        capabilities: "Text generation".to_string(),
                                    }
                                })
                                .collect();
                            (models, true, String::new())
                        }
                        Err(e) => {
                            // Fallback to default models on parse error
                            let fallback = available_models();
                            (
                                fallback,
                                false,
                                format!(
                                    "Failed to parse API response: {}\n\nResponse: {}",
                                    e, response_text
                                ),
                            )
                        }
                    }
                }
                Ok(response) => {
                    // Non-success status code
                    let status = response.status();
                    let text = response.text().unwrap_or_default();
                    let fallback = available_models();
                    (
                        fallback,
                        false,
                        format!("API returned HTTP {}: {}", status, text),
                    )
                }
                Err(e) => {
                    // Network/connection error
                    let fallback = available_models();
                    (fallback, false, format!("Connection failed: {}", e))
                }
            }
        })
        .join()
        .unwrap_or_else(|_| {
            let fallback = available_models();
            (fallback, false, "Thread panicked while fetching models".to_string())
        });

        let (models, from_api_result, error) = models_result;
        error_message = error;
        from_api = from_api_result;

        let selected = models
            .iter()
            .position(|m| m.id == current_model)
            .unwrap_or(0);

        Self {
            selected,
            current_model,
            models,
            error_message,
            from_api,
        }
    }

    /// Get the currently selected model ID
    pub fn selected_model_id(&self) -> Option<String> {
        self.models.get(self.selected).map(|m| m.id.clone())
    }

    /// Check if a model is the currently active one
    fn is_current_model(&self, id: &str) -> bool {
        self.current_model == id
    }

    /// Get the error message (if any)
    pub fn error_message(&self) -> &str {
        &self.error_message
    }

    /// Check if models were fetched from API (false = using fallback)
    pub fn from_api(&self) -> bool {
        self.from_api
    }

    /// Move selection up
    fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.models.len().saturating_sub(1);
        }
    }

    /// Move selection down
    fn select_down(&mut self) {
        if self.selected < self.models.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    /// Render a model item
    fn render_model_item(&self, model: &ModelInfo, index: usize) -> ListItem<'static> {
        let _is_selected = index == self.selected;
        let is_current = self.is_current_model(&model.id);

        // Current indicator
        let current_indicator = if is_current { " ● " } else { "   " };

        let name = model.name.clone();
        let description = model.description.clone();
        let capabilities = model.capabilities.clone();

        let mut lines = vec![];

        // Title line with model name and current indicator
        let title_line = Line::from(vec![Span::raw(current_indicator), Span::raw(name)]);
        lines.push(title_line);

        // Description line
        lines.push(Line::from(vec![Span::raw("     "), Span::raw(description)]));

        // Capabilities line
        lines.push(Line::from(vec![
            Span::raw("     "),
            Span::raw(format!("Capabilities: {}", capabilities)),
        ]));

        // Spacing between items
        lines.push(Line::from(""));

        ListItem::new(lines)
    }
}

impl ModalView for ModelPicker {
    fn kind(&self) -> ModalKind {
        ModalKind::ModelPicker
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        match key.code {
            KeyCode::Esc => ViewAction::EmitAndClose(ViewEvent::ModelPickerResult {
                result: ModelPickerResult::Cancelled,
            }),
            KeyCode::Enter => {
                if let Some(id) = self.selected_model_id() {
                    ViewAction::EmitAndClose(ViewEvent::ModelPickerResult {
                        result: ModelPickerResult::Selected(id),
                    })
                } else {
                    ViewAction::Close
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_up();
                ViewAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_down();
                ViewAction::None
            }
            _ => ViewAction::None,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Create a centered popup
        let popup_width = (area.width * 3 / 5).clamp(50, 70);
        let popup_height = (self.models.len() as u16 * 5 + 6).min(area.height - 4);
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the background
        Clear.render(popup_area, buf);

        // Draw the border
        let block = Block::default()
            .title(" Model Selection ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette::BLUE));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        // Model list
        let items: Vec<ListItem> = self
            .models
            .iter()
            .enumerate()
            .map(|(i, m)| self.render_model_item(m, i))
            .collect();

        let models_list = List::new(items);
        models_list.render(chunks[0], buf);

        // Help footer
        let help_text = format!(
            "↑/↓ to navigate | Enter to select | Esc to cancel | {} models",
            self.models.len()
        );

        // Build footer lines
        let mut footer_lines: Vec<Line<'static>> = vec![];

        // Error/warning line (if any)
        if !self.error_message.is_empty() {
            let error_span = if self.from_api {
                Span::styled(
                    format!("⚠ API warning: {}", self.error_message),
                    Style::default()
                        .fg(palette::YELLOW)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!("⚠ Connection failed: {}", self.error_message),
                    Style::default()
                        .fg(palette::RED)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
            };
            footer_lines.push(Line::from(vec![error_span]));
            footer_lines.push(Line::from(""));
        }

        // Fallback warning if models are from fallback
        if !self.from_api && self.error_message.is_empty() {
            footer_lines.push(Line::from(vec![Span::styled(
                "📡 Models loaded from fallback (no API connection)",
                Style::default().fg(palette::YELLOW),
            )]));
            footer_lines.push(Line::from(""));
        }

        // Help text line
        footer_lines.push(Line::from(vec![Span::styled(
            help_text,
            Style::default().fg(palette::TEXT_DIM),
        )]));

        let help = Paragraph::new(footer_lines);
        help.render(chunks[1], buf);
    }
}

/// Get available models info
#[allow(dead_code)]
pub fn get_model_info(model_name: &str) -> Option<ModelInfo> {
    let models = available_models();
    models.into_iter().find(|m| {
        m.id.eq_ignore_ascii_case(model_name)
            || m.name.eq_ignore_ascii_case(model_name)
            || m.id.to_lowercase() == model_name.to_lowercase()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_info_exact_match() {
        let model = get_model_info("model-01");
        assert!(model.is_some());
        assert_eq!(model.unwrap().id, "model-01");
    }

    #[test]
    fn test_get_model_info_case_insensitive() {
        let model = get_model_info("claude-3-5-sonnet-20241022");
        assert!(model.is_none()); // won't match, but doesn't crash
    }

    #[test]
    fn test_get_model_info_not_found() {
        let model = get_model_info("NonExistent-Model");
        assert!(model.is_none());
    }

    #[test]
    fn test_available_models() {
        let models = available_models();
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].id, "model-01");
        assert_eq!(models[1].id, "text-01");
        assert_eq!(models[2].id, "coding-01");
    }

    #[test]
    fn test_model_picker_navigation() {
        let mut picker = ModelPicker::new("model-01".to_string(), &Config::default());
        assert_eq!(picker.selected, 0);

        // Move down
        picker.select_down();
        assert_eq!(picker.selected, 1);

        // Move down again
        picker.select_down();
        assert_eq!(picker.selected, 2);

        // Wrap around
        picker.select_down();
        assert_eq!(picker.selected, 0);

        // Move up
        picker.select_up();
        assert_eq!(picker.selected, 2);
    }
}
