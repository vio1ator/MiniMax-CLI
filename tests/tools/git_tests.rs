//! Git tool tests.

#[cfg(test)]
mod git_tests {
    use std::process::Command;

    fn is_git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_git_detection() {
        if !is_git_available() {
            println!("git not available, skipping test");
            return;
        }

        let output = Command::new("git").arg("--version").output();

        assert!(output.is_ok());
    }

    #[test]
    fn test_git_command() {
        if !is_git_available() {
            println!("git not available, skipping test");
            return;
        }

        let output = Command::new("git").arg("version").output();

        assert!(output.is_ok());
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("git version"));
    }
}
