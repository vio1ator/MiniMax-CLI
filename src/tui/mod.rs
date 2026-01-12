//! Terminal UI (TUI) module for `MiniMax` CLI.

// === Submodules ===

pub mod app;
pub mod approval;
pub mod clipboard;
pub mod history;
pub mod scrolling;
pub mod selection;
pub mod streaming;
pub mod transcript;
pub mod ui;

// === Re-exports ===

pub use app::TuiOptions;
pub use ui::run_tui;
