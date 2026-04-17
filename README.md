# BizClaw

**Open-source AI agent platform that works alongside humans to amplify productivity.**

One binary. 18+ LLM providers. 9 channels. Hybrid RAG. Multi-agent orchestration. Built in Rust.

<p align="center">
  <a href="#-bizclaw-cloud">BizClaw Cloud</a> (SaaS) · <a href="#-bizclaw-single-tenant">BizClaw</a> (Self-Hosted)
</p>

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-743%20passing-brightgreen)]()
[![Version](https://img.shields.io/badge/version-v1.1.7-purple)]()
[![Website](https://img.shields.io/badge/🌐_bizclaw.vn-blue)](https://bizclaw.vn)

---

## Why BizClaw

AI should work **with** you, not replace you. BizClaw connects your messaging channels to AI agents that handle the repetitive work — answering customers, creating content, scheduling posts, monitoring dashboards — so you focus on decisions that matter.

- **No LLM hosting required** — bring your own API keys from any of 18+ providers
- **Works where your team already is** — Zalo, Telegram, Discord, Slack, Email, Web, and more
- **Runs on modest hardware** — ~13 MB binary, < 1s startup, works on a $5 VPS
- **Open source** — MIT licensed, fork and customize freely

---

## Core Features

| Category | Details |
|----------|---------|
| **LLM Providers (18+)** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, OpenRouter, MiniMax, xAI (Grok), Mistral, BytePlus ModelArk, Cohere, Perplexity, DashScope, Together, and any OpenAI-compatible API |
| **Channels (9)** | Zalo (Personal + OA), Telegram, Discord, Slack, Email (IMAP/SMTP), WhatsApp, Webhook, Web Chat |
| **Built-in Tools (35+)** | Browser automation (stealth), social posting, database queries, voice transcription (Whisper), shell exec, file operations, HTTP client, planning |
| **MCP Ecosystem** | Model Context Protocol — connect 1000+ external tools via [MCP Hub](https://github.com/modelcontextprotocol/servers) |
| **Knowledge RAG** | Hybrid search (FTS5 + vector), multi-model embedding, folder watcher, nudge system |
| **Multi-Agent** | Orchestrate agent teams with isolated roles — sequential, fan-out, conditional, and loop workflows |
| **Autonomous Hands** | Background agents running 24/7 — research, analytics, content creation, monitoring, outreach |
| **Workflows (23+)** | Pre-built templates: Content Pipeline, Expert Consensus, Research Pipeline, Code Review, AI Slides |
| **Security** | AES-256 vault, RBAC 4-tier, prompt injection scanner (8 patterns, 80+ keywords), SSRF protection, audit trail, command allowlisting |
| **Observability** | Prometheus metrics (`/metrics`), Grafana-ready dashboards, per-provider LLM call tracing |

---

## 🌩 BizClaw Cloud

> Managed SaaS — no server required, start in under 5 minutes.

```bash
# Sign up at https://bizclaw.vn

# Or via CLI
npm install -g @bizclaw/cli
bizclaw login
bizclaw init
```

| | |
|---|---|
| **Deployment** | Fully managed (hosted) |
| **Data** | Encrypted at rest (AES-256) |
| **Setup** | < 5 minutes |
| **Maintenance** | Managed by BizClaw team |
| **Support** | 24/7 |

---

## 🏠 BizClaw (Single-Tenant)

> Self-hosted — your data stays on your infrastructure, 100% on-premise.

### Install

```bash
# From source
git clone https://github.com/nguyenduchoai/bizclaw.git && cd bizclaw
cargo build --release
./target/release/bizclaw-desktop

# Docker
docker-compose -f docker-compose.standalone.yml up -d

# Remote access
./target/release/bizclaw serve --tunnel
```

| Platform | Binary | Size |
|----------|--------|------|
| 🍎 macOS | `bizclaw-desktop` | ~13 MB |
| 🪟 Windows | `bizclaw-desktop.exe` | ~12 MB |
| 🐧 Linux | `bizclaw-desktop` | ~12 MB |

### Web Dashboard

20+ pages, Vietnamese & English, dark/light mode. Built-in at `http://localhost:3000`.

| | |
|---|---|
| **Deployment** | VPS / Local / Docker |
| **Data** | 100% on-premise |
| **Setup** | < 10 minutes |
| **Maintenance** | Self-managed |
| **Support** | Community + Enterprise |

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    BizClaw Platform                       │
├──────────────────────────────────────────────────────────┤
│  Gateway (Axum)  ·  Scheduler (Cron)  ·  Hands (24/7)   │
├──────────────────────────────────────────────────────────┤
│  Channels (9)    ·  Multi-Agent       ·  Workflows (23+) │
├──────────────────────────────────────────────────────────┤
│  Knowledge RAG (FTS5 + Vector)  ·  MCP Ecosystem         │
├──────────────────────────────────────────────────────────┤
│  AI Providers (18+)                                      │
│  OpenAI · Anthropic · Gemini · DeepSeek · Groq           │
│  MiniMax · xAI · Mistral · OpenRouter · ...              │
└──────────────────────────────────────────────────────────┘
```

### Source Layout

```
bizclaw/
├── src/              # Rust core — gateway, agents, channels, providers
├── crates/           # Internal crates — tools, RAG, vault, workflows
├── dashboard/        # Web UI (Preact, 20+ pages)
├── migrations/       # SQLite schema migrations
├── deploy/           # Docker & VPS deployment configs
├── docs/             # Architecture, API reference, guides
├── android/          # Android interaction client
└── training/         # BizClaw Academy materials
```

---

## Multi-Agent Orchestration

Each agent runs with its own identity, LLM provider, tools, and context. Define teams in TOML:

```toml
[[agents]]
id = "researcher"
model = "gemini/gemini-2.0-flash"
tools = ["web_search", "web_fetch", "browser"]

[[agents]]
id = "writer"
model = "anthropic/claude-sonnet-4-20250514"
tools = ["file_write", "social_post"]

[[agents]]
id = "support"
model = "openai/gpt-4o"
tools = ["zalo", "telegram", "email"]
```

Orchestration modes: **Sequential** (chain) · **Fan-Out** (parallel) · **Conditional** (routing) · **Loop** (iterative)

---

## MCP Integration

Connect any MCP-compatible server for unlimited extensibility:

```toml
[[mcp_servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/data"]
```

**1000+ tools** available at [MCP Hub](https://github.com/modelcontextprotocol/servers).

---

## Autonomous Hands

Background agents that run 24/7, auto-retry on failure, and self-report:

| Hand | Purpose |
|------|---------|
| 🔍 Research | Gather intelligence, analyze trends |
| 📊 Analytics | Process data, generate reports |
| ✍️ Content | Create and self-review content |
| 🛡️ Monitor | System health, alert on anomalies |
| 🔄 Sync | Cross-channel data synchronization |
| 📣 Outreach | Draft and send multi-channel messages |

---

## Security

| Feature | Description |
|---------|-------------|
| AES-256 Vault | API keys encrypted at rest |
| RBAC (4-tier) | Admin → Manager → User → Viewer |
| Prompt Injection Scanner | 8 detection patterns, 80+ keywords (EN/VI/CN) |
| SSRF Protection | IPv4 + IPv6 validation |
| Audit Trail | Every action logged |
| Rate Limiting | Per-IP protection |
| Command Allowlist | Sandboxed shell execution |

---

## Documentation

| Resource | Link |
|----------|------|
| Full Documentation | [docs/](docs/) |
| SME Quickstart | [docs/sme-quickstart.md](docs/sme-quickstart.md) |
| Architecture | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| API Reference | [docs/api/](docs/api/) |
| Agent Templates | [gallery/](gallery/) — 51 templates |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

---

## Development

```bash
# Build from source
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=bizclaw=debug cargo run

# Docker (production)
docker-compose -f docker-compose.prod.yml up -d
```

Requirements: **Rust 1.80+**, macOS / Linux / Windows.

---

## Contributing

Pull requests welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting.

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Commit with clear messages
4. Open a Pull Request

---

## License

[MIT License](LICENSE) — free for commercial and non-commercial use.

---

<p align="center">
  <strong>Built with ❤️ for humans who want to do more with less.</strong><br>
  <a href="https://bizclaw.vn">bizclaw.vn</a> · <a href="https://facebook.com/bizclaw.vn">Facebook</a> · <a href="mailto:support@bizclaw.vn">Support</a>
</p>
