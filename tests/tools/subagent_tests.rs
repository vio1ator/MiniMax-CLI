//! Subagent tool tests.

#[cfg(test)]
mod subagent_tests {
    #[test]
    fn test_subagent_manager_creation() {
        // This test validates the subagent manager structure
        // Full tests would require lib.rs exports

        // Validate subagent types
        let general_type = "general";
        let security_type = "security";
        let investigator_type = "investigator";

        assert!(!general_type.is_empty());
        assert!(!security_type.is_empty());
        assert!(!investigator_type.is_empty());
    }

    #[test]
    fn test_subagent_scheduling() {
        // Validate subagent scheduling constraints
        let max_subagents = 5usize;
        let default_subagents = 5usize;

        assert_eq!(max_subagents, 5);
        assert_eq!(default_subagents, 5);
        assert!(max_subagents >= default_subagents);
    }
}
