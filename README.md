# MiniMax CLI

```
 ███╗   ███╗██╗███╗   ██╗██╗███╗   ███╗ █████╗ ██╗  ██╗
 ████╗ ████║██║████╗  ██║██║████╗ ████║██╔══██╗╚██╗██╔╝
 ██╔████╔██║██║██╔██╗ ██║██║██╔████╔██║███████║ ╚███╔╝
 ██║╚██╔╝██║██║██║╚██╗██║██║██║╚██╔╝██║██╔══██║ ██╔██╗
 ██║ ╚═╝ ██║██║██║ ╚████║██║██║ ╚═╝ ██║██║  ██║██╔╝ ██╗
 ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
```

**Unofficial CLI for the MiniMax M2.1 API** - *Not affiliated with MiniMax Inc.*

A powerful, feature-rich Rust CLI for interacting with MiniMax's M2.1 model via the Anthropic-compatible API. Supports text chat, agentic workflows, image/video/audio/music generation, and a Codex-like RLM sandbox mode.

## Features

- **Interactive TUI** - Beautiful ratatui-based terminal UI with multiple modes
- **Agent Mode** - Autonomous task execution with built-in tools
- **RLM Sandbox** - Recursive Language Model REPL for large context processing
- **Multi-Modal Generation** - Text, image, video, audio, and music
- **MCP Integration** - Model Context Protocol server support
- **Memory System** - Persistent long-term memory across sessions
- **Skills System** - Extensible prompt augmentation
- **Prompt Caching** - Efficient token usage with cache controls
- **Context Compaction** - Auto-summarization for long conversations

## Quick Start

```bash
# Install
cargo install --path .

# Configure
export MINIMAX_API_KEY=your_api_key_here
# Or create ~/.minimax/config.toml

# Interactive TUI (recommended)
minimax-cli tui

# Simple chat
minimax-cli text chat --prompt "Hello, MiniMax!"

# Agent mode with tools
minimax-cli agent run --allow-shell --prompt "List files in current directory"

# RLM sandbox for large files
minimax-cli rlm repl --load large_file.txt
```

## Modes

### Normal Mode (Chat)
Standard conversational interaction with the M2.1 model.

```bash
minimax-cli text chat
# Or in TUI: /mode normal
```

### Agent Mode
Autonomous task execution with built-in tools:
- `list_dir` - List directory contents
- `read_file` - Read files from workspace
- `write_file` - Write files to workspace
- `edit_file` - Search/replace in files
- `exec_shell` - Execute shell commands (requires `--allow-shell`)
- `note` - Append to notes file
- `mcp_call` - Call MCP server tools

```bash
minimax-cli agent run --workspace ./project --allow-shell
# Or in TUI: /mode agent or /yolo (enables shell)
```

### Plan Mode
Design-first workflow - plan your implementation before coding.

```bash
# In TUI: /mode plan
```

### Edit Mode
File modification focus with AI assistance.

```bash
# In TUI: /mode edit
```

### RLM Sandbox Mode
Recursive Language Model sandbox for processing large contexts. Based on the RLM paradigm that enables LMs to programmatically examine, decompose, and recursively process inputs of near-infinite length.

```bash
# Interactive REPL
minimax-cli rlm repl --load myfile.txt

# Command-line operations
minimax-cli rlm load --path myfile.txt --context-id main
minimax-cli rlm search --context-id main --pattern "TODO|FIXME"
minimax-cli rlm exec --context-id main --code "lines(0, 50)"
```

**RLM Expressions:**
- `len` - Character count
- `line_count` - Line count
- `head` / `tail` - First/last 10 lines
- `peek(start, end)` - Character slice
- `lines(start, end)` - Line range
- `search("pattern")` - Regex search
- `chunk(size, overlap)` - Split into chunks

## Interactive TUI

The TUI provides a rich terminal interface with:
- MiniMax branded header with logo
- Mode indicator (Normal/Edit/Agent/Plan/RLM)
- Chat history with scrolling
- Status bar with keybindings
- Help popup (F1)

**TUI Commands:**
| Command | Description |
|---------|-------------|
| `/mode <n/e/a/p/r>` | Switch modes |
| `/yolo` | Enable Agent + Shell |
| `/help` | Show help |
| `/clear` | Clear conversation |
| `/model <name>` | Change model |
| `/compact` | Toggle auto-compaction |
| `/save <path>` | Save session |
| `/load <path>` | Load session |
| `/exit` | Exit application |

**Keybindings:**
| Key | Action |
|-----|--------|
| `F1` | Help |
| `Ctrl+C` | Exit |
| `Esc` | Clear input / Normal mode |
| `Alt+Up/Down` | Scroll chat |
| `PageUp/PageDown` | Fast scroll |
| `Up/Down` | History navigation |

## Configuration

Create `~/.minimax/config.toml`:

```toml
api_key = "YOUR_MINIMAX_API_KEY"
anthropic_api_key = "YOUR_ANTHROPIC_COMPAT_API_KEY"
base_url = "https://api.minimax.io"
anthropic_base_url = "https://api.minimax.io/anthropic"

default_text_model = "MiniMax-M2.1"
output_dir = "./outputs"
allow_shell = false

[retry]
enabled = true
max_retries = 3

[compaction]
enabled = false
token_threshold = 50000
message_threshold = 50

[rlm]
max_context_chars = 10000000
session_dir = "~/.minimax/rlm"

[profiles.work]
api_key = "WORK_API_KEY"
```

**Environment Variables:**
- `MINIMAX_API_KEY` - API key
- `ANTHROPIC_API_KEY` - Anthropic-compatible API key
- `MINIMAX_BASE_URL` - Base URL
- `MINIMAX_PROFILE` - Config profile name
- `MINIMAX_ALLOW_SHELL` - Enable shell (true/false)

## Media Generation

### Images
```bash
minimax-cli image generate --prompt "A sunset over mountains"
```

### Videos
```bash
minimax-cli video generate --prompt "A timelapse of clouds" --wait
minimax-cli video query --task-id <id>
```

### Audio (TTS)
```bash
minimax-cli audio t2a --text "Hello, world!" --voice-id english-1
minimax-cli audio voice list
minimax-cli audio voice clone --clone-audio sample.wav
```

### Music
```bash
minimax-cli music generate --prompt "Upbeat electronic track"
```

## MCP Integration

Manage Model Context Protocol servers:

```bash
# List servers
minimax-cli mcp list

# Add server
minimax-cli mcp add --name myserver --command "python" --arg "-m" --arg "mcp_server"

# Remove server
minimax-cli mcp remove --name myserver
```

Use MCP tools in agent mode via the `mcp_call` tool.

## Memory & Skills

### Long-term Memory
```bash
minimax-cli memory show
minimax-cli memory add --content "Important fact to remember"
minimax-cli memory clear
```

Use `--memory` flag in agent/TUI to include memory in prompts.

### Skills
Skills are prompt templates stored in `~/.minimax/skills/<name>/SKILL.md`:

```bash
minimax-cli skills list
minimax-cli skills show --name coding
```

Load skills with `--skill coding --skill writing` in agent mode.

## Prompt Caching

Optimize token usage with Anthropic-style prompt caching:

```bash
# Cache system prompt
minimax-cli text chat --cache-system

# Cache tools
minimax-cli text chat --cache-tools

# Cache user message
minimax-cli text chat --cache

# Agent with caching
minimax-cli agent run --cache-system --cache-tools --cache-memory
```

## Context Compaction

Auto-summarize long conversations to stay within context limits:

```bash
# Enable in config.toml:
# [compaction]
# enabled = true
# token_threshold = 50000

# Or toggle in TUI:
# /compact
```

When enabled, older messages are summarized when the conversation exceeds thresholds.

## File Management

```bash
minimax-cli files upload --path file.txt --purpose "assistants"
minimax-cli files list --purpose "assistants"
minimax-cli files retrieve --file-id <id>
minimax-cli files retrieve-content --file-id <id> --output local.txt
minimax-cli files delete --file-id <id>
```

## Models Registry

```bash
minimax-cli models list
minimax-cli models list --json
```

## Session Management

Save and restore chat/agent sessions:

```bash
# In TUI or agent mode:
/save session.json
/load session.json
/export session.md
```

## Release

### GitHub Release
1. Update version in `Cargo.toml`
2. Tag release: `git tag v0.1.0 && git push --tags`
3. GitHub Actions builds and publishes

### PyPI (Python wrapper)
```bash
cd python && uv build && uv publish
```

### Crates.io
```bash
cargo publish
```

## Architecture

```
src/
├── main.rs          # CLI entry point (clap)
├── client.rs        # HTTP clients (MiniMax, Anthropic)
├── config.rs        # TOML config with profiles
├── models.rs        # API data structures
├── agent.rs         # Agentic tool loop
├── rlm.rs           # RLM sandbox REPL
├── compaction.rs    # Context auto-compaction
├── tui/             # Ratatui TUI
│   ├── app.rs       # App state & modes
│   └── ui.rs        # Rendering
├── modules/         # Media generation
│   ├── text.rs      # Chat
│   ├── image.rs     # Image gen
│   ├── video.rs     # Video gen
│   ├── audio.rs     # TTS
│   └── music.rs     # Music gen
├── mcp.rs           # MCP integration
├── skills.rs        # Skills system
├── memory.rs        # Long-term memory
└── session.rs       # Session save/load
```

## Requirements

- Rust 1.89+ (edition 2024)
- MiniMax API key
- macOS/Linux (Windows untested)

## License

MIT

---

*This is an unofficial, community-maintained project. MiniMax and the MiniMax logo are trademarks of MiniMax Inc.*
