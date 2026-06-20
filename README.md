<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/🌐_WebSearch--MCP-2D3748?style=for-the-badge&logo=rust&logoColor=white">
    <img alt="WebSearch-MCP" src="https://img.shields.io/badge/🌐_WebSearch--MCP-2D3748?style=for-the-badge&logo=rust&logoColor=white">
  </picture>
</p>

<p align="center">
  <em>Real browser. Real JavaScript. Real search results — served as clean Markdown for AI assistants.</em>
</p>

<p align="center">
  <a href="#"><img src="https://img.shields.io/badge/rust-1.80+-de5842?style=flat&logo=rust&logoColor=white" alt="Rust"></a>
  <a href="#"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat" alt="License"></a>
  <a href="#"><img src="https://img.shields.io/badge/platform-macOS%20|%20Linux-lightgrey?style=flat" alt="Platform"></a>
  <a href="#"><img src="https://img.shields.io/badge/MCP-server-7C3AED?style=flat&logo=modelcontextprotocol&logoColor=white" alt="MCP Server"></a>
  <a href="#"><img src="https://img.shields.io/badge/tools-16-059669?style=flat" alt="Tools"></a>
</p>

---

## 📋 Table of Contents

- [Overview](#-overview)
- [Features](#-features)
- [How It Works](#-how-it-works)
- [Prerequisites](#-prerequisites)
- [Installation](#-installation)
- [Configuration](#-configuration)
- [MCP Integration](#-mcp-integration)
- [Tool Reference](#-tool-reference)
- [Search Providers](#-search-providers)
- [Example Workflows](#-example-workflows)
- [Architecture](#-architecture)
- [Development](#-development)
- [Troubleshooting](#-troubleshooting)
- [Environment Variables](#-environment-variables)
- [Contributing](#-contributing)
- [License](#-license)

---

## 🌟 Overview

`websearch-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server that gives AI assistants full browser automation and web search capabilities. Instead of making basic HTTP requests, it controls a **real Chrome/Chromium browser instance** — navigating pages, interacting with elements, waiting for JavaScript to render, and extracting content as clean Markdown.

The server ships with **16 tools** covering two modes:

- **Convenience tools** — `search` and `fetch` for quick one-shot operations
- **Browser interaction tools** — granular DevTools-style control over tabs, navigation, elements, and content

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## ✨ Features

- **🧠 LLM-native output** — Returns clean Markdown instead of raw HTML or structured JSON. The AI parses results naturally.
- **🌍 Real browser rendering** — JavaScript-heavy pages work out of the box via headless Chromium.
- **🖱️ Full interaction** — Click elements, type into inputs, execute JavaScript, navigate history.
- **🗂️ Tab management** — Open, close, focus, and list multiple browser tabs.
- **🔌 Multi-provider search** — Brave (default), DuckDuckGo, and Google. Selectable per-request.
- **📄 Universal fetch** — Fetch any URL as clean Markdown, stripping noise automatically.
- **📸 Screenshots** — Capture visual snapshots as base64-encoded PNG.
- **⚡ Persistent browser** — One Chrome instance for the server lifetime. Tab recovery across restarts.
- **🛡️ Graceful shutdown** — Automatic cleanup of Chrome processes and stale lock files.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🔄 How It Works

```
MCP host (LLM)
    │  tool call
    ▼
websearch-mcp server
    │
    ├─ SessionManager (tab state, navigation, interaction)
    │   ├─ Open tab → navigate to URL
    │   ├─ Wait for JavaScript render (configurable)
    │   ├─ Extract page HTML
    │   ├─ Strip noise: <script>, <style>, <nav>, <footer>, ads
    │   ├─ Convert HTML → Markdown
    │   └─ Post-process: remove remaining UI chrome
    │
    └─ Return: clean Markdown string → LLM understands it naturally
```

**Key differentiator:** By using a real browser, `websearch-mcp` handles JavaScript-heavy pages that HTTP fetchers cannot. By returning Markdown, it lets the LLM parse results naturally — no fragile CSS selectors, no scraping contracts.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## ✅ Prerequisites

- **Rust 1.80+** — required for `std::sync::LazyLock`
- **Chrome or Chromium** installed on the system (autodetected on macOS and Linux)
- **macOS or Linux** — the server uses `pkill`/`pgrep` for browser process management

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 📦 Installation

```bash
git clone https://github.com/devstroop/websearch-mcp.git
cd websearch-mcp
cargo build --release
```

The compiled binary is at `./target/release/websearch`.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## ⚙️ Configuration

All settings can be provided via **CLI flags**, **environment variables**, or both. CLI flags take precedence.

| Flag | Env Var | Default | Description |
|---|---|---|---|
| `--profile <PATH>` | `WEBSEARCH_PROFILE` | `$DATA_DIR/websearch-mcp/chrome-profile` | Chrome user data directory |
| `--headless` | `WEBSEARCH_HEADLESS` | `false` | Run without visible browser window |
| `--chrome <PATH>` | `WEBSEARCH_CHROME` | autodetected | Path to Chrome/Chromium executable |
| `--port <PORT>` | — | random free port | Chrome DevTools debugging port |
| `--wait-seconds <N>` | `WEBSEARCH_WAIT` | `4` | Seconds to wait for page render |

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🔌 MCP Integration

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

### VS Code / Cline / Continue (`.vscode/mcp.json`)

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

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🛠️ Tool Reference

The server exposes **16 MCP tools** organized into four groups.

### High-Level Convenience

| Tool | Parameters | Description |
|------|-----------|-------------|
| **`search`** | `query` (string), `provider?` (string, default `"brave"`) | Search the web and return results as clean Markdown |
| **`fetch`** | `url` (string, `http://`/`https://` only) | Fetch any URL and return rendered content as Markdown |

### Tab Management

| Tool | Parameters | Description |
|------|-----------|-------------|
| **`browser_open`** | `url?` (string), `activate?` (bool, default `true`) | Open a new browser tab, optionally navigate to a URL |
| **`browser_tabs`** | — | List all open tabs with IDs, URLs, and titles |
| **`browser_focus`** | `tab_id` (string) | Switch the active tab by ID |
| **`browser_close`** | `tab_id?` (string) | Close a tab (active tab if omitted) |

### Navigation

| Tool | Parameters | Description |
|------|-----------|-------------|
| **`browser_navigate`** | `url` (string) | Navigate the active tab to a URL |
| **`browser_back`** | — | Go back in browser history |
| **`browser_forward`** | — | Go forward in browser history |
| **`browser_reload`** | — | Reload the current page |

### Interaction

| Tool | Parameters | Description |
|------|-----------|-------------|
| **`browser_click`** | `selector` (string) | Click an element by CSS selector |
| **`browser_type`** | `selector` (string), `text` (string), `submit?` (bool) | Type text into an input, optionally press Enter |

### Content & State

| Tool | Parameters | Description |
|------|-----------|-------------|
| **`browser_get_content`** | — | Get the active tab's content as clean Markdown |
| **`browser_get_html`** | — | Get the raw HTML source |
| **`browser_screenshot`** | `full_page?` (bool) | Capture a screenshot as base64-encoded PNG |
| **`browser_evaluate`** | `script` (string) | Execute JavaScript and return the result as JSON |

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🔍 Search Providers

| Provider | URL | Notes |
|---|---|---|
| **Brave** 🏆 (default) | `https://search.brave.com/search` | Minimal ads, no cookie wall, clean Markdown output |
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

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 💡 Example Workflows

### Quick search

```
search("rust tokio runtime") → clean Markdown of search results
browser_close() → clean up
```

### Multi-step browser interaction

```
1. browser_open("https://github.com")
2. browser_type("input[name=q]", "tokio runtime", submit=true)
3. browser_click("a[href='/tokio-rs/tokio']")
4. browser_get_content() → read the repo page
5. browser_evaluate("document.title") → "tokio-rs/tokio"
6. browser_close() → clean up
```

### Multi-tab research

```
1. browser_open("https://docs.rs/tokio")      → Tab 1
2. browser_open("https://crates.io/crates/tokio") → Tab 2
3. browser_tabs()                               → list both tabs
4. browser_focus(<tab1_id>)                     → switch to docs
5. browser_get_content()                        → read docs
6. browser_focus(<tab2_id>)                     → switch to crates.io
7. browser_get_content()                        → read crate info
8. browser_close(<tab1_id>)                     → close docs tab
9. browser_close(<tab2_id>)                     → close crates tab
```

### Screenshot for debugging

```
browser_open("https://example.com")
browser_screenshot(full_page=true) → base64 PNG for visual verification
browser_close()
```

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🏗️ Architecture

```
src/
├── main.rs                 Binary entrypoint (thin bootstrap)
├── lib.rs                  Library entrypoint (serve function)
├── config.rs               CLI args (clap) + validated Config
├── error.rs                Typed error enum (thiserror)
├── browser.rs              BrowserManager (launch, hold, kill Chrome)
├── session.rs              SessionManager (tabs, navigation, interaction)
├── registry.rs             Provider registry (resolve, list)
├── cleanup/
│   ├── mod.rs              Shared definitions
│   ├── html.rs             HTML noise stripping (regex)
│   └── markdown.rs         Markdown post-processing
├── providers/
│   ├── mod.rs              SearchProvider trait
│   ├── navigate.rs         Standalone navigation utility
│   ├── brave.rs            Brave URL builder
│   ├── google.rs           Google URL builder
│   └── duckduckgo.rs       DuckDuckGo URL builder
└── tools/
    ├── mod.rs              WebSearchServer + 16 MCP tool definitions
    ├── browser_tools.rs    Browser interaction tool handlers
    ├── search.rs           Search tool handler
    └── fetch.rs            Fetch tool handler
```

### Key Design Decisions

- **SessionManager pattern.** All browser state (tabs, active tab, URLs) lives in `SessionManager`. MCP tools lock the session mutex, call the appropriate method, and return the result. This serializes access cleanly.
- **Agent-controlled tab lifecycle.** Tools like `search` and `fetch` open tabs but do NOT auto-close them. The agent decides when to close via `browser_close`. This enables multi-step workflows.
- **No CSS selectors for content.** Pages are converted to Markdown via a cleanup pipeline: HTML noise stripping → `html-to-markdown-rs` conversion → Markdown post-processing. The LLM parses the result naturally.
- **Persistent browser.** One Chrome instance lives for the server lifetime. On startup, existing tabs are recovered via `Browser::pages()`.
- **Graceful shutdown.** A `BrowserGuard` (Drop impl) kills Chrome on exit. Stale lock files are cleaned on startup and shutdown.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🛠️ Development

```bash
# Build
cargo build

# Run with headless browser
cargo run -- --headless --wait-seconds 5

# Debug logging
RUST_LOG=debug cargo run -- --headless --wait-seconds 5

# Only websearch-module logs
RUST_LOG=websearch=debug cargo run -- --headless --wait-seconds 5
```

Logs are written to stderr so they do not interfere with the MCP stdio transport.

### Tests

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

### Release Build

```bash
cargo build --release
# Binary: ./target/release/websearch
```

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🔧 Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| "Failed to launch browser" | Chrome/Chromium not found | Install Chrome or pass `--chrome /path/to/chrome` |
| Stale lock file warnings | Previous Chrome session crashed | Handled automatically — cleaned on startup |
| Empty results returned | Search engine blocking automated access | Try a different provider (e.g. switch from Google to Brave) |
| Navigation timeout | Page is slow to render or blocked | Increase `--wait-seconds` (e.g. `--wait-seconds 10`) |
| Browser not found after restart | Chrome from previous run still alive | Kill it: `pkill -f "Google Chrome.*websearch-mcp"` |
| "no active tab" error | No tab opened yet | Use `browser_open` first, or use `search`/`fetch` which handle tabs internally |

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 📚 Environment Variables

| Variable | Corresponding Flag |
|---|---|
| `WEBSEARCH_PROFILE` | `--profile` |
| `WEBSEARCH_HEADLESS` | `--headless` |
| `WEBSEARCH_CHROME` | `--chrome` |
| `WEBSEARCH_WAIT` | `--wait-seconds` |

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. **Fork** the repository
2. **Create a feature branch**: `git checkout -b feature/my-feature`
3. **Commit your changes**: `git commit -am 'Add my feature'`
4. **Push**: `git push origin feature/my-feature`
5. **Open a Pull Request**

Please make sure your code passes `cargo build`, `cargo test`, and `cargo clippy -- -D warnings` before submitting.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>
