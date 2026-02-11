//! Shell execution tests.

#[cfg(test)]
mod shell_tests {
    use std::process::Command;

    #[test]
    fn test_echo_command() -> Result<(), std::io::Error> {
        let output = Command::new("echo").arg("Hello, Shell!").output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello, Shell!"));

        Ok(())
    }

    #[test]
    fn test_pwd_command() -> Result<(), std::io::Error> {
        let output = Command::new("pwd").output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let current_dir = std::env::current_dir()?;

        // pwd should return current directory
        assert!(stdout.contains(current_dir.to_string_lossy().as_ref()));

        Ok(())
    }

    #[test]
    fn test_failed_command() {
        let output = Command::new("sh").arg("-c").arg("exit 1").output();

        assert!(output.is_ok());
        assert!(!output.unwrap().status.success());
    }
}
