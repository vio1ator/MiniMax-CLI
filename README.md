# MiniMax CLI

```
 ███╗   ███╗██╗███╗   ██╗██╗███╗   ███╗ █████╗ ██╗  ██╗
 ████╗ ████║██║████╗  ██║██║████╗ ████║██╔══██╗╚██╗██╔╝
 ██╔████╔██║██║██╔██╗ ██║██║██╔████╔██║███████║ ╚███╔╝
 ██║╚██╔╝██║██║██║╚██╗██║██║██║╚██╔╝██║██╔══██║ ██╔██╗
 ██║ ╚═╝ ██║██║██║ ╚████║██║██║ ╚═╝ ██║██║  ██║██╔╝ ██╗
 ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
```

Chat with MiniMax M2.1 from your terminal.

> **Unofficial CLI** - Not affiliated with MiniMax Inc.

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

## Usage

```bash
# Start chatting (opens interactive TUI)
minimax

# One-shot prompt
minimax -p "Explain quantum computing"

# YOLO mode (enables agent tools + shell execution)
minimax --yolo
```

That's it. Just run `minimax` and start chatting.

## First Run

On first run, you'll be prompted to enter your MiniMax API key. Get one at [platform.minimax.chat](https://platform.minimax.chat).

Your key is saved to `~/.minimax/config.toml`.

## Commands

Inside the TUI, use slash commands:

| Command | Description |
|---------|-------------|
| `/help` | Show help |
| `/clear` | Clear conversation |
| `/yolo` | Enable agent mode + shell |
| `/exit` | Exit |

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Ctrl+C` | Exit |
| `F1` | Help |
| `Up/Down` | History |

## Configuration

Edit `~/.minimax/config.toml`:

```toml
api_key = "your-api-key"
default_text_model = "MiniMax-M2.1"
```

Or use environment variables:
```bash
export MINIMAX_API_KEY=your-api-key
```

## License

MIT

---

*This is an unofficial, community-maintained project. MiniMax and the MiniMax logo are trademarks of MiniMax Inc.*
