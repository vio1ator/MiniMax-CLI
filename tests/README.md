# Integration Tests

This directory contains integration tests for the Axiom CLI project.

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test config_tests
cargo test --test engine_tests
cargo test --test features_tests
cargo test --test workspace_tests

# Run all integration tests
cargo test --test '*'
```

## Test Structure

### `tests/common.rs`
Shared test utilities including:
- `temp_dir()` - Create temporary test directories
- `create_test_config()` - Create minimal config files
- `create_workspace()` - Create test workspace directories

### `tests/config_tests.rs`
Basic configuration file operations:
- Test temp directory creation
- Test config file creation with/without API key
- Test workspace creation

### `tests/engine_tests.rs`
Engine configuration structure validation:
- Default config values
- Custom config values

### `tests/features_tests.rs`
Feature flag validation:
- Feature flag existence
- Feature naming conventions

### `tests/tools/file_tests.rs`
File operations testing:
- Read/write file operations
- Handle missing files

### `tests/tools/shell_tests.rs`
Shell command execution:
- Execute echo/pwd commands
- Handle command failures

### `tests/tools/git_tests.rs`
Git command availability:
- Git version detection
- Git command execution

### `tests/tools/memory_tests.rs`
Memory file operations:
- Create and read memory files
- Handle parent directory creation

### `tests/tools/subagent_tests.rs`
Subagent structure validation:
- Subagent type definitions
- Scheduling constraints

### `tests/web_search_tests.rs` (moved from `tests/tools/`)
Web search tool validation:
- Web search tool structure
- Search parameter structure

### `tests/workspace_tests.rs`
Workspace safety validation:
- Path validation within workspace
- Path validation outside workspace
- Path normalization

## Adding New Tests

1. Create a new file in `tests/` or `tests/tools/`
2. Use `#[cfg(test)]` module for tests
3. Import utilities from `common.rs` when needed
4. Use descriptive test names
5. Clean up temporary files after tests

Example:
```rust
#[cfg(test)]
mod my_new_tests {
    use std::fs;

    #[test]
    fn test_something() -> Result<(), std::io::Error> {
        // Create temp dir
        let temp = std::env::temp_dir();
        let dir = temp.join("my-test");
        fs::create_dir_all(&dir)?;
        
        // Your test code here
        
        fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
```

## CI Integration

Tests are run on CI via GitHub Actions. Coverage reporting is also configured:
- Run with: `cargo test --all-features`
- Coverage: `cargo llvm-cov --all-features --html --output-dir target/coverage`
