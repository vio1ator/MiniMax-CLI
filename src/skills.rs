use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub fn list(skills_dir: PathBuf) -> Result<()> {
    if !skills_dir.exists() {
        println!("No skills directory found at {}", skills_dir.display());
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&skills_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            entries.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    if entries.is_empty() {
        println!("No skills found in {}", skills_dir.display());
        return Ok(());
    }

    entries.sort();
    for entry in entries {
        println!("{}", entry);
    }
    Ok(())
}

pub fn show(skills_dir: PathBuf, name: &str) -> Result<()> {
    let path = skills_dir.join(name).join("SKILL.md");
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    println!("{}", contents);
    Ok(())
}
