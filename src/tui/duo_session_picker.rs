//! Interactive Duo session picker for switching between saved Duo sessions.
//!
//! Provides a fuzzy-searchable list of Duo sessions with Duo-specific metadata.

use crate::duo::{DuoPhase, DuoState};
use crate::palette;
use crate::tui::views::{ModalKind, ModalView, ViewAction, ViewEvent};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::collections::HashMap;

const MAX_VISIBLE_SESSIONS: usize = 10;
const MAX_SUMMARY_LENGTH: usize = 60;

#[derive(Debug, Clone)]
pub enum DuoSessionPickerResult {
    Selected(String),
    Cancelled,
}

struct DuoSessionMatch {
    id: String,
    name: String,
    state: DuoState,
    score: i64,
    highlight_indices: Vec<usize>,
}

pub struct DuoSessionPicker {
    sessions: HashMap<String, DuoState>,
    matches: Vec<DuoSessionMatch>,
    query: String,
    selected: usize,
    current_session_id: Option<String>,
    filtering: bool,
}

impl DuoSessionPicker {
    pub fn new(current_session_id: Option<String>) -> Self {
        let sessions = load_duo_sessions();
        let matches = sessions
            .iter()
            .map(|(id, state)| DuoSessionMatch {
                id: id.clone(),
                name: state
                    .session_name
                    .clone()
                    .unwrap_or_else(|| id[..8.min(id.len())].to_string()),
                state: state.clone(),
                score: 0,
                highlight_indices: Vec::new(),
            })
            .collect();

        Self {
            sessions,
            matches,
            query: String::new(),
            selected: 0,
            current_session_id,
            filtering: false,
        }
    }

    pub fn selected_session_id(&self) -> Option<String> {
        self.matches.get(self.selected).map(|m| m.id.clone())
    }

    fn is_current_session(&self, id: &str) -> bool {
        self.current_session_id
            .as_ref()
            .is_some_and(|current| current.starts_with(id) || id.starts_with(current))
    }

    fn insert_char(&mut self, c: char) {
        self.filtering = true;
        self.query.push(c);
        self.update_matches();
    }

    fn backspace(&mut self) {
        self.query.pop();
        if self.query.is_empty() {
            self.filtering = false;
        }
        self.update_matches();
    }

    fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if !self.matches.is_empty() {
            self.selected = self.matches.len() - 1;
        }
    }

    fn select_down(&mut self) {
        if !self.matches.is_empty() && self.selected < self.matches.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    fn update_matches(&mut self) {
        if self.query.is_empty() {
            self.matches = self
                .sessions
                .iter()
                .map(|(id, state)| DuoSessionMatch {
                    id: id.clone(),
                    name: state
                        .session_name
                        .clone()
                        .unwrap_or_else(|| id[..8.min(id.len())].to_string()),
                    state: state.clone(),
                    score: 0,
                    highlight_indices: Vec::new(),
                })
                .collect();
        } else {
            let query_lower = self.query.to_lowercase();
            let mut scored: Vec<DuoSessionMatch> = self
                .sessions
                .iter()
                .filter_map(|(id, state)| {
                    let name_lower = state
                        .session_name
                        .as_deref()
                        .unwrap_or(&id[..8.min(id.len())])
                        .to_lowercase();
                    if let Some((score, indices)) = fuzzy_match(&name_lower, &query_lower) {
                        Some(DuoSessionMatch {
                            id: id.clone(),
                            name: state
                                .session_name
                                .clone()
                                .unwrap_or_else(|| id[..8.min(id.len())].to_string()),
                            state: state.clone(),
                            score,
                            highlight_indices: indices,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            scored.sort_by(|a, b| b.score.cmp(&a.score));
            self.matches = scored;
        }

        if self.selected >= self.matches.len() {
            self.selected = 0;
        }
    }

    fn format_age(&self, dt: &chrono::DateTime<chrono::Utc>) -> String {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(*dt);

        if duration.num_minutes() < 1 {
            "just now".to_string()
        } else if duration.num_hours() < 1 {
            format!("{}m ago", duration.num_minutes())
        } else if duration.num_days() < 1 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_weeks() < 1 {
            format!("{}d ago", duration.num_days())
        } else {
            format!("{}w ago", duration.num_weeks())
        }
    }

    fn format_phase(&self, phase: DuoPhase) -> (String, Style) {
        match phase {
            DuoPhase::Init => ("Init".to_string(), Style::default().fg(palette::MINIMAX_ORANGE)),
            DuoPhase::Player => (
                "Player".to_string(),
                Style::default()
                    .fg(palette::MINIMAX_BLUE)
                    .add_modifier(Modifier::BOLD),
            ),
            DuoPhase::Coach => (
                "Coach".to_string(),
                Style::default()
                    .fg(palette::MINIMAX_MAGENTA)
                    .add_modifier(Modifier::BOLD),
            ),
            DuoPhase::Approved => ("✓ Approved".to_string(), Style::default().fg(palette::MINIMAX_GREEN)),
            DuoPhase::Timeout => ("✗ Timeout".to_string(), Style::default().fg(palette::MINIMAX_RED)),
        }
    }

    fn format_summary(&self, summary: &str) -> String {
        if summary.len() <= MAX_SUMMARY_LENGTH {
            return summary.to_string();
        }
        let mut truncated = summary.chars().take(MAX_SUMMARY_LENGTH - 3).collect::<String>();
        truncated.push_str("...");
        truncated
    }

    fn render_session_item<'a>(&self, session_match: &'a DuoSessionMatch, index: usize) -> ListItem<'a> {
        let state = &session_match.state;
        let is_selected = index == self.selected;
        let is_current = self.is_current_session(&session_match.id);

        let base_style = if is_selected {
            Style::default()
                .bg(palette::MINIMAX_BLUE)
                .fg(palette::MINIMAX_SNOW)
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default()
                .fg(palette::MINIMAX_ORANGE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette::TEXT_PRIMARY)
        };

        let current_indicator = if is_current { " ● " } else { "   " };

        let (phase_label, phase_style) = self.format_phase(state.phase);

        let status_icon = match state.status {
            crate::duo::DuoStatus::Active => "🔄",
            crate::duo::DuoStatus::Approved => "✅",
            crate::duo::DuoStatus::Rejected => "❌",
            crate::duo::DuoStatus::Timeout => "⏰",
        };

        let quality_score = state
            .average_quality_score()
            .map(|s| format!("{:.0}%", s * 100.0))
            .unwrap_or_else(|| "N/A".to_string());

        let meta_text = format!(
            "{} | {} | Turn {}/{} | Quality: {}",
            &session_match.id[..8.min(session_match.id.len())],
            self.format_age(&state.updated_at),
            state.current_turn,
            state.max_turns,
            quality_score
        );

        let meta_style = if is_selected {
            Style::default().fg(palette::MINIMAX_SILVER)
        } else {
            Style::default().fg(palette::TEXT_DIM)
        };

        let mut lines = vec![Line::from(vec![Span::styled(
            current_indicator,
            if is_current {
                Style::default()
                    .fg(palette::MINIMAX_ORANGE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        )])];

        let mut title_line = Line::default();
        title_line.push_span(Span::styled("   ", base_style));
        title_line.push_span(Span::styled(
            status_icon,
            Style::default().fg(if is_selected {
                palette::MINIMAX_SNOW
            } else {
                palette::MINIMAX_GREEN
            }),
        ));
        title_line.push_span(Span::styled(" ", base_style));

        if self.filtering && !session_match.highlight_indices.is_empty() {
            let chars: Vec<char> = session_match.name.chars().collect();
            let mut last_idx = 0;
            for &idx in &session_match.highlight_indices {
                if idx > last_idx && last_idx < chars.len() {
                    title_line.push_span(Span::styled(
                        chars[last_idx..idx].iter().collect::<String>(),
                        base_style,
                    ));
                }
                if idx < chars.len() {
                    title_line.push_span(Span::styled(
                        chars[idx].to_string(),
                        base_style.add_modifier(Modifier::BOLD).fg(palette::MINIMAX_YELLOW),
                    ));
                    last_idx = idx + 1;
                }
            }
            if last_idx < chars.len() {
                title_line.push_span(Span::styled(
                    chars[last_idx..].iter().collect::<String>(),
                    base_style,
                ));
            }
        } else {
            title_line.push_span(Span::styled(&session_match.name, base_style));
        }

        if is_current {
            title_line.push_span(Span::styled(
                " (current)",
                if is_selected {
                    Style::default().fg(palette::MINIMAX_ORANGE)
                } else {
                    Style::default().fg(palette::TEXT_DIM)
                },
            ));
        }
        lines.push(title_line);

        lines.push(Line::from(vec![
            Span::styled("   ", base_style),
            Span::styled(phase_label, phase_style),
        ]));

        lines.push(Line::from(vec![
            Span::styled("   ", base_style),
            Span::styled(meta_text, meta_style),
        ]));

        lines.push(Line::from(""));

        ListItem::new(lines)
    }
}

impl ModalView for DuoSessionPicker {
    fn kind(&self) -> ModalKind {
        ModalKind::DuoSession
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        match key.code {
            KeyCode::Esc => ViewAction::EmitAndClose(ViewEvent::DuoSessionPickerResult {
                result: DuoSessionPickerResult::Cancelled,
            }),
            KeyCode::Enter => {
                if let Some(id) = self.selected_session_id() {
                    ViewAction::EmitAndClose(ViewEvent::DuoSessionSelected { session_id: id })
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
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.insert_char(c);
                ViewAction::None
            }
            KeyCode::Backspace => {
                self.backspace();
                ViewAction::None
            }
            _ => ViewAction::None,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let popup_width = (area.width * 4 / 5).clamp(50, 85);
        let popup_height = (MAX_VISIBLE_SESSIONS as u16 * 4 + 8).min(area.height - 4);
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(" Duo Session Picker ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette::MINIMAX_BLUE));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let query_text = if self.query.is_empty() {
            Line::from(vec![Span::styled(
                "Type to filter sessions...",
                Style::default().fg(palette::TEXT_DIM),
            )])
        } else {
            Line::from(vec![
                Span::styled("Filter: ", Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(&self.query, Style::default().fg(palette::TEXT_PRIMARY)),
            ])
        };
        let query = Paragraph::new(query_text)
            .block(Block::default().borders(Borders::BOTTOM))
            .wrap(Wrap { trim: true });
        query.render(chunks[0], buf);

        let items: Vec<ListItem> = self
            .matches
            .iter()
            .take(MAX_VISIBLE_SESSIONS)
            .enumerate()
            .map(|(i, m)| self.render_session_item(m, i))
            .collect();

        let sessions_list = List::new(items);
        sessions_list.render(chunks[1], buf);

        let help_text = if self.matches.is_empty() {
            "No Duo sessions found".to_string()
        } else {
            format!(
                "↑/↓ to navigate | Enter to select | Esc to cancel | {} sessions",
                self.matches.len()
            )
        };
        let help = Paragraph::new(Line::from(vec![Span::styled(
            help_text,
            Style::default().fg(palette::TEXT_DIM),
        )]));
        help.render(chunks[2], buf);
    }
}

fn load_duo_sessions() -> HashMap<String, DuoState> {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return HashMap::new(),
    };
    rt.block_on(async {
        crate::duo::list_sessions()
            .await
            .unwrap_or_default()
            .into_iter()
            .collect()
    })
}

fn fuzzy_match(haystack: &str, needle: &str) -> Option<(i64, Vec<usize>)> {
    if needle.is_empty() {
        return Some((0, Vec::new()));
    }

    let hay_chars: Vec<char> = haystack.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    let mut indices = Vec::new();
    let mut hay_idx = 0;
    let mut needle_idx = 0;
    let mut score: i64 = 0;
    let mut prev_match_idx: Option<usize> = None;

    while hay_idx < hay_chars.len() && needle_idx < needle_chars.len() {
        let hay_char = hay_chars[hay_idx];
        let needle_char = needle_chars[needle_idx];

        if hay_char == needle_char {
            indices.push(hay_idx);

            let mut char_score: i64 = 1;

            if let Some(prev) = prev_match_idx && hay_idx == prev + 1 {
                char_score += 5;
            }

            if hay_idx == 0 {
                char_score += 3;
            } else if let Some(prev) = hay_idx.checked_sub(1) {
                let prev_char = hay_chars[prev];
                if prev_char == ' ' || prev_char == '_' || prev_char == '-' {
                    char_score += 3;
                }
            }

            if let Some(prev) = prev_match_idx {
                let gap = hay_idx - prev - 1;
                score -= gap as i64;
            }

            score += char_score;
            prev_match_idx = Some(hay_idx);
            needle_idx += 1;
        }

        hay_idx += 1;
    }

    if needle_idx == needle_chars.len() {
        if indices.len() == hay_chars.len() {
            score += 10;
        }
        Some((score, indices))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_basic() {
        let (score, indices) = fuzzy_match("test session", "ts").unwrap();
        assert!(score > 0);
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        assert!(fuzzy_match("hello", "xyz").is_none());
    }

    #[test]
    fn test_fuzzy_match_empty() {
        let (score, indices) = fuzzy_match("hello", "").unwrap();
        assert_eq!(score, 0);
        assert!(indices.is_empty());
    }
}
