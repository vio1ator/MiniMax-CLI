//! Execution tools: run code snippets in a (semi) sandboxed environment.
//!
//! Currently provides `exec_python` for running Python code.

use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec, required_str,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

/// Tool for executing Python code.
pub struct ExecPythonTool;

#[async_trait]
impl ToolSpec for ExecPythonTool {
    fn name(&self) -> &'static str {
        "exec_python"
    }

    fn description(&self) -> &'static str {
        "Execute Python code and return the output. Useful for complex calculations, data processing, or scripting."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The Python code to execute"
                }
            },
            "required": ["code"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::ExecutesCode,
            ToolCapability::RequiresApproval,
        ]
    }

    fn supports_parallel(&self) -> bool {
        true
    }

    async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let code = required_str(&input, "code")?;

        let mut child = Command::new("python3")
            .arg("-c")
            .arg(code)
            .current_dir(&context.workspace)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::execution_failed(format!("Failed to spawn python3: {e}")))?;

        let timeout = Duration::from_secs(30);
        let status = match child.wait_timeout(timeout) {
            Ok(Some(status)) => status,
            Ok(None) => {
                child.kill().ok();
                return Ok(ToolResult::error(
                    "Python execution timed out after 30 seconds",
                ));
            }
            Err(e) => return Err(ToolError::execution_failed(format!("Wait failed: {e}"))),
        };

        let output = child.wait_with_output().map_err(|e| {
            ToolError::execution_failed(format!("Failed to read python output: {e}"))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if status.success() {
            Ok(ToolResult::success(stdout.to_string()))
        } else {
            Ok(ToolResult::error(format!(
                "Python execution failed with exit code {}:\nSTDOUT:\n{}
STDERR:\n{}",
                status.code().unwrap_or(-1),
                stdout,
                stderr
            )))
        }
    }
}
