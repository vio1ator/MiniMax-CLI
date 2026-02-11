//! Test utilities for Axiom CLI tests.

use std::fs;
use std::path::{Path, PathBuf};

/// Create a temporary directory for testing
pub fn temp_dir() -> Result<PathBuf, std::io::Error> {
    let temp = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = temp.join(format!("axiom-test-{}-{}", std::process::id(), nanos));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Create a minimal config file for testing
pub fn create_test_config(dir: &Path, api_key: Option<&str>) -> Result<PathBuf, std::io::Error> {
    let config_path = dir.join(".axiom").join("config.toml");
    fs::create_dir_all(config_path.parent().unwrap())?;

    let content = match api_key {
        Some(key) => format!(
            r#"api_key = "{key}"

default_model = "anthropic/claude-3-5-sonnet-20241022"
"#
        ),
        None => r#"default_model = "anthropic/claude-3-5-sonnet-20241022"
"#
        .to_string(),
    };

    fs::write(&config_path, content)?;
    Ok(config_path)
}

/// Create a test workspace directory
pub fn create_workspace(dir: &Path) -> Result<PathBuf, std::io::Error> {
    let workspace = dir.join("workspace");
    fs::create_dir_all(&workspace)?;
    Ok(workspace)
}
