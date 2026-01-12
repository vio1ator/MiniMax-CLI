# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Run
cargo run                # Run debug build (opens TUI)
cargo run -- --yolo      # YOLO mode: agent tools + shell execution
cargo run -- -p "prompt" # One-shot prompt mode

# Lint and Format
cargo fmt                # Format code
cargo fmt --check        # Check formatting (CI uses this)
cargo clippy             # Run linter
cargo clippy --all-targets --all-features  # Full lint check (CI uses this)

# Test
cargo test               # Run all tests
cargo test --all-features  # Run with all features (CI uses this)

# Documentation
cargo doc --no-deps      # Build docs
```

## Architecture Overview

This is a Rust CLI application for chatting with MiniMax M2.1 (and optionally Claude). The architecture follows an event-driven model with clear separation between UI and core logic.

### Core Flow

```
User Input -> TUI (tui/app.rs) -> Op messages -> Engine (core/engine.rs) -> LLM Client -> Events -> TUI
                                                        |
                                                        v
                                                  Tool Execution
                                                  (tools/*.rs)
```

### Key Components

**Entry & CLI** (`main.rs`)
- Clap-based CLI with subcommands: `doctor`, `completions`, `sessions`, `init`
- Routes to TUI for interactive mode or one-shot for `-p` prompts

**Engine** (`core/engine.rs`)
- Async agent loop running in background task
- Communicates via `Op` (operations in) and `Event` (events out) channels
- Handles streaming responses, tool execution, cancellation

**LLM Layer** (`client.rs`, `llm_client.rs`)
- `AnthropicClient`: HTTP client for both MiniMax and Anthropic APIs
- Streaming SSE parsing, retry logic with exponential backoff

**Tool System** (`tools/`)
- `ToolRegistry` + `ToolRegistryBuilder` pattern for tool registration
- Built-in tools: shell, file ops, search, todo, plan, subagent, MiniMax media
- Tools receive `ToolContext` with workspace path, approval settings

**Extension Systems**
- **MCP** (`mcp.rs`): Model Context Protocol for external tool servers, configured in `~/.minimax/mcp.json`
- **Skills** (`skills.rs`): Plugin system, skills live in `~/.minimax/skills/` with `SKILL.md` files
- **Hooks** (`hooks.rs`): Lifecycle hooks (session_start, tool_call_before, etc.) configured in `config.toml`

**TUI** (`tui/`)
- Ratatui-based terminal UI with streaming support
- `app.rs`: Application state, message handling
- `approval.rs`: Tool approval dialogs (non-YOLO mode)

**Sandbox** (`sandbox/`)
- macOS-only sandboxing via Seatbelt profiles
- `seatbelt.rs` generates profiles, `policy.rs` defines policies

### Configuration Files

- `~/.minimax/config.toml` - API keys, default model, hooks
- `~/.minimax/mcp.json` - MCP server definitions
- `~/.minimax/skills/` - User-defined skills
- `AGENTS.md` (per-project) - Project-specific agent instructions

### Adding a New Tool

1. Create tool struct in `tools/` implementing the tool interface
2. Register in `tools/registry.rs` via `ToolRegistryBuilder`
3. Define tool spec with name, description, JSON schema for inputs

### MiniMax API Integration

MiniMax M2.1 supports two API formats:

1. **Anthropic-compatible** (`api.minimax.io/anthropic`) - MiniMax recommends this
2. **OpenAI-compatible** (`api.minimax.io/v1`) - Available but not used by the engine

#### Current Implementation (Anthropic format)

- `src/client.rs` `AnthropicClient` targets `anthropic_base_url` (defaults to MiniMax `/anthropic`)
- `core/engine.rs` uses `handle_anthropic_turn()` for MiniMax and Claude models
- Streaming responses with structured tool calls and `thinking` blocks

#### Tool Calling

Anthropic format tools use `name`, `description`, and `input_schema`:
```json
{
  "tools": [{
    "name": "tool_name",
    "description": "...",
    "input_schema": { /* JSON Schema */ }
  }]
}
```

Tool results are sent back as `tool_result` content blocks.

#### Interleaved Thinking

Thinking arrives as `thinking` content blocks in the stream when the model emits them.

#### Debug Logging

Enable `RUST_LOG` if you need request/response logging from the Anthropic path.

### Commit Messages

Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
