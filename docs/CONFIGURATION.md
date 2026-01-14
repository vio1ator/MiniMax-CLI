# Configuration

MiniMax CLI reads configuration from a TOML file plus environment variables.

## Where It Looks

Default config path:

- `~/.minimax/config.toml`

Overrides:

- CLI: `minimax --config /path/to/config.toml`
- Env: `MINIMAX_CONFIG_PATH=/path/to/config.toml`

If both are set, `--config` wins. Environment variable overrides are applied after the file is loaded.

## Profiles

You can define multiple profiles in the same file:

```toml
api_key = "PERSONAL_KEY"
default_text_model = "MiniMax-M2.1"

[profiles.work]
api_key = "WORK_KEY"
base_url = "https://api.minimax.io"
```

Select a profile with:

- CLI: `minimax --profile work`
- Env: `MINIMAX_PROFILE=work`

If a profile is selected but missing, MiniMax CLI exits with an error listing available profiles.

## Environment Variables

These override config values:

- `MINIMAX_API_KEY`
- `MINIMAX_BASE_URL`
- `MINIMAX_OUTPUT_DIR`
- `MINIMAX_SKILLS_DIR`
- `MINIMAX_MCP_CONFIG`
- `MINIMAX_NOTES_PATH`
- `MINIMAX_MEMORY_PATH`
- `MINIMAX_ALLOW_SHELL` (`1`/`true` enables)
- `MINIMAX_MAX_SUBAGENTS` (clamped to `1..=5`)

## Key Reference

### Core keys (used by the TUI/engine)

- `api_key` (string, required): must be non-empty (or set `MINIMAX_API_KEY`).
- `base_url` (string, optional): defaults to `https://api.minimax.io` (the CLI derives the Anthropic-compatible endpoint as `<base_url>/anthropic`).
- `default_text_model` (string, optional): defaults to `MiniMax-M2.1`.
- `allow_shell` (bool, optional): defaults to `false`.
- `max_subagents` (int, optional): defaults to `5` and is clamped to `1..=5`.
- `skills_dir` (string, optional): defaults to `~/.minimax/skills` (each skill is a directory containing `SKILL.md`).
- `mcp_config_path` (string, optional): defaults to `~/.minimax/mcp.json`.
- `notes_path` (string, optional): defaults to `~/.minimax/notes.txt` and is used by the `note` tool.
- `retry.*` (optional): retry/backoff settings for API requests:
  - `[retry].enabled` (bool, default `true`)
  - `[retry].max_retries` (int, default `3`)
  - `[retry].initial_delay` (float seconds, default `1.0`)
  - `[retry].max_delay` (float seconds, default `60.0`)
  - `[retry].exponential_base` (float, default `2.0`)
- `hooks` (optional): lifecycle hooks configuration (see `config.example.toml`).

### Parsed but currently unused (reserved for future versions)

These keys are accepted by the config loader but not currently used by the interactive TUI or built-in tools:

- `default_image_model`, `default_video_model`, `default_audio_model`, `default_music_model`
- `output_dir`
- `tools_file`
- `memory_path`

## Notes On `minimax doctor`

`minimax doctor` checks default locations under `~/.minimax/` (including `config.toml` and `mcp.json`). If you override paths via `--config` or `MINIMAX_MCP_CONFIG`, the doctor output may not reflect those overrides.

