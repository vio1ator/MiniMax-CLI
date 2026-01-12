//! Clipboard handling for paste support in TUI
//!
//! Supports text and image paste operations.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use arboard::{Clipboard, ImageData};

// === Types ===

/// Clipboard payloads supported by the TUI.
pub enum ClipboardContent {
    Text(String),
    Image { path: PathBuf, description: String },
}

/// Clipboard reader/writer helper.
pub struct ClipboardHandler {
    clipboard: Option<Clipboard>,
}

impl ClipboardHandler {
    /// Create a new clipboard handler, falling back to a no-op when unavailable.
    pub fn new() -> Self {
        let clipboard = Clipboard::new().ok();
        Self { clipboard }
    }

    /// Read the clipboard and return the parsed content.
    pub fn read(&mut self, workspace: &Path) -> Option<ClipboardContent> {
        let clipboard = self.clipboard.as_mut()?;
        if let Ok(text) = clipboard.get_text() {
            return Some(ClipboardContent::Text(text));
        }

        if let Ok(image) = clipboard.get_image()
            && let Ok(path) = save_image_to_workspace(workspace, &image)
        {
            let description = format!("image {}x{}", image.width, image.height);
            return Some(ClipboardContent::Image { path, description });
        }

        None
    }

    /// Write text to the clipboard (no-op if unavailable).
    pub fn write_text(&mut self, text: &str) -> Result<()> {
        let Some(clipboard) = self.clipboard.as_mut() else {
            return Ok(());
        };
        clipboard.set_text(text.to_string())?;
        Ok(())
    }
}

fn save_image_to_workspace(workspace: &Path, image: &ImageData) -> Result<PathBuf> {
    let dir = workspace.join("clipboard-images");
    std::fs::create_dir_all(&dir)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!("clipboard-{timestamp}.ppm"));

    let mut data = Vec::with_capacity((image.width * image.height * 3) + 64);
    data.extend_from_slice(format!("P6\n{} {}\n255\n", image.width, image.height).as_bytes());

    let bytes = image.bytes.as_ref();
    for chunk in bytes.chunks(4) {
        let r = chunk.first().copied().unwrap_or(0);
        let g = chunk.get(1).copied().unwrap_or(0);
        let b = chunk.get(2).copied().unwrap_or(0);
        data.extend_from_slice(&[r, g, b]);
    }

    std::fs::write(&path, data)?;
    Ok(path)
}
