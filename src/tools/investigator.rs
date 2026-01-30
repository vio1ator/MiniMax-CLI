//! Codebase Investigator tool: autonomous codebase analysis.
//!
//! Spawns a specialized sub-agent to explore and report on the codebase structure,
//! patterns, and architecture.

use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, required_str,
};
use super::subagent::{SharedSubAgentManager, SubAgentRuntime, SubAgentType};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for investigating the codebase.
pub struct CodebaseInvestigatorTool {
    manager: SharedSubAgentManager,
    runtime: SubAgentRuntime,
}

impl CodebaseInvestigatorTool {
    #[must_use]
    pub fn new(manager: SharedSubAgentManager, runtime: SubAgentRuntime) -> Self {
        Self { manager, runtime }
    }
}

#[async_trait]
impl ToolSpec for CodebaseInvestigatorTool {
    fn name(&self) -> &'static str {
        "codebase_investigator"
    }

    fn description(&self) -> &'static str {
        "Analyze the codebase to understand its structure, architecture, and dependencies. Spawns an autonomous investigator agent."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "objective": {
                    "type": "string",
                    "description": "Specific goal for the investigation (e.g., 'How is tool registration handled?', 'Map the project structure')"
                }
            },
            "required": ["objective"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let objective = required_str(&input, "objective")?;

        let prompt = format!(
            "Your task is to investigate the codebase with the following objective: {}\n\n\
            1. Start by listing the root directory.\n\
            2. Explore relevant directories and read key files.\n\
            3. Identify core components, architectural patterns, and dependencies.\n\
            4. Provide a structured report summarizing your findings.",
            objective
        );

        let mut manager = self
            .manager
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock sub-agent manager"))?;

        let result = manager
            .spawn_background(
                Arc::clone(&self.manager),
                self.runtime.clone(),
                SubAgentType::Explore,
                prompt,
                None,
            )
            .map_err(|e| {
                ToolError::execution_failed(format!("Failed to spawn investigator: {e}"))
            })?;

        Ok(ToolResult::json(&result).map_err(|e| ToolError::execution_failed(e.to_string()))?)
    }
}
