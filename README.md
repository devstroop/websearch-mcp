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
  <a href="#"><img src="https://img.shields.io/badge/providers-Brave%20|%20DuckDuckGo%20|%20Google-059669?style=flat" alt="Providers"></a>
</p>

---

## 📋 Table of Contents

- [Overview](#-overview)
- [Features](#-features)
- [How It Works](#-how-it-works)
- [Prerequisites](#-prerequisites)
- [Installation](#-installation)
- [Configuration](#-configuration)
- [Usage — CLI Arguments](#-usage--cli-arguments)
- [Usage — MCP Integration](#-usage--mcp-integration)
- [Search Providers](#-search-providers)
- [Architecture](#-architecture)
- [Development](#-development)
- [Troubleshooting](#-troubleshooting)
- [Environment Variables Reference](#-environment-variables-reference)
- [Contributing](#-contributing)
- [License](#-license)

---

## 🌟 Overview

`websearch-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server that provides real web search capabilities to AI assistants. Instead of making basic HTTP requests or parsing RSS feeds, it controls a **real Chrome/Chromium browser instance** — navigating to search engines, waiting for JavaScript to render, extracting the resulting HTML, and converting it to clean Markdown. This approach means the LLM receives the search results page *as rendered*, including content that only appears after JS execution.

The server ships with **three search engine providers**: **Brave**, **DuckDuckGo**, and **Google**. The caller selects the provider by name in each MCP tool call.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## ✨ Features

- **🧠 LLM-native output** — Returns clean Markdown instead of raw HTML or structured JSON. The AI parses results naturally, no fragile CSS selectors or scraping contracts.
- **🌍 Real browser rendering** — JavaScript-heavy pages that basic HTTP fetchers cannot handle work out of the box via headless Chromium.
- **🔌 Multi-provider search** — Brave (default, recommended), DuckDuckGo, and Google. Selectable per-request.
- **📄 Universal fetch** — Fetch and render any URL as clean Markdown, stripping nav bars, headers, footers, and ads automatically.
- **⚡ Persistent browser** — One Chrome instance lives for the server lifetime. Each request opens a new tab, renders, extracts, and closes it — no browser launch overhead.
- **🛡️ Graceful shutdown** — Automatic cleanup of Chrome processes and stale `SingletonLock` / `SingletonSocket` files on exit.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🔄 How It Works

```
MCP host (LLM)
    │  tool call: search(query, provider)    or    fetch(url)
    ▼
websearch-mcp server
    │
    ├─ Resolve provider (Brave / DuckDuckGo / Google)
    ├─ Open new browser tab → navigate to URL
    ├─ Wait for JavaScript render (configurable)
    ├─ Extract page HTML
    ├─ Strip noise: <script>, <style>, <nav>, <footer>, <iframe>, ads
    ├─ Convert HTML → Markdown
    └─ Post-process: remove remaining UI chrome
         │
         ▼
    Return: clean Markdown string → LLM understands it naturally
```

**Key differentiator:** By using a real browser, `websearch-mcp` handles JavaScript-heavy pages that basic HTTP fetchers cannot. And by returning Markdown instead of structured JSON or raw HTML, it lets the LLM parse results *naturally* — no fragile CSS selectors, no scraping contracts, just clean text the model already understands.

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
git clone <repo>
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

## 🚀 Usage — CLI Arguments

Run the server directly for testing:

```bash
# Default mode (visible browser window)
cargo run -- --wait-seconds 5

# Headless mode (no visible window)
cargo run -- --headless --wait-seconds 5

# Debug logging
RUST_LOG=debug cargo run -- --headless --wait-seconds 5
```

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🔌 Usage — MCP Integration

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

### Available Tools

The server exposes two MCP tools:

| Tool | Parameters | Returns |
|---|---|---|
| **`search`** | `query` (string, required), `provider` (string, optional, default `"brave"`) | Rendered search results page as Markdown |
| **`fetch`** | `url` (string, required, `http://` or `https://` only) | Clean page content as Markdown, with nav/headers/footers/ads stripped |

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

## 🏗️ Architecture

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

### Key Design Decisions

- **Persistent browser.** One Chrome instance is launched at server startup and lives for the entire server lifetime. Each search opens a new tab, navigates, waits for the render to settle, extracts HTML, and closes the tab. This avoids the overhead of launching a browser per request.
- **No CSS selectors.** After navigation, the full page HTML is extracted, stripped of noise elements (`<script>`, `<style>`, `<nav>`, `<footer>`, `<iframe>`, tracking anchors, etc.), converted to Markdown via `html-to-markdown-rs`, and then post-processed to remove UI chrome and ad labels. The final Markdown is returned as-is — the LLM parses it naturally.
- **Graceful shutdown.** A `BrowserGuard` (Drop impl) sends SIGTERM, waits 2 seconds, then sends SIGKILL to the Chrome process when the server stops. Stale `SingletonLock` / `SingletonSocket` / `SingletonCookie` files are cleaned on both startup and shutdown.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 🛠️ Development

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
| Stale lock file warnings | Previous Chrome session crashed | Handled automatically — the server removes `SingletonLock` / `SingletonSocket` / `SingletonCookie` on startup |
| Empty results returned | Search engine blocking automated access | Try a different provider (e.g. switch from Google to Brave) |
| Navigation timeout | Page is slow to render or blocked | Increase `--wait-seconds` (e.g. `--wait-seconds 10`) |
| Browser not found after server restart | Chrome process from a previous run still alive | Kill it manually: `pkill -f "Google Chrome.*websearch-mcp"` |

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 📚 Environment Variables Reference

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

Please make sure your code passes `cargo build` and `cargo test` before submitting.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>

---

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

<p align="right"><a href="#-table-of-contents">⬆ back to top</a></p>
