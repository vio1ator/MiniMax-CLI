//! Memory tools: save and retrieve user-related facts.
//!
//! These tools allow the model to remember specific information about the user,
//! their preferences, or important project details across sessions.

use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, required_str,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

/// Tool for saving a fact to long-term memory.
pub struct SaveMemoryTool {
    memory_path: PathBuf,
}

impl SaveMemoryTool {
    #[must_use]
    pub fn new(memory_path: PathBuf) -> Self {
        Self { memory_path }
    }

    fn ensure_memory_dir(&self) -> Result<(), ToolError> {
        if let Some(parent) = self.memory_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ToolError::execution_failed(format!("Failed to create memory directory: {e}"))
            })?;
        }
        Ok(())
    }

    fn load_memory(&self) -> Result<Vec<String>, ToolError> {
        if !self.memory_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.memory_path)
            .map_err(|e| ToolError::execution_failed(format!("Failed to read memory file: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| ToolError::execution_failed(format!("Failed to parse memory file: {e}")))
    }

    fn save_memory(&self, facts: &[String]) -> Result<(), ToolError> {
        let content = serde_json::to_string_pretty(facts)
            .map_err(|e| ToolError::execution_failed(format!("Failed to serialize memory: {e}")))?;
        fs::write(&self.memory_path, content)
            .map_err(|e| ToolError::execution_failed(format!("Failed to write memory file: {e}")))
    }
}

#[async_trait]
impl ToolSpec for SaveMemoryTool {
    fn name(&self) -> &'static str {
        "save_memory"
    }

    fn description(&self) -> &'static str {
        "Saves a specific piece of information or fact to your long-term memory. Use this for user preferences, important project paths, or other facts that should persist across sessions."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "fact": {
                    "type": "string",
                    "description": "The specific fact or piece of information to remember."
                }
            },
            "required": ["fact"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::RequiresApproval]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Suggest
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let fact = required_str(&input, "fact")?;
        self.ensure_memory_dir()?;
        let mut facts = self.load_memory()?;

        // Avoid duplicates
        if !facts.contains(&fact.to_string()) {
            facts.push(fact.to_string());
            self.save_memory(&facts)?;
            Ok(ToolResult::success(format!("Fact remembered: {fact}")))
        } else {
            Ok(ToolResult::success("I already remember that fact."))
        }
    }
}

/// Tool for retrieving facts from long-term memory.
pub struct GetMemoryTool {
    memory_path: PathBuf,
}

impl GetMemoryTool {
    #[must_use]
    pub fn new(memory_path: PathBuf) -> Self {
        Self { memory_path }
    }
}

#[async_trait]
impl ToolSpec for GetMemoryTool {
    fn name(&self) -> &'static str {
        "get_memory"
    }

    fn description(&self) -> &'static str {
        "Retrieve all saved facts from your long-term memory."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    async fn execute(
        &self,
        _input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        if !self.memory_path.exists() {
            return Ok(ToolResult::success("No facts remembered yet."));
        }
        let content = fs::read_to_string(&self.memory_path)
            .map_err(|e| ToolError::execution_failed(format!("Failed to read memory file: {e}")))?;
        let facts: Vec<String> = serde_json::from_str(&content).map_err(|e| {
            ToolError::execution_failed(format!("Failed to parse memory file: {e}"))
        })?;

        if facts.is_empty() {
            Ok(ToolResult::success("No facts remembered yet."))
        } else {
            let output = facts
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{}. {}\n", i + 1, f))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(ToolResult::success(format!("Remembered facts:\n{output}")))
        }
    }
}
