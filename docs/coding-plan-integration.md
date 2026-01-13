# MiniMax Coding Plan MCP Integration

MiniMax CLI supports MCP servers so the model can call `web_search` and
`understand_image` when you have the Coding Plan subscription.

## Quick setup

1. Install `uv` (to get `uvx`).

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

2. Create `~/.minimax/mcp.json` with the Coding Plan MCP server.

```json
{
  "mcpServers": {
    "MiniMax": {
      "command": "uvx",
      "args": ["minimax-coding-plan-mcp", "-y"],
      "env": {
        "MINIMAX_API_KEY": "YOUR_CODING_PLAN_KEY",
        "MINIMAX_API_HOST": "https://api.minimax.io",
        "MINIMAX_MCP_BASE_PATH": "/absolute/path/for/outputs",
        "MINIMAX_API_RESOURCE_MODE": "url"
      }
    }
  }
}
```

MiniMax CLI accepts `mcpServers` (as shown above) or `servers` for backward
compatibility.

## Built-in web search

The CLI already provides a built-in `web_search` tool backed by DuckDuckGo.
If you do not want to use MCP, you can rely on that default tool instead.
