# Configuration

Axiom CLI reads configuration from a TOML file plus environment variables.

## Where It Looks

Default config path:

- `~/.axiom/config.toml`

Overrides:

- CLI: `axiom --config /path/to/config.toml`
- Env: `AXIOM_CONFIG_PATH=/path/to/config.toml`

If both are set, `--config` wins. Environment variable overrides are applied after the file is loaded.

## Profiles

You can define multiple profiles in the same file:

```toml
api_key = "PERSONAL_KEY"
default_model = "anthropic/claude-3-5-sonnet-20241022"

[profiles.work]
api_key = "WORK_KEY"
base_url = "https://api.axiom.io"
```

Select a profile with:

- CLI: `axiom --profile work`
- Env: `AXIOM_PROFILE=work`

If a profile is selected but missing, Axiom CLI exits with an error listing available profiles.

## Environment Variables

These override config values:

- `AXIOM_API_KEY`
- `AXIOM_BASE_URL`
- `AXIOM_SKILLS_DIR`
- `AXIOM_MCP_CONFIG`
- `AXIOM_NOTES_PATH`
- `AXIOM_MEMORY_PATH`
- `AXIOM_ALLOW_SHELL` (`1`/`true` enables)
- `AXIOM_MAX_SUBAGENTS` (clamped to `1..=5`)

## Key Reference

### Core keys (used by the TUI/engine)

- `api_key` (string, required): must be non-empty (or set `AXIOM_API_KEY`).
- `base_url` (string, optional): defaults to `https://api.axiom.io`.
- `default_model` (string, optional): defaults to `anthropic/claude-3-5-sonnet-20241022`.
- `allow_shell` (bool, optional): defaults to `false`.
- `max_subagents` (int, optional): defaults to `5` and is clamped to `1..=5`.
- `skills_dir` (string, optional): defaults to `~/.axiom/skills` (each skill is a directory containing `SKILL.md`).
- `mcp_config_path` (string, optional): defaults to `~/.axiom/mcp.json`.
- `notes_path` (string, optional): defaults to `~/.axiom/notes.txt` and is used by the `note` tool.
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

## Notes On `axiom doctor`

`axiom doctor` checks default locations under `~/.axiom/` (including `config.toml` and `mcp.json`). If you override paths via `--config` or `AXIOM_MCP_CONFIG`, the doctor output may not reflect those overrides.

