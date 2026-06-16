# WebSearch-MCP 

> MCP server for web search via headless Chromium

## Overview

`websearch-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server that provides web search capabilities to AI assistants. Instead of making basic HTTP requests or parsing RSS feeds, it controls a real Chrome/Chromium browser instance — navigating to search engines, waiting for JavaScript to render, extracting the resulting HTML, and converting it to clean Markdown. This approach means the LLM receives the search results page *as rendered*, including content that only appears after JS execution.

The server ships with three search engine providers: **Brave**, **DuckDuckGo**, and **Google**. The caller selects the provider by name in each MCP tool call. Brave is the default and recommended provider — it shows minimal advertising, works without a cookie wall, and its results page converts well to Markdown.

**Key differentiator:** By using a real browser, `websearch-mcp` handles JavaScript-heavy pages that basic HTTP fetchers cannot. And by returning Markdown instead of structured JSON or raw HTML, it lets the LLM parse results *naturally* — no fragile CSS selectors, no scraping contracts, just clean text the model already understands.

## Prerequisites

- **Rust 1.80+** — required for `std::sync::LazyLock`
- **Chrome or Chromium** installed on the system (autodetected on macOS and Linux)
- **macOS or Linux** — the server uses `pkill`/`pgrep` for browser process management

## Installation

```bash
git clone <repo>
cd websearch-mcp
cargo build --release
```

The compiled binary is at `./target/release/websearch`.

## Usage — CLI Arguments

| Arg | Env Var | Default | Description |
|---|---|---|---|
| `--profile <PATH>` | `WEBSEARCH_PROFILE` | `$DATA_DIR/websearch-mcp/chrome-profile` | Chrome user data directory |
| `--headless` | `WEBSEARCH_HEADLESS` | `false` | Run without visible browser window |
| `--chrome <PATH>` | `WEBSEARCH_CHROME` | autodetected | Path to Chrome/Chromium executable |
| `--port <PORT>` | — | random free port | Chrome DevTools debugging port |
| `--wait-seconds <N>` | `WEBSEARCH_WAIT` | `4` | Seconds to wait for page render |

## Usage — MCP Integration

The server communicates over **stdio** (stdin/stdout). Configure it in your MCP host's settings file.

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "websearch": {
      "command": "/path/to/websearch",
      "args": ["--wait-seconds", "5"]
    }
  }
}
```

### VS Code (Cline / Continue / similar MCP extensions)

```json
{
  "servers": {
    "websearch": {
      "type": "stdio",
      "command": "/path/to/websearch",
      "args": ["--wait-seconds", "5"]
    }
  }
}
```

The server exposes two tools: `search` and `fetch`:

- **`search`** — Search the web via a pluggable search engine
  - `query` (string, required) — the search query
  - `provider` (string, optional, default `"brave"`) — one of `brave`, `duckduckgo`, `google`
  - Returns: the rendered search results page as Markdown

- **`fetch`** — Fetch any URL and return rendered content as Markdown
  - `url` (string, required) — the URL to fetch (only `http://` and `https://` schemes)
  - Returns: the rendered page content as clean Markdown, with non-content elements (nav, headers, footers, ads) stripped automatically

## Search Providers

| Provider | URL | Notes |
|---|---|---|
| **Brave** (default) | `https://search.brave.com/search` | Minimal ads, no cookie wall, clean Markdown output |
| **DuckDuckGo** | `https://html.duckduckgo.com/html/` | Lightweight HTML-only page, fast to render |
| **Google** | `https://www.google.com/search` | Heaviest page, may trigger bot detection or consent walls |

Select a provider by passing its name in the MCP tool call:

```json
{
  "query": "rust async patterns",
  "provider": "duckduckgo"
}
```

The provider name is matched case-insensitively and supports prefix matching (e.g. `"g"` resolves to `"google"`).

## Architecture

```
src/
├── main.rs              Entrypoint, CLI args, MCP tool wiring
├── browser.rs           Browser lifecycle (launch, shared access, graceful shutdown)
├── registry.rs          Provider registry and name resolution
├── cleanup.rs           HTML/Markdown noise stripping (regex-based)
└── providers/
    ├── mod.rs           SearchProvider trait, shared navigation logic
    ├── brave.rs         Brave search implementation
    ├── duckduckgo.rs    DuckDuckGo search implementation
    └── google.rs        Google search implementation
```

### Key design decisions

- **Persistent browser.** One Chrome instance is launched at server startup and lives for the entire server lifetime. Each search opens a new tab, navigates, waits for the render to settle, extracts HTML, and closes the tab. This avoids the overhead of launching a browser per request.
- **No CSS selectors.** After navigation, the full page HTML is extracted, stripped of noise elements (`<script>`, `<style>`, `<nav>`, `<footer>`, `<iframe>`, tracking anchors, etc.), converted to Markdown via `html-to-markdown-rs`, and then post-processed to remove UI chrome and ad labels. The final Markdown is returned as-is — the LLM parses it naturally.
- **Graceful shutdown.** A `BrowserGuard` (Drop impl) sends SIGTERM, waits 2 seconds, then sends SIGKILL to the Chrome process when the server stops. Stale `SingletonLock` / `SingletonSocket` / `SingletonCookie` files are cleaned on both startup and shutdown.

### Data flow

```
MCP host (LLM)
    │  tool call: search(query, provider)    or    fetch(url)
    ▼
main.rs :: WebSearchServer::search()  /  fetch()
    │
    ├─ [search only] registry.rs :: resolve(provider) → &dyn SearchProvider
    └─ browser.rs :: handle() → Arc<Mutex<Browser>>
            │
            ▼
providers/mod.rs :: navigate_and_get_markdown(browser, url)
    │
    ├─ Open new tab, navigate to search URL
    ├─ Wait for JS render (--wait-seconds)
    ├─ Extract page HTML
    ├─ cleanup.rs :: strip_noise(html)     — remove <script>, <nav>, ads, etc.
    ├─ html-to-markdown-rs :: convert()    — HTML → Markdown
    └─ cleanup.rs :: clean_markdown(md)    — strip remaining UI chrome
         │
         ▼
    Return: clean Markdown string
```

## Development

```bash
# Build
cargo build

# Run with headless browser and longer render wait
cargo run -- --headless --wait-seconds 5

# Debug logging (browser events, navigation, cleanup)
RUST_LOG=debug cargo run -- --headless --wait-seconds 5

# Only websearch-module logs
RUST_LOG=websearch=debug cargo run -- --headless --wait-seconds 5
```

Logs are written to stderr so they do not interfere with the MCP stdio transport.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| "Failed to launch browser" | Chrome/Chromium not found | Install Chrome or pass `--chrome /path/to/chrome` |
| Stale lock file warnings | Previous Chrome session crashed | Handled automatically — the server removes `SingletonLock` / `SingletonSocket` / `SingletonCookie` on startup |
| Empty results returned | Search engine blocking automated access | Try a different provider (e.g. switch from Google to Brave) |
| Navigation timeout | Page is slow to render or blocked | Increase `--wait-seconds` (e.g. `--wait-seconds 10`) |
| Browser not found after server restart | Chrome process from a previous run still alive | Kill it manually: `pkill -f "Google Chrome.*websearch-mcp"` |

## Environment Variables Reference

| Variable | Corresponding Flag |
|---|---|
| `WEBSEARCH_PROFILE` | `--profile` |
| `WEBSEARCH_HEADLESS` | `--headless` |
| `WEBSEARCH_CHROME` | `--chrome` |
| `WEBSEARCH_WAIT` | `--wait-seconds` |

## License

MIT
