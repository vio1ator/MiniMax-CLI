# Axiom CLI Architecture

This document provides an overview of the Axiom CLI architecture for developers and contributors.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Interface                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │   TUI (ratatui) │  │  One-shot Mode  │  │  Config/CLI    │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬───────┘  │
└───────────┼─────────────────────┼────────────────────┼──────────┘
            │                     │                    │
            ▼                     ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Core Engine                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Agent Loop (core/engine.rs)           │   │
│  │  ┌─────────┐  ┌─────────────┐  ┌──────────────────────┐ │   │
│  │  │ Session │  │ Turn Mgmt   │  │ Tool Orchestration   │ │   │
│  │  └─────────┘  └─────────────┘  └──────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
            │                     │                    │
            ▼                     ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Tool & Extension Layer                      │
│  ┌──────────┐  ┌──────────┐  ┌─────────┐  ┌────────────────┐   │
│  │  Tools   │  │  Skills  │  │  Hooks  │  │  MCP Servers   │   │
│  │ (shell,  │  │ (plugins)│  │ (pre/   │  │  (external)    │   │
│  │  file)   │  │          │  │  post)  │  │                │   │
│  └──────────┘  └──────────┘  └─────────┘  └────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
            │                     │                    │
            ▼                     ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                        LLM Layer                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           LLM Client Abstraction (llm_client.rs)          │  │
│  │  ┌─────────────────┐  ┌─────────────────────────────┐    │  │
│  │  │ Generic Client  │  │  Compatible Client          │    │  │
│  │  │   (client.rs)   │  │       (client.rs)           │    │  │
│  │  └─────────────────┘  └─────────────────────────────┘    │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Module Organization

### Entry Point

- **`main.rs`** - CLI argument parsing (clap), configuration loading, entry point routing

### Core Components

- **`core/`** - Main engine components
  - `engine.rs` - Agent loop, message processing, tool execution orchestration
  - `session.rs` - Session state management
  - `turn.rs` - Turn-based conversation handling
  - `events.rs` - Event system for UI updates
  - `ops.rs` - Core operations

### Configuration

- **`config.rs`** - Configuration loading, profiles, environment variables
- **`settings.rs`** - Runtime settings management

### LLM Integration

- **`client.rs`** - HTTP client for LLM APIs, including the Anthropic-compatible format
- **`llm_client.rs`** - Abstract LLM client trait with retry logic
- **`models.rs`** - Data structures for API requests/responses

#### LLM API Endpoints

The CLI uses generic LLM provider APIs:
- Generic provider APIs via configurable base URL
- Anthropic-compatible endpoints (common format)

The engine uses standard API formats for chat models.

### Tool System

- **`tools/`** - Built-in tool implementations
  - `mod.rs` - Tool registry and common types
  - `shell.rs` - Shell command execution
  - `file.rs` - File read/write operations
  - `todo.rs` - Todo list management
  - `plan.rs` - Planning tools
  - `subagent.rs` - Sub-agent spawning
  - `spec.rs` - Tool specifications

### Duo Mode

- **`duo.rs`** - State machine, workflow execution, and session persistence
  - `DuoPhase`, `DuoStatus`, `DuoState` - Core state types
  - `run_duo_workflow()` - Player-coach loop with LLM API integration
  - `save_session()`, `load_session()`, `list_sessions()`, `delete_session()` - Session persistence to `~/.axiom/sessions/duo/`
  - File I/O helpers: `read_file()`, `write_file()`, `list_files()`, `validate_path()`
- **`tools/duo.rs`** - Tool definitions (`duo_init`, `duo_player`, `duo_coach`, `duo_advance`, `duo_status`)

### Extension Systems

- **`mcp.rs`** - Model Context Protocol client for external tool servers
- **`skills.rs`** - Plugin/skill loading and execution
- **`hooks.rs`** - Pre/post execution hooks with conditions

### User Interface

- **`tui/`** - Terminal UI components (ratatui-based)
  - `app.rs` - Application state and message handling
  - `ui.rs` - Rendering logic (includes Duo footer progress indicator)
  - `approval.rs` - Tool approval dialog
  - `clipboard.rs` - Clipboard handling
  - `streaming.rs` - Streaming text collector
  - `views/duo_view.rs` - DuoView modal component (phase color coding, turn counter, progress bar, quality scores, feedback history, loop visualization)
  - `duo_session_picker.rs` - DuoSessionPicker session browser (fuzzy search, metadata preview, resume capability)

- **`ui.rs`** - Legacy/simple UI utilities

### Security

- **`sandbox/`** - macOS sandboxing support
  - `mod.rs` - Sandbox type definitions
  - `policy.rs` - Sandbox policy configuration
  - `seatbelt.rs` - macOS Seatbelt profile generation

### Utilities

- **`utils.rs`** - Common utilities
- **`logging.rs`** - Logging infrastructure
- **`compaction.rs`** - Context compaction for long conversations
- **`rlm.rs`** - Reflection/reasoning utilities
- **`pricing.rs`** - Cost estimation
- **`prompts.rs`** - System prompt templates
- **`project_doc.rs`** - Project documentation handling
- **`session.rs`** - Session serialization

## Data Flow

### Interactive Session

1. User input received in TUI
2. Input processed by `core/engine.rs`
3. Message sent to LLM via `llm_client.rs`
4. Response streamed back, parsed in `client.rs`
5. Tool calls extracted and executed via `tools/`
6. Hooks triggered before/after tool execution
7. Results aggregated and sent back to LLM
8. Final response rendered in TUI

### Tool Execution

1. LLM requests tool via `tool_use` content block
2. Tool registry looks up handler
3. Pre-execution hooks run
4. Approval requested if needed (non-yolo mode)
5. Tool executed (possibly sandboxed on macOS)
6. Post-execution hooks run
7. Result returned to agent loop

## Extension Points

### Adding a New Tool

1. Create handler in `tools/`
2. Register in `tools/registry.rs`
3. Add tool specification (name, description, input schema)

### Adding an MCP Server

1. Configure in `~/.axiom/mcp.json`
2. Server auto-discovered at startup
3. Tools exposed to LLM automatically

### Creating a Skill

1. Create skill directory with `SKILL.md`
2. Define skill prompt and optional scripts
3. Place in `~/.axiom/skills/`

### Adding Hooks

Configure in `~/.axiom/config.toml`:

```toml
[[hooks]]
event = "tool_call_before"
command = "echo 'Running tool: $TOOL_NAME'"
```

## Key Design Decisions

1. **Streaming-first**: All LLM responses stream for responsiveness
2. **Tool safety**: Non-yolo mode requires approval for destructive operations
3. **Extensibility**: MCP, skills, and hooks allow customization without code changes
4. **Cross-platform**: Core works on Linux/macOS/Windows, sandboxing macOS-only
5. **Minimal dependencies**: Careful dependency selection for build speed

## Configuration Files

- `~/.axiom/config.toml` - Main configuration
- `~/.axiom/mcp.json` - MCP server configuration
- `~/.axiom/skills/` - User skills directory
- `~/.axiom/sessions/` - Session history
- `~/.axiom/sessions/duo/` - Duo mode session persistence
