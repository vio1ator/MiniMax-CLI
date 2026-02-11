//! File tool tests.

#[cfg(test)]
mod file_tests {
    use std::fs;

    #[test]
    fn test_file_read_write() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!("axiom-file-test-{}-{}", std::process::id(), nanos));
        fs::create_dir_all(&dir)?;

        let test_file = dir.join("test.txt");
        fs::write(&test_file, "Hello, World!")?;

        let content = fs::read_to_string(&test_file)?;
        assert!(content.contains("Hello, World!"));

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_file_not_found() -> Result<(), std::io::Error> {
        let temp = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = temp.join(format!(
            "axiom-file-not-found-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir)?;

        let nonexistent = dir.join("nonexistent.txt");
        let result = fs::read_to_string(&nonexistent);

        assert!(result.is_err());

        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
