# MiniMax CLI

[![CI](https://github.com/Hmbown/MiniMax-CLI/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/MiniMax-CLI/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/minimax-cli)](https://crates.io/crates/minimax-cli)

Unofficial terminal UI (TUI) + CLI for the [MiniMax platform](https://platform.minimax.io): chat with **MiniMax-M2.1**, run an approval-gated tool-using agent, and generate media (images, video, music, TTS).

Not affiliated with MiniMax Inc.

## Quickstart

1. Get an API key from https://platform.minimax.io
2. Install and run:

```bash
cargo install minimax-cli --locked
export MINIMAX_API_KEY="YOUR_MINIMAX_API_KEY"
minimax
```

Or download a prebuilt binary from [GitHub Releases](https://github.com/Hmbown/MiniMax-CLI/releases).

3. Press `F1` or run `/help` for the in-app command list
4. If anything looks off, run `minimax doctor`

## Install

### From crates.io (recommended)

```bash
cargo install minimax-cli --locked
```

### Build from source

```bash
git clone https://github.com/Hmbown/MiniMax-CLI.git
cd MiniMax-CLI
cargo build --release
./target/release/minimax --help
```

### Direct download

Download a prebuilt binary from https://github.com/Hmbown/MiniMax-CLI/releases and put it on your `PATH` as `minimax`.

## Configuration

On first run, the TUI can prompt for your API key and save it to `~/.minimax/config.toml`. You can also create the file manually:

```toml
# ~/.minimax/config.toml
api_key = "YOUR_MINIMAX_API_KEY"   # must be non-empty
default_text_model = "MiniMax-M2.1" # optional
allow_shell = false                 # optional
max_subagents = 3                   # optional (1-5)

# Optional: Configure custom model context windows (for self-hosted models)
[model_context_windows]
"my-custom-model" = 128000
```

Useful environment variables:

- `MINIMAX_API_KEY` (overrides `api_key`)
- `MINIMAX_BASE_URL` (default: `https://api.minimax.io`; China users may use `https://api.minimaxi.com`)
- `MINIMAX_MODEL_CONTEXT_WINDOWS` (custom model contexts, format: `model1:context1,model2:context2`)
- `MINIMAX_PROFILE` (selects `[profiles.<name>]` from the config; errors if missing)
- `MINIMAX_CONFIG_PATH` (override config path)
- `MINIMAX_MCP_CONFIG`, `MINIMAX_SKILLS_DIR`, `MINIMAX_NOTES_PATH`, `MINIMAX_MEMORY_PATH`, `MINIMAX_ALLOW_SHELL`, `MINIMAX_MAX_SUBAGENTS`

See `config.example.toml` and `docs/CONFIGURATION.md` for a full reference.

## Modes

In the TUI, press `Tab` to cycle modes: **Normal → Plan → Agent → YOLO → RLM → Duo → Normal**.

- **Normal**: chat; asks before file writes, shell, or paid tools
- **Plan**: design-first prompting; same approvals as Normal
- **Agent**: multi-step tool use; asks before shell or paid tools
- **YOLO**: enables shell + trust + auto-approves all tools (dangerous)
- **RLM**: externalized context + REPL helpers; auto-approves tools (best for large files)
- **Duo**: player-coach autocoding with iterative validation, dedicated TUI view, and session management (based on g3 paper)

Approval behavior is mode-dependent, but you can also override it at runtime with `/set approval_mode auto|suggest|never`.

## Tools

MiniMax CLI exposes tools to the model: file read/write/patching, shell execution, web search, sub-agents, and MiniMax media APIs.

- **Workspace boundary**: file tools are restricted to `--workspace` unless you enable `/trust` (YOLO enables trust automatically).
- **Approvals**: the TUI requests approval depending on mode and tool category (file writes, shell, paid media).
- **Web search**: `web_search` uses DuckDuckGo HTML results and is auto-approved.
- **Media tools**: image/video/music/TTS tools make paid API calls and write real files.
- **Skills**: reusable workflows stored as `SKILL.md` directories (default: `~/.minimax/skills`). Use `/skills` and `/skill <name>` (this repo includes examples under `skills/`).
- **MCP**: load external tool servers via `~/.minimax/mcp.json` (supports `servers` and `mcpServers`). MCP tools currently execute without TUI approval prompts, so only enable servers you trust. See `docs/MCP.md`.

## RLM

RLM mode is designed for “too big for context” tasks: large files, whole-doc sweeps, and big pasted blocks.

- Auto-switch triggers: “largest file”, explicit “RLM”, large file requests, and large pastes.
- In **RLM mode**, `/load @path` loads a file into the external context store (outside RLM mode, `/load` loads a saved chat JSON).
- Use `/repl` to enter expression mode (e.g. `search(\"pattern\")`, `lines(1, 80)`).
- Power tools: `rlm_load`, `rlm_exec`, `rlm_status`, `rlm_query`.

`rlm_query` can be expensive: prefer batching and check `/status` if you're doing lots of sub-queries.

## Duo Mode

Duo mode implements the player-coach autocoding paradigm for iterative development with built-in validation:

- **Player**: implements requirements (builder role)
- **Coach**: validates implementation against requirements (critic role)
- Tools: `duo_init`, `duo_player`, `duo_coach`, `duo_advance`, `duo_status`

Workflow: `init → player → coach → advance → (repeat until approved)`

### Duo Session Management

Sessions are persisted to `~/.minimax/sessions/duo/` and auto-saved after each turn. Use the CLI or TUI to manage sessions:

- `minimax duo start --requirements <file>` - Start a new Duo workflow
- `minimax duo continue <id>` - Resume a saved session by ID
- `minimax duo sessions` - List all saved Duo sessions

In the TUI, the **DuoView** modal shows phase color coding, turn counter, quality scores, feedback history, and a loop visualization of Player/Coach progression. The **DuoSessionPicker** provides fuzzy search over saved sessions with metadata preview and resume capability. The footer displays a progress indicator (e.g. `🎮 Player Phase (Turn 2/10)`).

## Examples

```bash
minimax                       # Interactive TUI
minimax -p "Write a haiku"     # One-shot prompt (prints and exits)

minimax doctor                 # Diagnose config + API key
minimax sessions --limit 50    # List sessions (~/.minimax/sessions)
minimax --resume latest        # Resume most recent session
minimax --resume <id-prefix>   # Resume by ID/prefix

minimax --workspace /path/to/project
minimax --yolo                 # Start in YOLO mode (dangerous)

minimax duo start --requirements docs/requirements.md  # Start Duo workflow
minimax duo continue <session-id>                      # Resume a Duo session
minimax duo sessions                                   # List Duo sessions
minimax duo start --requirements spec.md --workspace .  # Duo in current dir

minimax init                   # Generate a starter AGENTS.md
```

Shell completions:

```bash
minimax completions zsh > _minimax
minimax completions bash > minimax.bash
minimax completions fish > minimax.fish
```

Run the paid media smoke test (writes real files and spends credits):

```bash
minimax --workspace . smoke-media --confirm
```

## Troubleshooting

- **No API key**: set `MINIMAX_API_KEY` or run `minimax` and complete onboarding
- **Config not found**: check `~/.minimax/config.toml` (or `MINIMAX_CONFIG_PATH`)
- **Wrong region / base URL**: set `MINIMAX_BASE_URL` to `https://api.minimaxi.com` (China)
- **Session issues**: run `minimax sessions` and try `minimax --resume latest`
- **MCP tools missing**: validate `~/.minimax/mcp.json` (or `MINIMAX_MCP_CONFIG`) and restart

## Documentation

- `docs/README.md`
- `docs/CONFIGURATION.md`
- `docs/MCP.md`
- `docs/ARCHITECTURE.md`
- `docs/DUO.md`
- `CONTRIBUTING.md`

## Development

```bash
cargo build
cargo test
cargo fmt
cargo clippy
```

## License

MIT

---

MiniMax is a trademark of MiniMax Inc. This is an unofficial project.
