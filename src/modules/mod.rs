//! `MiniMax` multimedia modules.
//!
//! These modules provide native `MiniMax` API access for:
//! - Text-to-Speech (TTS) and voice cloning
//! - Video generation (text-to-video, image-to-video)
//! - Image generation
//! - Music generation
//!
//! Currently these are standalone functions that can be called directly.
//! Future work: Convert to agent tools with `ApprovalLevel::Required`.

#![allow(dead_code)] // Public API - multimedia functions for future tool integration

pub mod audio;
pub mod files;
pub mod image;
pub mod music;
pub mod text;
pub mod video;
