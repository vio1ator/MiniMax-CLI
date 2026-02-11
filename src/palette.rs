//! MiniMax color palette and semantic roles.
//!
//! This module provides the color palette used throughout the MiniMax TUI.

use ratatui::style::Color;

pub const BLUE_RGB: (u8, u8, u8) = (20, 86, 240);
pub const RED_RGB: (u8, u8, u8) = (242, 63, 93);
pub const ORANGE_RGB: (u8, u8, u8) = (255, 99, 58);
pub const MAGENTA_RGB: (u8, u8, u8) = (228, 23, 127);
pub const INK_RGB: (u8, u8, u8) = (24, 30, 37);
pub const BLACK_RGB: (u8, u8, u8) = (10, 13, 13);
pub const SLATE_RGB: (u8, u8, u8) = (53, 60, 67);
pub const SILVER_RGB: (u8, u8, u8) = (201, 205, 212);
pub const SNOW_RGB: (u8, u8, u8) = (247, 248, 250);
pub const GREEN_RGB: (u8, u8, u8) = (74, 222, 128);
pub const YELLOW_RGB: (u8, u8, u8) = (250, 204, 21);

pub const BLUE: Color = Color::Rgb(BLUE_RGB.0, BLUE_RGB.1, BLUE_RGB.2);
pub const RED: Color = Color::Rgb(RED_RGB.0, RED_RGB.1, RED_RGB.2);
pub const ORANGE: Color = Color::Rgb(ORANGE_RGB.0, ORANGE_RGB.1, ORANGE_RGB.2);
pub const MAGENTA: Color = Color::Rgb(MAGENTA_RGB.0, MAGENTA_RGB.1, MAGENTA_RGB.2);
pub const INK: Color = Color::Rgb(INK_RGB.0, INK_RGB.1, INK_RGB.2);
pub const BLACK: Color = Color::Rgb(BLACK_RGB.0, BLACK_RGB.1, BLACK_RGB.2);
pub const SLATE: Color = Color::Rgb(SLATE_RGB.0, SLATE_RGB.1, SLATE_RGB.2);
pub const SILVER: Color = Color::Rgb(SILVER_RGB.0, SILVER_RGB.1, SILVER_RGB.2);
pub const SNOW: Color = Color::Rgb(SNOW_RGB.0, SNOW_RGB.1, SNOW_RGB.2);
pub const GREEN: Color = Color::Rgb(GREEN_RGB.0, GREEN_RGB.1, GREEN_RGB.2);
pub const YELLOW: Color = Color::Rgb(YELLOW_RGB.0, YELLOW_RGB.1, YELLOW_RGB.2);

#[deprecated(note = "Use BLUE_RGB instead")]
pub const AXIOM_BLUE_RGB: (u8, u8, u8) = BLUE_RGB;
#[deprecated(note = "Use RED_RGB instead")]
pub const AXIOM_RED_RGB: (u8, u8, u8) = RED_RGB;
#[deprecated(note = "Use ORANGE_RGB instead")]
pub const AXIOM_ORANGE_RGB: (u8, u8, u8) = ORANGE_RGB;
#[deprecated(note = "Use MAGENTA_RGB instead")]
pub const AXIOM_MAGENTA_RGB: (u8, u8, u8) = MAGENTA_RGB;
#[deprecated(note = "Use INK_RGB instead")]
pub const AXIOM_INK_RGB: (u8, u8, u8) = INK_RGB;
#[deprecated(note = "Use BLACK_RGB instead")]
pub const AXIOM_BLACK_RGB: (u8, u8, u8) = BLACK_RGB;
#[deprecated(note = "Use SLATE_RGB instead")]
pub const AXIOM_SLATE_RGB: (u8, u8, u8) = SLATE_RGB;
#[deprecated(note = "Use SILVER_RGB instead")]
pub const AXIOM_SILVER_RGB: (u8, u8, u8) = SILVER_RGB;
#[deprecated(note = "Use SNOW_RGB instead")]
pub const AXIOM_SNOW_RGB: (u8, u8, u8) = SNOW_RGB;
#[deprecated(note = "Use GREEN_RGB instead")]
pub const AXIOM_GREEN_RGB: (u8, u8, u8) = GREEN_RGB;
#[deprecated(note = "Use YELLOW_RGB instead")]
pub const AXIOM_YELLOW_RGB: (u8, u8, u8) = YELLOW_RGB;

#[deprecated(note = "Use BLUE instead")]
pub const AXIOM_BLUE: Color = BLUE;
#[deprecated(note = "Use RED instead")]
pub const AXIOM_RED: Color = RED;
#[deprecated(note = "Use ORANGE instead")]
pub const AXIOM_ORANGE: Color = ORANGE;
#[deprecated(note = "Use MAGENTA instead")]
pub const AXIOM_MAGENTA: Color = MAGENTA;
#[deprecated(note = "Use INK instead")]
pub const AXIOM_INK: Color = INK;
#[deprecated(note = "Use BLACK instead")]
pub const AXIOM_BLACK: Color = BLACK;
#[deprecated(note = "Use SLATE instead")]
pub const AXIOM_SLATE: Color = SLATE;
#[deprecated(note = "Use SILVER instead")]
pub const AXIOM_SILVER: Color = SILVER;
#[deprecated(note = "Use SNOW instead")]
pub const AXIOM_SNOW: Color = SNOW;
#[deprecated(note = "Use GREEN instead")]
pub const AXIOM_GREEN: Color = GREEN;
#[deprecated(note = "Use YELLOW instead")]
pub const AXIOM_YELLOW: Color = YELLOW;

pub const TEXT_PRIMARY: Color = SNOW;
pub const TEXT_MUTED: Color = SILVER;
pub const TEXT_DIM: Color = SLATE;

pub const STATUS_SUCCESS: Color = GREEN;
pub const STATUS_WARNING: Color = ORANGE;
pub const STATUS_ERROR: Color = RED;
pub const STATUS_INFO: Color = BLUE;

pub const SELECTION_BG: Color = SLATE;
pub const COMPOSER_BG: Color = INK;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTheme {
    pub name: &'static str,
    pub composer_bg: Color,
    pub selection_bg: Color,
    pub header_bg: Color,
}

pub fn ui_theme(name: &str) -> UiTheme {
    match name.to_ascii_lowercase().as_str() {
        "dark" => UiTheme {
            name: "dark",
            composer_bg: BLACK,
            selection_bg: INK,
            header_bg: BLACK,
        },
        "light" => UiTheme {
            name: "light",
            composer_bg: SLATE,
            selection_bg: SILVER,
            header_bg: SLATE,
        },
        _ => UiTheme {
            name: "default",
            composer_bg: COMPOSER_BG,
            selection_bg: SELECTION_BG,
            header_bg: BLACK,
        },
    }
}
