use crate::duo::{DuoPhase, SharedDuoSession};
use crate::palette;
use crate::tui::views::{ModalKind, ModalView, ViewAction};
use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{Stylize, Widget},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

const MAX_FEEDBACK_HISTORY: usize = 5;
const MAX_SUMMARY_LENGTH: usize = 80;

pub struct DuoView {
    session: SharedDuoSession,
    scroll: usize,
}

impl DuoView {
    pub fn new(session: SharedDuoSession) -> Self {
        Self { session, scroll: 0 }
    }

    fn get_current_state(&self) -> Option<crate::duo::DuoState> {
        self.session
            .lock()
            .ok()
            .map(|s| s.get_active().cloned())
            .flatten()
    }

    fn format_phase(&self, phase: DuoPhase) -> (String, Style) {
        match phase {
            DuoPhase::Init => (
                "Initializing".to_string(),
                Style::default().fg(palette::MINIMAX_ORANGE),
            ),
            DuoPhase::Player => (
                "Player (Implementing)".to_string(),
                Style::default()
                    .fg(palette::MINIMAX_BLUE)
                    .add_modifier(Modifier::BOLD),
            ),
            DuoPhase::Coach => (
                "Coach (Validating)".to_string(),
                Style::default()
                    .fg(palette::MINIMAX_MAGENTA)
                    .add_modifier(Modifier::BOLD),
            ),
            DuoPhase::Approved => (
                "Approved ✓".to_string(),
                Style::default().fg(palette::MINIMAX_GREEN),
            ),
            DuoPhase::Timeout => (
                "Timeout ✗".to_string(),
                Style::default().fg(palette::MINIMAX_RED),
            ),
        }
    }

    fn format_turns(&self, current: u32, max: u32) -> String {
        format!("Turn {} / {}", current, max)
    }

    fn calculate_quality_bar(&self, scores: &[f64]) -> (f64, String) {
        if scores.is_empty() {
            return (0.0, "━━━━━━━━━━".to_string());
        }

        let avg: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        let percentage = (avg * 100.0).round() as usize;
        let filled: usize = (avg * 10.0).round() as usize;
        let empty: usize = 10usize.saturating_sub(filled);

        let filled_bar = "█".repeat(filled);
        let empty_bar = "░".repeat(empty);

        (
            avg,
            format!("{}{} [{}%]", filled_bar, empty_bar, percentage),
        )
    }

    fn format_summary(&self, summary: &str) -> String {
        if summary.len() <= MAX_SUMMARY_LENGTH {
            return summary.to_string();
        }
        let mut truncated = summary
            .chars()
            .take(MAX_SUMMARY_LENGTH - 3)
            .collect::<String>();
        truncated.push_str("...");
        truncated
    }

    fn render_loop_visualization(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &crate::duo::DuoState,
    ) {
        let player_color = palette::MINIMAX_BLUE;
        let coach_color = palette::MINIMAX_MAGENTA;
        let active_color = palette::MINIMAX_ORANGE;
        let approved_color = palette::MINIMAX_GREEN;
        let timeout_color = palette::MINIMAX_RED;

        let (_current_phase_label, _current_phase_color) = match state.phase {
            DuoPhase::Init => ("●", active_color),
            DuoPhase::Player => ("▶", player_color),
            DuoPhase::Coach => ("▶", coach_color),
            DuoPhase::Approved => ("✓", approved_color),
            DuoPhase::Timeout => ("✗", timeout_color),
        };

        let turn_progress = state.current_turn as f64 / state.max_turns as f64;
        let progress_bar_len: usize = 10;
        let filled_len = (turn_progress * progress_bar_len as f64).round() as usize;
        let progress_bar = format!(
            "{}{}",
            "▓".repeat(filled_len),
            "░".repeat(progress_bar_len.saturating_sub(filled_len))
        );

        let loop_line = if matches!(state.phase, DuoPhase::Approved | DuoPhase::Timeout) {
            match state.phase {
                DuoPhase::Approved => format!("[ {} ]──▶[ ✓ APPROVED ]", progress_bar),
                DuoPhase::Timeout => format!("[ {} ]──▶[ ✗ TIMEOUT ]", progress_bar),
                _ => unreachable!(),
            }
        } else {
            let player_arrow = if matches!(state.phase, DuoPhase::Player) {
                "▶"
            } else {
                "→"
            };
            let coach_arrow = if matches!(state.phase, DuoPhase::Coach) {
                "▶"
            } else {
                "→"
            };
            let player_status = if matches!(state.phase, DuoPhase::Player) {
                format!("Player ●")
            } else {
                "Player".to_string()
            };
            let coach_status = if matches!(state.phase, DuoPhase::Coach) {
                format!("Coach ●")
            } else {
                "Coach".to_string()
            };
            format!(
                "[{}]──{}──[{}]──{}──[{}]",
                player_status, player_arrow, progress_bar, coach_arrow, coach_status
            )
        };

        let mut loop_lines = vec![
            Line::from(Span::styled(
                "Loop Progress",
                Style::default().fg(palette::MINIMAX_ORANGE).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &loop_line,
                Style::default().fg(palette::TEXT_PRIMARY),
            )),
            Line::from(""),
        ];

        let turn_info = format!(
            "Turn {}/{} | {} turns remaining",
            state.current_turn,
            state.max_turns,
            state.turns_remaining()
        );
        let turn_style = if state.current_turn >= state.max_turns - 1 {
            Style::default().fg(palette::STATUS_WARNING)
        } else {
            Style::default().fg(palette::TEXT_MUTED)
        };
        loop_lines.push(Line::from(Span::styled(turn_info, turn_style)));

        let threshold_info = format!(
            "Approval threshold: {:.0}%",
            state.approval_threshold * 100.0
        );
        loop_lines.push(Line::from(Span::styled(
            threshold_info,
            Style::default().fg(palette::TEXT_DIM),
        )));

        let loop_widget = Paragraph::new(loop_lines)
            .style(Style::default().fg(palette::TEXT_PRIMARY))
            .wrap(Wrap::default());

        loop_widget.render(area, buf);
    }

    fn render_header(&self, area: Rect, buf: &mut Buffer, state: &crate::duo::DuoState) {
        let (phase_label, phase_style) = self.format_phase(state.phase);
        let turns_label = self.format_turns(state.current_turn, state.max_turns);
        let (_, quality_bar) = self.calculate_quality_bar(&state.quality_scores);

        let mut header_lines = vec![
            Line::from(Span::styled(
                "Duo Session Status",
                Style::default().fg(palette::MINIMAX_BLUE).bold(),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Phase: ", Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(phase_label, phase_style),
            ]),
            Line::from(vec![
                Span::styled("Turns: ", Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(turns_label, Style::default().fg(palette::TEXT_PRIMARY)),
            ]),
        ];

        if !matches!(state.phase, DuoPhase::Init) {
            header_lines.push(Line::from(""));
            header_lines.push(Line::from(vec![
                Span::styled("Quality: ", Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(quality_bar, Style::default().fg(palette::MINIMAX_ORANGE)),
            ]));
        }

        let header_widget = Paragraph::new(header_lines)
            .style(Style::default().fg(palette::TEXT_PRIMARY))
            .wrap(Wrap::default());

        header_widget.render(area, buf);
    }

    fn render_feedback_history(&self, area: Rect, buf: &mut Buffer, state: &crate::duo::DuoState) {
        let history: Vec<&crate::duo::TurnRecord> = state
            .turn_history
            .iter()
            .rev()
            .take(MAX_FEEDBACK_HISTORY)
            .collect();

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(
                "Feedback History",
                Style::default().fg(palette::MINIMAX_ORANGE).bold(),
            )),
            Line::from(""),
        ];

        for record in history.iter().rev() {
            let (phase_label, phase_style) = self.format_phase(record.phase);
            let summary = self.format_summary(&record.summary);

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("[{}] ", record.turn),
                    Style::default().fg(palette::TEXT_MUTED),
                ),
                Span::styled(phase_label, phase_style),
                Span::styled(": ", Style::default().fg(palette::TEXT_MUTED)),
                Span::styled(summary, Style::default().fg(palette::TEXT_PRIMARY)),
            ]));

            if let Some(score) = record.quality_score {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("Quality: ", Style::default().fg(palette::TEXT_MUTED)),
                    Span::styled(
                        format!("{:.0}%", score * 100.0),
                        Style::default().fg(palette::MINIMAX_GREEN),
                    ),
                ]));
            }
        }

        if history.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No feedback yet...",
                Style::default().fg(palette::TEXT_MUTED),
            )));
        }

        let history_widget = Paragraph::new(lines)
            .style(Style::default().fg(palette::TEXT_PRIMARY))
            .wrap(Wrap::default());

        history_widget.render(area, buf);
    }

    fn render_status_line(&self, area: Rect, buf: &mut Buffer, state: &crate::duo::DuoState) {
        let status_text = match state.status {
            crate::duo::DuoStatus::Active => {
                Span::styled("Active", Style::default().fg(palette::MINIMAX_BLUE))
            }
            crate::duo::DuoStatus::Approved => {
                Span::styled("✓ Approved", Style::default().fg(palette::MINIMAX_GREEN))
            }
            crate::duo::DuoStatus::Rejected => {
                Span::styled("✗ Rejected", Style::default().fg(palette::MINIMAX_RED))
            }
            crate::duo::DuoStatus::Timeout => {
                Span::styled("⏰ Timeout", Style::default().fg(palette::MINIMAX_ORANGE))
            }
        };

        let status_widget = Paragraph::new(vec![Line::from(status_text)])
            .style(Style::default().fg(palette::TEXT_PRIMARY))
            .wrap(Wrap::default());

        status_widget.render(area, buf);
    }
}

impl ModalView for DuoView {
    fn kind(&self) -> ModalKind {
        ModalKind::DuoSession
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ViewAction::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                ViewAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                ViewAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => ViewAction::Close,
            _ => ViewAction::None,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let Some(state) = self.get_current_state() else {
            let msg = Paragraph::new(vec![Line::from("Loading Duo session...")])
                .style(Style::default().fg(palette::TEXT_MUTED))
                .wrap(Wrap::default());
            msg.render(area, buf);
            return;
        };

        let popup_width = 70.min(area.width.saturating_sub(4));
        let popup_height = 32.min(area.height.saturating_sub(4));

        let popup_area = Rect {
            x: (area.width - popup_width) / 2,
            y: (area.height - popup_height) / 2,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        let header_height = 7;
        let loop_height = 8;
        let feedback_height = 12.min(
            popup_height
                .saturating_sub(header_height)
                .saturating_sub(loop_height)
                .saturating_sub(4),
        );
        let status_height = 2;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Length(loop_height),
                Constraint::Length(feedback_height),
                Constraint::Length(status_height),
            ])
            .split(popup_area);

        self.render_header(chunks[0], buf, &state);
        self.render_loop_visualization(chunks[1], buf, &state);
        self.render_feedback_history(chunks[2], buf, &state);
        self.render_status_line(chunks[3], buf, &state);

        let footer = Paragraph::new(vec![Line::from(Span::styled(
            " Esc to close ",
            Style::default().fg(palette::TEXT_MUTED),
        ))])
        .style(Style::default().fg(palette::TEXT_PRIMARY))
        .centered()
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(palette::MINIMAX_ORANGE)),
        );

        footer.render(popup_area, buf);
    }
}
