//! Memory tool tests.

#[cfg(test)]
mod memory_tests {
    use std::fs;

    #[test]
    fn test_memory_file_operations() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-memory-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let memory_path = dir.join("memory.md");

        let content = "# Test Memory\n\nThis is a test memory file.";
        fs::write(&memory_path, content)?;

        let read = fs::read_to_string(&memory_path)?;
        assert!(read.contains("Test Memory"));
        assert!(read.contains("This is a test memory file."));

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_memory_create_parent_dirs() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-memory-parents-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let memory_path = dir.join("subdir").join("memory.md");

        if let Some(parent) = memory_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&memory_path, "Test content")?;

        assert!(memory_path.exists());

        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
