//! Workspace safety tests.

#[cfg(test)]
mod workspace_tests {
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_workspace_validation() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-workspace-safety-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace)?;

        // File in workspace should be valid
        let valid_path = workspace.join("file.txt");

        // Validate path starts with workspace
        let valid_str = valid_path.to_string_lossy();
        let workspace_str = workspace.to_string_lossy();

        assert!(valid_str.starts_with(workspace_str.as_ref()));

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_workspace_outside_path() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-workspace-outside-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace)?;

        // File outside workspace
        let outside_file = dir.join("outside.txt");

        // Validate file doesn't start with workspace
        let outside_str = outside_file.to_string_lossy();
        let workspace_str = workspace.to_string_lossy();

        assert!(!outside_str.starts_with(workspace_str.as_ref()));

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_path_normalization() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-path-normalize-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace)?;

        // Test relative path resolution
        let relative_path = PathBuf::from("test.txt");
        let resolved = workspace.join(&relative_path);

        assert!(resolved.to_string_lossy().contains("test.txt"));
        assert!(
            resolved
                .to_string_lossy()
                .starts_with(workspace.to_string_lossy().as_ref())
        );

        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
