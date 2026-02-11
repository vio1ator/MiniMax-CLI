//! Palette audit tests to prevent color drift.
//!
//! These tests ensure that ANSI color shorthands do not sneak back into
//! user-visible output. Use the Axiom palette constants instead.

use std::fs;
use std::path::Path;

const DISALLOWED_COLORIZE: &[&str] = &[
    ".red()",
    ".green()",
    ".yellow()",
    ".blue()",
    ".cyan()",
    ".magenta()",
];

const DISALLOWED_COLOR_ENUMS: &[&str] = &[
    "Color::Red",
    "Color::Green",
    "Color::Yellow",
    "Color::Blue",
    "Color::Cyan",
    "Color::Magenta",
];

fn audit_file(path: &Path, violations: &mut Vec<String>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (line_num, line) in content.lines().enumerate() {
        for pattern in DISALLOWED_COLORIZE
            .iter()
            .chain(DISALLOWED_COLOR_ENUMS.iter())
        {
            if line.contains(pattern) {
                violations.push(format!(
                    "{}:{}: direct color usage ({pattern})",
                    path.display(),
                    line_num + 1
                ));
            }
        }
    }
}

fn audit_directory(dir: &Path, violations: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            audit_directory(&path, violations);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if path.file_name().is_some_and(|n| n == "palette.rs") {
                continue;
            }
            audit_file(&path, violations);
        }
    }
}

#[test]
fn audit_no_direct_color_usage() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = Path::new(manifest_dir).join("src");
    let mut violations = Vec::new();

    audit_directory(&src_dir, &mut violations);

    if !violations.is_empty() {
        let report = violations.join("\n");
        panic!(
            "Palette audit failed! Found {} direct color uses:\n{}",
            violations.len(),
            report
        );
    }
}

#[test]
fn verify_brand_colors_defined() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let palette_path = Path::new(manifest_dir).join("src/palette.rs");
    let content = fs::read_to_string(&palette_path).expect("Failed to read palette.rs");

    assert!(
        content.contains("BLUE_RGB: (u8, u8, u8) = (20, 86, 240)"),
        "BLUE should be #1456F0"
    );
    assert!(
        content.contains("RED_RGB: (u8, u8, u8) = (242, 63, 93)"),
        "RED should be #F23F5D"
    );
    assert!(
        content.contains("ORANGE_RGB: (u8, u8, u8) = (255, 99, 58)"),
        "ORANGE should be #FF633A"
    );
    assert!(
        content.contains("MAGENTA_RGB: (u8, u8, u8) = (228, 23, 127)"),
        "MAGENTA should be #E4177F"
    );
    assert!(
        content.contains("GREEN_RGB: (u8, u8, u8) = (74, 222, 128)"),
        "GREEN should be #4ADE80"
    );
}

#[test]
fn verify_semantic_tokens() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let palette_path = Path::new(manifest_dir).join("src/palette.rs");
    let content = fs::read_to_string(&palette_path).expect("Failed to read palette.rs");

    assert!(
        content.contains("pub const TEXT_PRIMARY: Color = SNOW;"),
        "TEXT_PRIMARY should use SNOW"
    );
    assert!(
        content.contains("pub const STATUS_SUCCESS: Color = GREEN;"),
        "STATUS_SUCCESS should use GREEN"
    );
    assert!(
        content.contains("pub const STATUS_WARNING: Color = ORANGE;"),
        "STATUS_WARNING should use ORANGE"
    );
    assert!(
        content.contains("pub const STATUS_ERROR: Color = RED;"),
        "STATUS_ERROR should use RED"
    );
    assert!(
        content.contains("pub const STATUS_INFO: Color = BLUE;"),
        "STATUS_INFO should use BLUE"
    );
}
