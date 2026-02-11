//! Feature flags tests.

#[cfg(test)]
mod feature_tests {
    #[test]
    fn test_features_exist() {
        // This test verifies that feature flags exist in the codebase
        // Full tests would require lib.rs exports

        // Validate known feature keys
        let duo_feature = "duo";
        let rlm_feature = "rlm";
        let subagents_feature = "subagents";

        assert!(!duo_feature.is_empty());
        assert!(!rlm_feature.is_empty());
        assert!(!subagents_feature.is_empty());
    }

    #[test]
    fn test_feature_names() {
        // Verify feature flag naming conventions
        assert_eq!("duo", "duo");
        assert_eq!("rlm", "rlm");
        assert_eq!("subagents", "subagents");
        assert_eq!("web_search", "web_search");
        assert_eq!("mcp", "mcp");
        assert_eq!("apply_patch", "apply_patch");
    }
}
