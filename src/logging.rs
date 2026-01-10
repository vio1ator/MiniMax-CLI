use colored::Colorize;
use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::SeqCst);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

pub fn info(message: impl AsRef<str>) {
    if is_verbose() {
        eprintln!("{} {}", "info".blue().bold(), message.as_ref());
    }
}

pub fn warn(message: impl AsRef<str>) {
    if is_verbose() {
        eprintln!("{} {}", "warn".yellow().bold(), message.as_ref());
    }
}
