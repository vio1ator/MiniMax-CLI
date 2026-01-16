mod renderable;

pub use renderable::Renderable;

use crate::tui::app::{App, AppMode};
use crate::tui::approval::{ApprovalRequest, ToolCategory};
use crate::tui::scrolling::TranscriptScroll;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Stylize,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub struct ChatWidget {
    content_area: Rect,
    scrollbar_area: Option<Rect>,
    lines: Vec<Line<'static>>,
    scrollbar: Option<ScrollbarState>,
}

struct ScrollbarState {
    top: usize,
    visible_lines: usize,
    total_lines: usize,
}

impl ChatWidget {
    pub fn new(app: &mut App, area: Rect) -> Self {
        let mut content_area = area;
        let mut scrollbar_area = None;

        let show_scrollbar = !matches!(app.transcript_scroll, TranscriptScroll::ToBottom)
            && area.width > 1
            && area.height > 1;
        if show_scrollbar {
            content_area.width = content_area.width.saturating_sub(1);
            scrollbar_area = Some(Rect {
                x: content_area.x + content_area.width,
                y: content_area.y,
                width: 1,
                height: content_area.height,
            });
        }

        app.transcript_cache
            .ensure(&app.history, content_area.width.max(1), app.history_version);

        let total_lines = app.transcript_cache.total_lines();
        let visible_lines = content_area.height as usize;
        let line_meta = app.transcript_cache.line_meta();

        if app.pending_scroll_delta != 0 {
            app.transcript_scroll = app.transcript_scroll.scrolled_by(
                app.pending_scroll_delta,
                line_meta,
                visible_lines,
            );
            app.pending_scroll_delta = 0;
        }

        let max_start = total_lines.saturating_sub(visible_lines);
        let (scroll_state, top) = app.transcript_scroll.resolve_top(line_meta, max_start);
        app.transcript_scroll = scroll_state;

        app.last_transcript_area = Some(content_area);
        app.last_scrollbar_area = scrollbar_area;
        app.last_transcript_top = top;
        app.last_transcript_visible = visible_lines;
        app.last_transcript_total = total_lines;
        app.last_transcript_padding_top = 0;

        let end = (top + visible_lines).min(total_lines);
        let mut lines = if total_lines == 0 {
            vec![Line::from("")]
        } else {
            app.transcript_cache.lines()[top..end].to_vec()
        };

        apply_selection(&mut lines, top, app);

        if matches!(app.transcript_scroll, TranscriptScroll::ToBottom) {
            app.last_transcript_padding_top = visible_lines.saturating_sub(lines.len());
            pad_lines_to_bottom(&mut lines, visible_lines);
        }

        let scrollbar = scrollbar_area.map(|_| ScrollbarState {
            top,
            visible_lines,
            total_lines,
        });

        Self {
            content_area,
            scrollbar_area,
            lines,
            scrollbar,
        }
    }
}

impl Renderable for ChatWidget {
    fn render(&self, _area: Rect, buf: &mut Buffer) {
        let paragraph = Paragraph::new(self.lines.clone());
        paragraph.render(self.content_area, buf);

        if let (Some(scrollbar_area), Some(scrollbar)) = (self.scrollbar_area, &self.scrollbar) {
            render_scrollbar(
                buf,
                scrollbar_area,
                scrollbar.top,
                scrollbar.visible_lines,
                scrollbar.total_lines,
            );
        }
    }

    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

pub struct ComposerWidget<'a> {
    app: &'a App,
    prompt: &'a str,
    max_height: u16,
}

impl<'a> ComposerWidget<'a> {
    pub fn new(app: &'a App, prompt: &'a str, max_height: u16) -> Self {
        Self {
            app,
            prompt,
            max_height,
        }
    }
}

impl Renderable for ComposerWidget<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let prompt_width = self.prompt.width();
        let prompt_width_u16 = u16::try_from(prompt_width).unwrap_or(u16::MAX);
        let content_width = usize::from(area.width.saturating_sub(prompt_width_u16).max(1));
        let max_height = usize::from(area.height);

        let (visible_lines, _cursor_row, _cursor_col) = layout_input(
            &self.app.input,
            self.app.cursor_position,
            content_width,
            max_height,
        );

        let background = Style::default().bg(Color::Rgb(24, 32, 24));
        let block = Block::default().style(background);
        block.render(area, buf);

        let mut lines = Vec::new();
        if self.app.input.is_empty() {
            let placeholder = if self.app.mode == AppMode::Rlm {
                if self.app.rlm_repl_active {
                    "Type an RLM expression or /repl to exit..."
                } else {
                    "Ask a question or /repl to enter expression mode..."
                }
            } else {
                "Type a message or /help for commands..."
            };
            lines.push(Line::from(vec![
                Span::styled(self.prompt, Style::default().fg(Color::Green).bold()),
                Span::styled(placeholder, Style::default().fg(Color::DarkGray).italic()),
            ]));
        } else {
            for (idx, line) in visible_lines.iter().enumerate() {
                let prefix = if idx == 0 { self.prompt } else { "  " };
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Green).bold()),
                    Span::styled(line.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines).style(background);
        paragraph.render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        composer_height(&self.app.input, width, self.max_height, self.prompt)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let prompt_width = self.prompt.width();
        let prompt_width_u16 = u16::try_from(prompt_width).unwrap_or(u16::MAX);
        let content_width = usize::from(area.width.saturating_sub(prompt_width_u16).max(1));
        let max_height = usize::from(area.height);

        let (_visible_lines, cursor_row, cursor_col) = layout_input(
            &self.app.input,
            self.app.cursor_position,
            content_width,
            max_height,
        );

        let cursor_x = area
            .x
            .saturating_add(prompt_width_u16)
            .saturating_add(u16::try_from(cursor_col).unwrap_or(u16::MAX));
        let cursor_y = area
            .y
            .saturating_add(u16::try_from(cursor_row).unwrap_or(u16::MAX));
        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            Some((cursor_x, cursor_y))
        } else {
            None
        }
    }
}

pub struct ApprovalWidget<'a> {
    request: &'a ApprovalRequest,
    selected: usize,
}

impl<'a> ApprovalWidget<'a> {
    pub fn new(request: &'a ApprovalRequest, selected: usize) -> Self {
        Self { request, selected }
    }
}

impl Renderable for ApprovalWidget<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let popup_width = 65.min(area.width.saturating_sub(4));
        let popup_height = 18.min(area.height.saturating_sub(4));
        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        Clear.render(popup_area, buf);

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Tool: "),
                Span::styled(
                    &self.request.tool_name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let category_label = match self.request.category {
            ToolCategory::Safe => ("Safe", Color::Green),
            ToolCategory::FileWrite => ("File Write", Color::Yellow),
            ToolCategory::Shell => ("Shell Command", Color::Red),
            ToolCategory::PaidMultimedia => ("Paid API", Color::Magenta),
        };
        lines.push(Line::from(vec![
            Span::raw("  Type: "),
            Span::styled(
                category_label.0,
                Style::default()
                    .fg(category_label.1)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        if let Some(cost) = &self.request.estimated_cost {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  Cost: "),
                Span::styled(
                    cost.display(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {}", &cost.breakdown),
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No cost (free operation)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines.push(Line::from(""));
        let params_str = self.request.params_display();
        let params_truncated = crate::utils::truncate_with_ellipsis(&params_str, 50, "...");
        lines.push(Line::from(Span::styled(
            format!("  Params: {params_truncated}"),
            Style::default().fg(Color::DarkGray),
        )));

        lines.push(Line::from(""));

        let options = [
            ("y", "Approve (this time)"),
            ("a", "Approve for session"),
            ("n", "Deny"),
            ("Esc", "Abort turn"),
        ];

        for (i, (key, label)) in options.iter().enumerate() {
            let is_selected = i == self.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("[{key}] "), Style::default().fg(Color::Green)),
                Span::styled(*label, style),
            ]));
        }

        let title = format!(" Approve Tool: {} ", &self.request.tool_name);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });

        paragraph.render(popup_area, buf);
    }

    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

pub(crate) fn pad_lines_to_bottom(lines: &mut Vec<Line<'static>>, height: usize) {
    if lines.len() >= height {
        return;
    }
    let padding = height.saturating_sub(lines.len());
    if padding == 0 {
        return;
    }

    let mut padded = Vec::with_capacity(height);
    padded.extend(std::iter::repeat_n(Line::from(""), padding));
    padded.append(lines);
    *lines = padded;
}

fn apply_selection(lines: &mut [Line<'static>], top: usize, app: &App) {
    let Some((start, end)) = app.transcript_selection.ordered_endpoints() else {
        return;
    };

    let selection_style = Style::default().bg(Color::Rgb(60, 60, 80));

    for (idx, line) in lines.iter_mut().enumerate() {
        let line_index = top + idx;
        if line_index < start.line_index || line_index > end.line_index {
            continue;
        }

        let (col_start, col_end) = if start.line_index == end.line_index {
            (start.column, end.column)
        } else if line_index == start.line_index {
            (start.column, usize::MAX)
        } else if line_index == end.line_index {
            (0, end.column)
        } else {
            (0, usize::MAX)
        };

        let new_spans = apply_selection_to_line(line, col_start, col_end, selection_style);
        line.spans = new_spans;
    }
}

fn apply_selection_to_line(
    line: &Line<'static>,
    col_start: usize,
    col_end: usize,
    selection_style: Style,
) -> Vec<Span<'static>> {
    let mut result = Vec::new();
    let mut current_col = 0usize;

    for span in &line.spans {
        let span_text: &str = span.content.as_ref();
        let span_len = span_text.chars().count();
        let span_end = current_col + span_len;

        if span_end <= col_start || current_col >= col_end {
            result.push(span.clone());
        } else if current_col >= col_start && span_end <= col_end {
            result.push(Span::styled(
                span.content.clone(),
                span.style.patch(selection_style),
            ));
        } else {
            let chars: Vec<char> = span_text.chars().collect();
            let mut before = String::new();
            let mut selected = String::new();
            let mut after = String::new();

            for (i, &ch) in chars.iter().enumerate() {
                let char_col = current_col + i;
                if char_col < col_start {
                    before.push(ch);
                } else if char_col < col_end {
                    selected.push(ch);
                } else {
                    after.push(ch);
                }
            }

            if !before.is_empty() {
                result.push(Span::styled(before, span.style));
            }
            if !selected.is_empty() {
                result.push(Span::styled(selected, span.style.patch(selection_style)));
            }
            if !after.is_empty() {
                result.push(Span::styled(after, span.style));
            }
        }

        current_col = span_end;
    }

    result
}

fn render_scrollbar(buf: &mut Buffer, area: Rect, top: usize, visible: usize, total: usize) {
    if total <= visible || area.height == 0 {
        return;
    }

    let height = usize::from(area.height);
    let max_start = total.saturating_sub(visible).max(1);
    let thumb_height = visible
        .saturating_mul(height)
        .div_ceil(total)
        .clamp(1, height);
    let track = height.saturating_sub(thumb_height).max(1);
    let thumb_start = (top.saturating_mul(track) + max_start / 2) / max_start;

    let mut lines = Vec::new();
    for row in 0..height {
        let ch = if row >= thumb_start && row < thumb_start + thumb_height {
            "#"
        } else {
            "|"
        };
        lines.push(Line::from(Span::styled(
            ch,
            Style::default().fg(Color::DarkGray),
        )));
    }

    let scrollbar = Paragraph::new(lines);
    scrollbar.render(area, buf);
}

fn composer_height(input: &str, width: u16, available_height: u16, prompt: &str) -> u16 {
    let prompt_width = prompt.width();
    let prompt_width_u16 = u16::try_from(prompt_width).unwrap_or(u16::MAX);
    let content_width = usize::from(width.saturating_sub(prompt_width_u16).max(1));
    let mut line_count = wrap_input_lines(input, content_width).len();
    if line_count == 0 {
        line_count = 1;
    }
    let max_height = usize::from(available_height.clamp(1, 8));
    line_count.clamp(1, max_height).try_into().unwrap_or(1)
}

fn layout_input(
    input: &str,
    cursor: usize,
    width: usize,
    max_height: usize,
) -> (Vec<String>, usize, usize) {
    let mut lines = wrap_input_lines(input, width);
    if lines.is_empty() {
        lines.push(String::new());
    }
    let (cursor_row, cursor_col) = cursor_row_col(input, cursor, width.max(1));

    let max_height = max_height.max(1);
    let mut start = 0usize;
    if cursor_row >= max_height {
        start = cursor_row + 1 - max_height;
    }
    if start + max_height > lines.len() {
        start = lines.len().saturating_sub(max_height);
    }
    let visible = lines
        .into_iter()
        .skip(start)
        .take(max_height)
        .collect::<Vec<_>>();
    let visible_cursor_row = cursor_row.saturating_sub(start);

    (
        visible,
        visible_cursor_row,
        cursor_col.min(width.saturating_sub(1)),
    )
}

fn cursor_row_col(input: &str, cursor: usize, width: usize) -> (usize, usize) {
    let mut row = 0usize;
    let mut col = 0usize;

    for (char_idx, ch) in input.chars().enumerate() {
        if char_idx >= cursor {
            break;
        }

        if ch == '\n' {
            row += 1;
            col = 0;
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if col + ch_width > width {
            row += 1;
            col = 0;
        }
        col += ch_width;
        if col >= width {
            row += 1;
            col = 0;
        }
    }

    (row, col)
}

fn wrap_input_lines(input: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    if input.is_empty() {
        return lines;
    }

    for raw in input.split('\n') {
        let wrapped = wrap_text(raw, width);
        if wrapped.is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(wrapped);
        }
    }

    if input.ends_with('\n') {
        lines.push(String::new());
    }

    lines
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);
        if current_width == 0 {
            current.push_str(word);
            current_width = word_width;
            continue;
        }

        if current_width + 1 + word_width <= width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(current);
            current = word.to_string();
            current_width = word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::pad_lines_to_bottom;
    use ratatui::text::Line;

    #[test]
    fn pad_lines_to_bottom_noop_when_already_filled() {
        let mut lines = vec![Line::from("one"), Line::from("two")];
        pad_lines_to_bottom(&mut lines, 2);
        assert_eq!(lines, vec![Line::from("one"), Line::from("two")]);
    }

    #[test]
    fn pad_lines_to_bottom_prepends_empty_lines() {
        let mut lines = vec![Line::from("one"), Line::from("two")];
        pad_lines_to_bottom(&mut lines, 5);

        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], Line::from(""));
        assert_eq!(lines[1], Line::from(""));
        assert_eq!(lines[2], Line::from(""));
        assert_eq!(lines[3], Line::from("one"));
        assert_eq!(lines[4], Line::from("two"));
    }

    #[test]
    fn pad_lines_to_bottom_noop_when_height_is_zero() {
        let mut lines = vec![Line::from("one")];
        pad_lines_to_bottom(&mut lines, 0);
        assert_eq!(lines, vec![Line::from("one")]);
    }
}
