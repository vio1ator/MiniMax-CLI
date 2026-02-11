//! Tool system modules and re-exports.

#![allow(dead_code, unused_imports)]

// === Modules ===

pub mod artifact;
pub mod coding;
pub mod duo;
pub mod execution;
pub mod file;
pub mod git;
pub mod investigator;
pub mod memory;
pub mod patch;
pub mod plan;
pub mod registry;
pub mod rlm;
pub mod search;
pub mod security;
pub mod shell;
pub mod spec;
pub mod subagent;
pub mod think;
pub mod todo;
pub mod web_search;

// === Re-exports ===

// Re-export commonly used types from spec
pub use spec::ToolContext;

// Re-export coding tools
pub use coding::{CodingCompleteTool, CodingReviewTool};

// Re-export git tools
pub use git::{GitBranchTool, GitCommitTool, GitDiffTool, GitLogTool, GitStatusTool};

// Re-export memory tools
pub use memory::{GetMemoryTool, SaveMemoryTool};

// Re-export artifact tools
pub use artifact::{ArtifactCreateTool, ArtifactListTool};

// Re-export execution tools
pub use execution::ExecPythonTool;

// Re-export investigator tools
pub use investigator::CodebaseInvestigatorTool;

// Re-export registry types
pub use registry::{ToolRegistry, ToolRegistryBuilder};

// Re-export search tools
pub use search::GrepFilesTool;

// Re-export web search tools
pub use web_search::{WebFetchTool, WebSearchTool};

// Re-export patch tools
pub use patch::ApplyPatchTool;

// Re-export file tools
pub use file::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};

// Re-export shell types
pub use shell::{ExecShellInteractTool, ExecShellKillTool, ExecShellTool, ExecShellWaitTool};

// Re-export subagent types
pub use subagent::SubAgent;

// Re-export todo types
pub use todo::TodoWriteTool;

// Re-export plan types
pub use plan::UpdatePlanTool;

// Re-export RLM tools
pub use rlm::{RlmExecTool, RlmLoadTool, RlmQueryTool, RlmStatusTool};

// Re-export think tool
pub use think::ThinkTool;
