//! Simple test examples for Phase 8 implementation.

#[cfg(test)]
mod simple_tests {
    use std::fs;

    #[test]
    fn test_temp_dir_creation() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-simple-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        assert!(dir.exists());
        assert!(dir.is_dir());

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_config_file_creation() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-config-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let config_path = dir.join(".axiom").join("config.toml");
        fs::create_dir_all(config_path.parent().unwrap())?;

        let content = r#"api_key = "test-key-123"

default_model = "anthropic/claude-3-5-sonnet-20241022"
"#;
        fs::write(&config_path, content)?;

        let read = fs::read_to_string(&config_path)?;
        assert!(read.contains("test-key-123"));
        assert!(read.contains("anthropic/claude-3-5-sonnet-20241022"));

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_workspace_creation() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-workspace-test-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace)?;

        assert!(workspace.exists());
        assert!(workspace.is_dir());

        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
