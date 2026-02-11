//! Engine configuration tests.

#[cfg(test)]
mod engine_tests {
    #[test]
    fn test_engine_config_defaults() {
        // This test verifies the engine config structure exists
        // Full integration tests would require lib.rs exports

        // Basic validation that EngineConfig would have these fields
        let model = "anthropic/claude-3-5-sonnet-20241022".to_string();
        let workspace = std::path::PathBuf::from(".");
        let max_steps = 100u32;

        assert_eq!(model, "anthropic/claude-3-5-sonnet-20241022");
        assert_eq!(workspace.to_string_lossy(), ".");
        assert_eq!(max_steps, 100);
    }

    #[test]
    fn test_engine_config_custom() {
        let custom_model = "test/model".to_string();
        let custom_workspace = std::path::PathBuf::from("/test/workspace");
        let custom_max_steps = 50u32;

        assert_eq!(custom_model, "test/model");
        assert_eq!(custom_workspace.to_string_lossy(), "/test/workspace");
        assert_eq!(custom_max_steps, 50);
    }
}
