//! Think tool — transparent reasoning pass-through.
//!
//! Allows the model to reason step-by-step without side effects.
//! Inspired by gotui's think tool pattern.

use async_trait::async_trait;
use serde_json::{Value, json};

use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, required_str,
};

/// A tool that lets the model think through complex problems step-by-step.
pub struct ThinkTool;

#[async_trait]
impl ToolSpec for ThinkTool {
    fn name(&self) -> &str {
        "think"
    }

    fn description(&self) -> &str {
        "Use this tool to think through a complex problem step by step. \
         Your reasoning will be shown to the user but has no side effects. \
         Use it before making decisions, planning multi-step tasks, or analyzing trade-offs."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your thought process, reasoning, or analysis"
                }
            },
            "required": ["thought"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let thought = required_str(&input, "thought")?;
        if thought.is_empty() {
            return Err(ToolError::invalid_input("thought cannot be empty"));
        }
        Ok(ToolResult::success(format!("🤔 {thought}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_think_tool_basic() {
        let tool = ThinkTool;
        let ctx = ToolContext::new("/tmp");
        let result = tool
            .execute(json!({"thought": "Let me consider the options..."}), &ctx)
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.content.contains("Let me consider the options..."));
    }

    #[tokio::test]
    async fn test_think_tool_empty() {
        let tool = ThinkTool;
        let ctx = ToolContext::new("/tmp");
        let result = tool.execute(json!({"thought": ""}), &ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_think_tool_missing_field() {
        let tool = ThinkTool;
        let ctx = ToolContext::new("/tmp");
        let result = tool.execute(json!({}), &ctx).await;
        assert!(result.is_err());
    }
}
