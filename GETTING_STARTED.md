# WebSearch-MCP — Quick Start

After extracting this archive, you have the `websearch` binary (or `websearch.exe` on Windows). Here's how to use it.

## 1. Prerequisites

- **Chrome or Chromium** must be installed on your system.
- **macOS or Linux** (Windows support requires running under WSL or similar).

## 2. Run the server

```bash
# Make the binary executable (macOS / Linux)
chmod +x websearch

# Start with a visible browser (default)
./websearch --wait-seconds 5

# Or headless (no visible window)
./websearch --headless --wait-seconds 5
```

## 3. Configure in your MCP host

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "websearch": {
      "command": "/full/path/to/websearch",
      "args": ["--wait-seconds", "5"]
    }
  }
}
```

### VS Code (`.vscode/mcp.json`)

```json
{
  "servers": {
    "websearch": {
      "type": "stdio",
      "command": "/full/path/to/websearch",
      "args": ["--wait-seconds", "5"]
    }
  }
}
```

## 4. Available tools

| Tool | What it does |
|---|---|
| `search` | Search the web via Brave, DuckDuckGo, or Google. Returns results as Markdown. |
| `fetch` | Fetch any URL and return clean Markdown (nav, ads, footers stripped). |

## 5. Common options

| Flag | Default | Description |
|---|---|---|
| `--wait-seconds` | `4` | Seconds to wait for pages to render |
| `--headless` | off | Run without a visible browser window |
| `--chrome` | autodetected | Path to Chrome/Chromium executable |
| `--profile` | `$DATA_DIR/websearch-mcp/chrome-profile` | Browser profile directory |

---

Full documentation: https://github.com/your-org/websearch-mcp
