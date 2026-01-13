# MiniMax CLI

> **Alpha** — Unofficial community project. Not affiliated with MiniMax Inc.

![CI](https://github.com/Hmbown/MiniMax-CLI/actions/workflows/ci.yml/badge.svg)

Terminal client for [MiniMax M2.1](https://platform.minimax.io). Supports chat, agent mode, media generation, and custom skills.

## Install

```bash
cargo install minimax-cli
```

Or build from source:

```bash
git clone https://github.com/Hmbown/MiniMax-CLI
cd MiniMax-CLI
cargo install --path .
```

Prebuilt binaries: [Releases](https://github.com/Hmbown/MiniMax-CLI/releases)

## Usage

```bash
minimax              # Interactive TUI
minimax -p "prompt"  # One-shot mode
minimax --yolo       # Agent mode (shell + file tools)
```

## Configuration

Create `~/.minimax/config.toml`:

```toml
api_key = "your-api-key"
```

Or set `MINIMAX_API_KEY` in your environment.

Get an API key at [platform.minimax.io](https://platform.minimax.io).

## Features

- **Chat**: Streaming responses with 204K context
- **Agent mode**: File operations, shell execution, MCP tools
- **Media tools**: Image, audio, video, music generation
- **Skills**: Reusable workflows (see `skills/` directory)

## Commands

| Command | Description |
|---------|-------------|
| `/help` | Show help |
| `/mode` | Switch modes (normal/edit/agent/plan) |
| `/model` | Switch models |
| `/yolo` | Enable agent mode |
| `/skills` | List available skills |
| `/save` `/load` | Session management |
| `/tokens` | Show usage |

## Skills

Skills are workflow templates in `skills/`. Run with `/skills <name>`.

Available: `video-studio`, `voiceover-studio`, `audiobook-studio`, `music-video-generator`, and more.

Create your own by adding a `SKILL.md` file to a new folder in `~/.minimax/skills/`.

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)

## License

MIT

---

*Unofficial project. MiniMax is a trademark of MiniMax Inc.*
