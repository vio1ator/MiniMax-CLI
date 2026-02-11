//! Core commands: help, clear, exit, model

use std::fmt::Write;

use crate::tools::plan::PlanState;
use crate::tui::app::{App, AppAction};
use crate::tui::views::{HelpView, ModalKind};

use super::CommandResult;

/// Show help information
pub fn help(app: &mut App, topic: Option<&str>) -> CommandResult {
    if let Some(topic) = topic {
        // Show help for specific command
        if let Some(cmd) = super::get_command_info(topic) {
            let mut help = format!(
                "{}\n\n  {}\n\n  Usage: {}",
                cmd.name, cmd.description, cmd.usage
            );
            if !cmd.aliases.is_empty() {
                let _ = write!(help, "\n  Aliases: {}", cmd.aliases.join(", "));
            }
            return CommandResult::message(help);
        }
        return CommandResult::error(format!("Unknown command: {topic}"));
    }

    // Show help overlay
    if app.view_stack.top_kind() != Some(ModalKind::Help) {
        app.view_stack.push(HelpView::new());
    }
    CommandResult::ok()
}

/// Clear conversation history
pub fn clear(app: &mut App) -> CommandResult {
    app.history.clear();
    app.mark_history_updated();
    app.api_messages.clear();
    app.transcript_selection.clear();
    app.total_conversation_tokens = 0;
    app.clear_todos();
    if let Ok(mut plan) = app.plan_state.lock() {
        *plan = PlanState::default();
    }
    app.tool_log.clear();
    CommandResult::message("Conversation cleared")
}

/// Exit the application
pub fn exit() -> CommandResult {
    CommandResult::action(AppAction::Quit)
}

use crate::settings::Settings;

/// Switch or view current model
///
/// When called without arguments, opens an interactive model picker.
/// When called with an argument, validates and sets the model directly.
pub fn model(app: &mut App, model_name: Option<&str>) -> CommandResult {
    if let Some(name) = model_name {
        // Validate the model name
        let available = crate::tui::model_picker::available_models();
        let mut model_found = None;
        for m in &available {
            if m.id.eq_ignore_ascii_case(name)
                || m.name.eq_ignore_ascii_case(name)
                || m.id.to_lowercase() == name.to_lowercase()
            {
                model_found = Some(m.clone());
                break;
            }
        }

        if let Some(model_info) = model_found {
            let old_model = app.model.clone();
            let new_model = model_info.id.clone();
            app.model = new_model.clone();

            // Persist to settings
            let mut settings = Settings::load().unwrap_or_default();
            settings.default_model = Some(new_model.clone());
            if let Err(e) = settings.save() {
                return CommandResult::message(format!(
                    "Model changed: {old_model} → {new_model} (failed to save: {e})"
                ));
            }

            CommandResult::message(format!(
                "Model changed: {old_model} → {new_model} (saved)\n\n{}",
                model_info.description
            ))
        } else {
            // Invalid model - show available models
            let available = crate::tui::model_picker::available_models()
                .iter()
                .map(|m| format!("  • {} - {}", m.id, m.name))
                .collect::<Vec<_>>()
                .join("\n");
            CommandResult::error(format!(
                "Unknown model: '{name}'\n\nAvailable models:\n{available}"
            ))
        }
    } else {
        // No argument - open the interactive picker
        CommandResult::action(crate::tui::app::AppAction::OpenModelPicker)
    }
}

/// List sub-agent status from the engine
pub fn subagents(_app: &mut App) -> CommandResult {
    CommandResult::with_message_and_action("Fetching sub-agent status...", AppAction::ListSubAgents)
}

/// Show dashboard and docs links
pub fn axiom_links() -> CommandResult {
    CommandResult::message(
        "Axiom Links:\n\
  ─────────────────────────────\n\
  Dashboard: https://platform.axiom.io\n\
  Docs:      https://platform.axiom.io/docs\n\n\
  Tip: API keys are available in the dashboard console.",
    )
}

/// Copy last assistant message (or Nth message) to clipboard
pub fn copy(app: &mut App, arg: Option<&str>) -> CommandResult {
    use crate::tui::history::HistoryCell;

    if app.history.is_empty() {
        return CommandResult::error("No messages to copy");
    }

    let content = if let Some(n_str) = arg {
        // Copy specific message by 1-indexed number
        match n_str.parse::<usize>() {
            Ok(n) if n >= 1 && n <= app.history.len() => {
                extract_text_from_cell(&app.history[n - 1])
            }
            Ok(n) => {
                return CommandResult::error(format!(
                    "Message {n} out of range (1-{})",
                    app.history.len()
                ));
            }
            Err(_) => {
                return CommandResult::error("Usage: /copy [n]  — n is a message number");
            }
        }
    } else {
        // No arg: copy last assistant message
        app.history
            .iter()
            .rev()
            .find_map(|cell| {
                if let HistoryCell::Assistant { content, .. } = cell {
                    Some(content.clone())
                } else {
                    None
                }
            })
            .ok_or(())
            .unwrap_or_default()
    };

    if content.is_empty() {
        return CommandResult::error("No assistant message to copy");
    }

    match app.clipboard.write_text(&content) {
        Ok(()) => CommandResult::message("Copied to clipboard ✓"),
        Err(e) => CommandResult::error(format!("Failed to copy: {e}")),
    }
}

fn extract_text_from_cell(cell: &crate::tui::history::HistoryCell) -> String {
    use crate::tui::history::HistoryCell;
    match cell {
        HistoryCell::User { content }
        | HistoryCell::Assistant { content, .. }
        | HistoryCell::System { content } => content.clone(),
        HistoryCell::ThinkingSummary { summary } => summary.clone(),
        HistoryCell::Error { message, .. } => message.clone(),
        HistoryCell::Tool(_) => String::new(),
    }
}
