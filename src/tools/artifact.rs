//! Artifact tools: create and list persistent outputs.
//!
//! Artifacts are significant outputs produced by the agent (e.g., reports,
//! diagrams, code snippets) that should be easily accessible and listed.

use super::spec::{ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, required_str};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

/// Tool for creating an artifact.
pub struct ArtifactCreateTool;

#[async_trait]
impl ToolSpec for ArtifactCreateTool {
    fn name(&self) -> &'static str {
        "artifact_create"
    }

    fn description(&self) -> &'static str {
        "Create a persistent artifact (e.g., a report, document, or code block). The artifact will be saved in the .artifacts directory of the workspace."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Title of the artifact"
                },
                "content": {
                    "type": "string",
                    "description": "Content of the artifact"
                },
                "type": {
                    "type": "string",
                    "description": "Type of artifact (e.g., doc, code, report)"
                }
            },
            "required": ["title", "content"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::WritesFiles]
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let title = required_str(&input, "title")?;
        let content = required_str(&input, "content")?;
        let kind = input.get("type").and_then(|v| v.as_str()).unwrap_or("doc");

        let artifact_dir = context.workspace.join(".artifacts");
        fs::create_dir_all(&artifact_dir).map_err(|e| {
            ToolError::execution_failed(format!("Failed to create artifacts directory: {e}"))
        })?;

        // Sanitize title for filename
        let safe_title = title
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let filename = format!("{}_{}.md", safe_title, kind);
        let path = artifact_dir.join(&filename);

        fs::write(&path, content)
            .map_err(|e| ToolError::execution_failed(format!("Failed to write artifact: {e}")))?;

        Ok(ToolResult::success(format!(
            "Artifact '{}' created at .artifacts/{}",
            title, filename
        )))
    }
}

/// Tool for listing artifacts.
pub struct ArtifactListTool;

#[async_trait]
impl ToolSpec for ArtifactListTool {
    fn name(&self) -> &'static str {
        "artifact_list"
    }

    fn description(&self) -> &'static str {
        "List all artifacts in the .artifacts directory."
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

    fn supports_parallel(&self) -> bool {
        true
    }

    async fn execute(&self, _input: Value, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let artifact_dir = context.workspace.join(".artifacts");
        if !artifact_dir.exists() {
            return Ok(ToolResult::success("No artifacts found."));
        }

        let entries = fs::read_dir(&artifact_dir).map_err(|e| {
            ToolError::execution_failed(format!("Failed to read artifacts directory: {e}"))
        })?;

        let mut artifacts = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ToolError::execution_failed(e.to_string()))?;
            let path = entry.path();
            if path.is_file() {
                artifacts.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }

        if artifacts.is_empty() {
            return Ok(ToolResult::success("No artifacts found."));
        }

        let output = artifacts.join("\n");
        Ok(ToolResult::success(format!("Artifacts:\n{}", output)))
    }
}
