//! Tools for Duo mode: Player-Coach autocoding workflow.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::duo::{
    DuoPhase, SharedDuoSession, generate_coach_prompt, generate_player_prompt, session_summary,
};
use crate::tools::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec,
    optional_str, required_str,
};

/// Initialize an autocoding session with requirements.
pub struct DuoInitTool {
    session: SharedDuoSession,
}

impl DuoInitTool {
    #[must_use]
    pub fn new(session: SharedDuoSession) -> Self {
        Self { session }
    }
}

#[async_trait]
impl ToolSpec for DuoInitTool {
    fn name(&self) -> &'static str {
        "duo_init"
    }

    fn description(&self) -> &'static str {
        "Initialize a Duo autocoding session with requirements. Returns session summary."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "requirements": {
                    "type": "string",
                    "description": "The requirements document (source of truth). Should be structured as a checklist."
                },
                "max_turns": {
                    "type": "integer",
                    "description": "Maximum turns before timeout (default: 10)"
                },
                "session_name": {
                    "type": "string",
                    "description": "Optional human-readable session name (e.g., 'auth-feature')"
                },
                "approval_threshold": {
                    "type": "number",
                    "description": "Minimum compliance score for approval (0-1, default: 0.9)"
                }
            },
            "required": ["requirements"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let requirements = required_str(&input, "requirements")?;
        let max_turns = input
            .get("max_turns")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let session_name = optional_str(&input, "session_name").map(str::to_string);
        let approval_threshold = input.get("approval_threshold").and_then(|v| v.as_f64());

        let mut session = self
            .session
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock Duo session"))?;

        let state = session.start_session(
            requirements.to_string(),
            session_name,
            max_turns,
            approval_threshold,
        );

        let summary = state.summary();
        Ok(ToolResult::success(format!(
            "Duo session initialized. Ready for player phase.\n\n{}",
            summary
        )))
    }
}

/// Generate the player prompt for implementation.
pub struct DuoPlayerTool {
    session: SharedDuoSession,
}

impl DuoPlayerTool {
    #[must_use]
    pub fn new(session: SharedDuoSession) -> Self {
        Self { session }
    }
}

#[async_trait]
impl ToolSpec for DuoPlayerTool {
    fn name(&self) -> &'static str {
        "duo_player"
    }

    fn description(&self) -> &'static str {
        "Generate the player prompt for implementation. Must be in Init or Player phase. Call after implementing to advance to Coach phase."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "implementation_summary": {
                    "type": "string",
                    "description": "Optional summary of implementation work done (recorded in history)"
                }
            }
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let implementation_summary = optional_str(&input, "implementation_summary")
            .map(str::to_string)
            .unwrap_or_else(|| "Implementation in progress".to_string());

        let mut session = self
            .session
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock Duo session"))?;

        let state = session
            .get_active_mut()
            .ok_or_else(|| ToolError::invalid_input("No active session. Call duo_init first."))?;

        // Check we're in a valid phase for player
        match state.phase {
            DuoPhase::Init | DuoPhase::Player => {
                // Generate prompt first
                let prompt = generate_player_prompt(state);

                // Advance to Coach phase
                state
                    .advance_to_coach(implementation_summary)
                    .map_err(|e| ToolError::execution_failed(e.to_string()))?;

                Ok(ToolResult::success(format!(
                    "=== PLAYER PROMPT ===\n\n{}\n\n---\nAdvanced to Coach phase. Use duo_coach for verification.",
                    prompt
                )))
            }
            DuoPhase::Coach => Err(ToolError::invalid_input(
                "Already in Coach phase. Use duo_coach to get verification prompt.",
            )),
            DuoPhase::Approved => Err(ToolError::invalid_input(
                "Session already approved. Start a new session with duo_init.",
            )),
            DuoPhase::Timeout => Err(ToolError::invalid_input(
                "Session timed out. Start a new session with duo_init.",
            )),
        }
    }
}

/// Generate the coach prompt for validation.
pub struct DuoCoachTool {
    session: SharedDuoSession,
}

impl DuoCoachTool {
    #[must_use]
    pub fn new(session: SharedDuoSession) -> Self {
        Self { session }
    }
}

#[async_trait]
impl ToolSpec for DuoCoachTool {
    fn name(&self) -> &'static str {
        "duo_coach"
    }

    fn description(&self) -> &'static str {
        "Generate the coach prompt for validation. Must be in Coach phase. Does NOT advance state."
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

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(
        &self,
        _input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let session = self
            .session
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock Duo session"))?;

        let state = session
            .get_active()
            .ok_or_else(|| ToolError::invalid_input("No active session. Call duo_init first."))?;

        if state.phase != DuoPhase::Coach {
            return Err(ToolError::invalid_input(format!(
                "Expected Coach phase, but current phase is {}. Use duo_player first.",
                state.phase
            )));
        }

        let prompt = generate_coach_prompt(state);

        Ok(ToolResult::success(format!(
            "=== COACH PROMPT ===\n\n{}\n\n---\nAfter verification, use duo_advance with feedback and approval status.",
            prompt
        )))
    }
}

/// Advance the session after coach review.
pub struct DuoAdvanceTool {
    session: SharedDuoSession,
}

impl DuoAdvanceTool {
    #[must_use]
    pub fn new(session: SharedDuoSession) -> Self {
        Self { session }
    }
}

#[async_trait]
impl ToolSpec for DuoAdvanceTool {
    fn name(&self) -> &'static str {
        "duo_advance"
    }

    fn description(&self) -> &'static str {
        "Advance the session after coach review. Updates turn count and records feedback. Returns new status."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "feedback": {
                    "type": "string",
                    "description": "The coach's feedback text (compliance checklist and actions needed)"
                },
                "approved": {
                    "type": "boolean",
                    "description": "Whether the coach approved the implementation (look for 'COACH APPROVED')"
                },
                "compliance_score": {
                    "type": "number",
                    "description": "Optional compliance score (0-1) based on checklist items satisfied"
                }
            },
            "required": ["feedback", "approved"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::ReadOnly]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let feedback = required_str(&input, "feedback")?;
        let approved = input
            .get("approved")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| ToolError::missing_field("approved"))?;
        let compliance_score = input.get("compliance_score").and_then(|v| v.as_f64());

        let mut session = self
            .session
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock Duo session"))?;

        let state = session
            .get_active_mut()
            .ok_or_else(|| ToolError::invalid_input("No active session. Call duo_init first."))?;

        if state.phase != DuoPhase::Coach {
            return Err(ToolError::invalid_input(format!(
                "Expected Coach phase, but current phase is {}",
                state.phase
            )));
        }

        // Advance the turn
        state
            .advance_turn(feedback.to_string(), approved, compliance_score)
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;

        // Determine status message based on new phase
        let status_msg = match state.phase {
            DuoPhase::Approved => "🎉 APPROVED! All requirements verified.",
            DuoPhase::Timeout => "⏰ TIMEOUT. Max turns reached without approval.",
            DuoPhase::Player => "🔄 Continuing to next player turn...",
            _ => "Session updated.",
        };

        let summary = state.summary();
        let mut result = ToolResult::success(format!("{}\n\n{}", status_msg, summary));
        result.metadata = Some(json!({
            "phase": state.phase.to_string(),
            "status": state.status.to_string(),
            "turn": state.current_turn,
            "max_turns": state.max_turns,
            "approved": approved,
            "compliance_score": compliance_score,
            "is_complete": state.is_complete(),
        }));

        Ok(result)
    }
}

/// Show the current session status.
pub struct DuoStatusTool {
    session: SharedDuoSession,
}

impl DuoStatusTool {
    #[must_use]
    pub fn new(session: SharedDuoSession) -> Self {
        Self { session }
    }
}

#[async_trait]
impl ToolSpec for DuoStatusTool {
    fn name(&self) -> &'static str {
        "duo_status"
    }

    fn description(&self) -> &'static str {
        "Show the current Duo session status including phase, turn count, and requirements."
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

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Auto
    }

    async fn execute(
        &self,
        _input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let session = self
            .session
            .lock()
            .map_err(|_| ToolError::execution_failed("Failed to lock Duo session"))?;

        Ok(ToolResult::success(session_summary(&session)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duo::new_shared_duo_session;

    #[test]
    fn test_duo_init_tool_schema() {
        let session = new_shared_duo_session();
        let tool = DuoInitTool::new(session);

        assert_eq!(tool.name(), "duo_init");
        assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);

        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("requirements"))
        );
    }

    #[test]
    fn test_duo_player_tool_schema() {
        let session = new_shared_duo_session();
        let tool = DuoPlayerTool::new(session);

        assert_eq!(tool.name(), "duo_player");
        assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
    }

    #[test]
    fn test_duo_coach_tool_schema() {
        let session = new_shared_duo_session();
        let tool = DuoCoachTool::new(session);

        assert_eq!(tool.name(), "duo_coach");
        assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
    }

    #[test]
    fn test_duo_advance_tool_schema() {
        let session = new_shared_duo_session();
        let tool = DuoAdvanceTool::new(session);

        assert_eq!(tool.name(), "duo_advance");
        assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);

        let schema = tool.input_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("feedback")));
        assert!(required.contains(&json!("approved")));
    }

     #[test]
     fn test_duo_status_tool_schema() {
         let session = new_shared_duo_session();
         let tool = DuoStatusTool::new(session);

         assert_eq!(tool.name(), "duo_status");
         assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
     }
 }

 // === File System Tools ===

 /// Write content to a file (for player implementation).
 /// Path is relative to workspace and sandboxed for security.
 pub struct DuoWriteFileTool {
     workspace: std::path::PathBuf,
 }

 impl DuoWriteFileTool {
     #[must_use]
     pub fn new(workspace: std::path::PathBuf) -> Self {
         Self { workspace }
     }
 }

 #[async_trait]
 impl ToolSpec for DuoWriteFileTool {
     fn name(&self) -> &'static str {
         "duo_write_file"
     }

     fn description(&self) -> &'static str {
         "Write content to a file in the workspace. Use this for the PLAYER to implement code files."
     }

     fn input_schema(&self) -> Value {
         json!({
             "type": "object",
             "properties": {
                 "path": {
                     "type": "string",
                     "description": "Path to the file (relative to workspace)"
                 },
                 "content": {
                     "type": "string",
                     "description": "Content to write"
                 }
             },
             "required": ["path", "content"]
         })
     }

     fn capabilities(&self) -> Vec<ToolCapability> {
         vec![
             ToolCapability::WritesFiles,
             ToolCapability::Sandboxable,
         ]
     }

     fn approval_requirement(&self) -> ApprovalRequirement {
         ApprovalRequirement::Auto
     }

     async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
         let path_str = required_str(&input, "path")?;
         let file_content = required_str(&input, "content")?;

         let file_path = self.workspace.join(path_str);

         if let Some(parent) = file_path.parent() {
             tokio::fs::create_dir_all(parent)
                 .await
                 .map_err(|e| ToolError::execution_failed(format!("Failed to create directory: {}", e)))?;
         }

         tokio::fs::write(&file_path, file_content)
             .await
             .map_err(|e| ToolError::execution_failed(format!("Failed to write file: {}", e)))?;

         Ok(ToolResult::success(format!(
             "Wrote {} bytes to {}",
             file_content.len(),
             path_str
         )))
     }
 }

 /// Read a file from the workspace (for coach validation).
 pub struct DuoReadFileTool {
     workspace: std::path::PathBuf,
 }

 impl DuoReadFileTool {
     #[must_use]
     pub fn new(workspace: std::path::PathBuf) -> Self {
         Self { workspace }
     }
 }

 #[async_trait]
 impl ToolSpec for DuoReadFileTool {
     fn name(&self) -> &'static str {
         "duo_read_file"
     }

     fn description(&self) -> &'static str {
         "Read a file from the workspace. Use this for the COACH to validate implementation files."
     }

     fn input_schema(&self) -> Value {
         json!({
             "type": "object",
             "properties": {
                 "path": {
                     "type": "string",
                     "description": "Path to the file (relative to workspace)"
                 }
             },
             "required": ["path"]
         })
     }

     fn capabilities(&self) -> Vec<ToolCapability> {
         vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
     }

     fn approval_requirement(&self) -> ApprovalRequirement {
         ApprovalRequirement::Auto
     }

     async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
         let path_str = required_str(&input, "path")?;

         let file_path = self.workspace.join(path_str);

         let content = tokio::fs::read_to_string(&file_path)
             .await
             .map_err(|e| ToolError::execution_failed(format!("Failed to read file: {}", e)))?;

         Ok(ToolResult::success(content))
     }
 }

 /// List files in a directory (for context discovery).
 pub struct DuoListDirTool {
     workspace: std::path::PathBuf,
 }

 impl DuoListDirTool {
     #[must_use]
     pub fn new(workspace: std::path::PathBuf) -> Self {
         Self { workspace }
     }
 }

 #[async_trait]
 impl ToolSpec for DuoListDirTool {
     fn name(&self) -> &'static str {
         "duo_list_dir"
     }

     fn description(&self) -> &'static str {
         "List files in a directory. Use this to explore the workspace structure."
     }

     fn input_schema(&self) -> Value {
         json!({
             "type": "object",
             "properties": {
                 "path": {
                     "type": "string",
                     "description": "Path to directory (relative to workspace, default: .)"
                 }
             },
             "required": []
         })
     }

     fn capabilities(&self) -> Vec<ToolCapability> {
         vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable]
     }

     fn approval_requirement(&self) -> ApprovalRequirement {
         ApprovalRequirement::Auto
     }

     async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
         let path_str = optional_str(&input, "path").unwrap_or(".");
         let dir_path = self.workspace.join(path_str);

         let mut entries = Vec::new();

         let mut stream = tokio::fs::read_dir(&dir_path)
             .await
             .map_err(|e| ToolError::execution_failed(format!("Failed to read directory: {}", e)))?;

         while let Some(entry) = stream.next_entry().await.map_err(|e| ToolError::execution_failed(e.to_string()))? {
             let path = entry.path();
             let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("").to_string();
             let file_type = entry.file_type().await.map_err(|e| ToolError::execution_failed(e.to_string()))?;

             if !should_skip_file(&file_name) {
                 entries.push(json!({
                     "name": file_name,
                     "is_dir": file_type.is_dir(),
                     "path": path.to_string_lossy().to_string()
                 }));
             }
         }

         ToolResult::json(&entries).map_err(|e| ToolError::execution_failed(e.to_string()))
     }
 }

 fn should_skip_file(file_name: &str) -> bool {
     matches!(
         file_name,
         ".git"
             | ".svn"
             | ".idea"
             | "target"
             | "node_modules"
             | "vendor"
             | ".DS_Store"
             | "Thumbs.db"
             | ".minimax"
             | "Cargo.lock"
             | "package-lock.json"
             | "yarn.lock"
     )
 }

 #[cfg(test)]
 mod file_tool_tests {
     use super::*;
     use crate::tools::spec::ToolContext;
     use tempfile::tempdir;

     #[tokio::test]
     async fn test_duo_write_file_tool() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();
         let tool = DuoWriteFileTool::new(workspace.clone());

         let result = tool
             .execute(
                 json!({"path": "test.txt", "content": "hello world"}),
                 &ToolContext::new(workspace),
             )
             .await
             .expect("execute");

         assert!(result.success);
         assert!(result.content.contains("Wrote"));

         let written = std::fs::read_to_string(tmp.path().join("test.txt")).expect("read");
         assert_eq!(written, "hello world");
     }

     #[tokio::test]
     async fn test_duo_write_file_creates_dirs() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();
         let tool = DuoWriteFileTool::new(workspace.clone());

         let result = tool
             .execute(
                 json!({"path": "subdir/nested/file.txt", "content": "nested content"}),
                 &ToolContext::new(workspace),
             )
             .await
             .expect("execute");

         assert!(result.success);

         let written = std::fs::read_to_string(tmp.path().join("subdir/nested/file.txt")).expect("read");
         assert_eq!(written, "nested content");
     }

     #[tokio::test]
     async fn test_duo_read_file_tool() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();

         std::fs::write(tmp.path().join("test.txt"), "hello world").expect("write");

         let tool = DuoReadFileTool::new(workspace.clone());
         let result = tool
             .execute(json!({"path": "test.txt"}), &ToolContext::new(workspace))
             .await
             .expect("execute");

         assert!(result.success);
         assert_eq!(result.content, "hello world");
     }

     #[tokio::test]
     async fn test_duo_list_dir_tool() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();

         std::fs::write(tmp.path().join("file1.txt"), "").expect("write");
         std::fs::write(tmp.path().join("file2.txt"), "").expect("write");
         std::fs::create_dir(tmp.path().join("subdir")).expect("mkdir");

         let tool = DuoListDirTool::new(workspace.clone());
         let result = tool
             .execute(json!({}), &ToolContext::new(workspace))
             .await
             .expect("execute");

         assert!(result.success);
         let entries: Vec<Value> = serde_json::from_str(&result.content).expect("parse");
         assert!(entries.iter().any(|e| e["name"] == "file1.txt"));
         assert!(entries.iter().any(|e| e["name"] == "file2.txt"));
         assert!(entries.iter().any(|e| e["name"] == "subdir" && e["is_dir"] == true));
     }

     #[tokio::test]
     async fn test_duo_list_dir_filters_hidden() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();

         std::fs::write(tmp.path().join("file.txt"), "").expect("write");
         std::fs::create_dir(tmp.path().join("target")).expect("mkdir");
         std::fs::write(tmp.path().join("target").join("lib.rs"), "").expect("write");

         let tool = DuoListDirTool::new(workspace.clone());
         let result = tool
             .execute(json!({}), &ToolContext::new(workspace))
             .await
             .expect("execute");

         assert!(result.success);
         let entries: Vec<Value> = serde_json::from_str(&result.content).expect("parse");
         assert!(entries.iter().any(|e| e["name"] == "file.txt"));
         assert!(!entries.iter().any(|e| e["name"] == "target"));
     }

      #[test]
      fn test_duo_write_file_tool_properties() {
          let tmp = tempdir().expect("tempdir");
          let workspace = tmp.path().to_path_buf();
          let tool = DuoWriteFileTool::new(workspace.clone());

          assert_eq!(tool.name(), "duo_write_file");
          assert!(!tool.is_read_only());
          assert!(tool.is_sandboxable());
          assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
      }

     #[test]
     fn test_duo_read_file_tool_properties() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();
         let tool = DuoReadFileTool::new(workspace.clone());

         assert_eq!(tool.name(), "duo_read_file");
         assert!(tool.is_read_only());
         assert!(tool.is_sandboxable());
         assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
     }

     #[test]
     fn test_duo_list_dir_tool_properties() {
         let tmp = tempdir().expect("tempdir");
         let workspace = tmp.path().to_path_buf();
         let tool = DuoListDirTool::new(workspace.clone());

         assert_eq!(tool.name(), "duo_list_dir");
         assert!(tool.is_read_only());
         assert!(tool.is_sandboxable());
         assert_eq!(tool.approval_requirement(), ApprovalRequirement::Auto);
     }

     #[test]
     fn test_should_skip_file() {
         assert!(should_skip_file(".git"));
         assert!(should_skip_file("target"));
         assert!(should_skip_file("node_modules"));
         assert!(should_skip_file("Cargo.lock"));
         assert!(!should_skip_file("src"));
         assert!(!should_skip_file("main.rs"));
     }
 }
