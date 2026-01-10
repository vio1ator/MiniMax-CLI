use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn show(path: PathBuf) -> Result<()> {
    if !path.exists() {
        println!("No memory file found at {}", path.display());
        return Ok(());
    }
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    println!("{}", contents);
    Ok(())
}

pub fn add(path: PathBuf, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(format!("{}\n", content).as_bytes())?;
    println!("Memory updated at {}", path.display());
    Ok(())
}

pub fn clear(path: PathBuf) -> Result<()> {
    if path.exists() {
        fs::write(&path, "")?;
    }
    println!("Memory cleared at {}", path.display());
    Ok(())
}
