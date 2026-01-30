//! Security Analysis tool: autonomous security auditing.
//!
//! Spawns a specialized sub-agent to perform security reviews of the codebase,
//! identifying potential vulnerabilities and recommending remediations.

use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, optional_str,
};
use super::subagent::{SharedSubAgentManager, SubAgentRuntime, SubAgentType};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for security analysis.
pub struct SecurityAnalyzeTool {
    manager: SharedSubAgentManager,
    runtime: SubAgentRuntime,
}

impl SecurityAnalyzeTool {
    #[must_use]
    pub fn new(manager: SharedSubAgentManager, runtime: SubAgentRuntime) -> Self {
        Self { manager, runtime }
    }
}

#[async_trait]
impl ToolSpec for SecurityAnalyzeTool {
    fn name(&self) -> &'static str {
        "security_analyze"
    }

    fn description(&self) -> &'static str {
        "Perform a security audit of the codebase or a specific file. Spawns an autonomous security auditor agent."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Specific file or directory to audit (default: .)"
                },
                "focus": {
                    "type": "string",
                    "description": "Specific security focus (e.g., 'injection', 'auth', 'secrets', 'all')"
                }
            },
            "required": []
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly, ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let target = optional_str(&input, "target").unwrap_or(".");
        let focus = optional_str(&input, "focus").unwrap_or("all");

        let prompt = format!(
            "Your task is to perform a security audit of target: {}. Focus on: {}.\n\n\
            1. Scan for hardcoded secrets, API keys, and credentials.\n\
            2. Check for common injection vulnerabilities (SQLi, Command Injection, XSS).\n\
            3. Analyze access control and authentication logic.\n\
            4. Identify insecure data handling or weak cryptography.\n\
            5. Provide a detailed report with findings, severity levels, and remediation steps.",
            target, focus
        );

        let mut manager = self
            .manager
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock sub-agent manager"))?;

        let result = manager
            .spawn_background(
                Arc::clone(&self.manager),
                self.runtime.clone(),
                SubAgentType::Review,
                prompt,
                None,
            )
            .map_err(|e| {
                ToolError::execution_failed(format!("Failed to spawn security auditor: {e}"))
            })?;

        Ok(ToolResult::json(&result).map_err(|e| ToolError::execution_failed(e.to_string()))?)
    }
}
