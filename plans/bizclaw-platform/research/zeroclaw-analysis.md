# ZeroClaw Analysis Report

> **Source**: https://github.com/openagen/zeroclaw
> **Language**: Rust (96.4%), Shell (2.1%), Python (1.1%)
> **License**: Dual (Apache-2.0 + MIT)

## 1. Overview

ZeroClaw là một **AI assistant infrastructure** viết 100% bằng Rust, được thiết kế ultra-lightweight:
- **Mục tiêu**: Zero overhead, zero compromise - deploy anywhere, swap anything
- **Built by**: Harvard, MIT, Sundai.Club communities
- **Triết lý**: Trait-driven architecture, secure-by-default, pluggable everything

## 2. Key Metrics

| Metric | ZeroClaw | So sánh OpenClaw |
|--------|----------|-----------------|
| RAM | <5MB | ~390MB (Node.js) |
| Binary size | Static single binary | N/A |
| Cold start | Near-instant | Slow (Node.js boot) |
| Target hardware | $10 boards | Mac mini ($700+) |

## 3. Architecture (Trait-driven)

Mọi subsystem đều là 1 **trait** → swap implementations via config, zero code changes:

```
┌─ Provider Trait        → OpenAI, Anthropic, Ollama, LlamaCpp, Custom
├─ Channel Trait         → Telegram, Discord, WhatsApp, CLI, Gateway/Webhook
├─ Memory Trait          → SQLite, Lucid, PostgreSQL, Markdown, None
├─ Tool Trait            → Shell, File, Browser, ComposIO, Custom
├─ Observer Trait        → Logging, Metrics
├─ RuntimeAdapter Trait  → Native, Docker
├─ SecurityPolicy Trait  → Sandboxing, Allowlists
├─ IdentityConfig Trait  → OpenClaw format, AIEOS JSON
└─ Tunnel Trait          → None, Cloudflare, Tailscale, Ngrok, Custom
```

## 4. Core Features

### 4.1 Providers (LLM Backends)
- **OpenRouter** (default)
- **Anthropic** (Claude)
- **OpenAI** / **OpenAI Codex** (ChatGPT subscription OAuth)
- **Ollama** (local/remote)
- **LlamaCpp** (`llama-server` as first-class local provider)
- **Custom OpenAI-compatible** (`custom:https://your-api.com`)
- **Custom Anthropic-compatible** (`anthropic-custom:https://your-api.com`)

### 4.2 Channels (Communication)
- **CLI** (interactive terminal)
- **Telegram** (bot)
- **Discord** (bot)
- **WhatsApp** (Business API)
- **Gateway API** (HTTP webhook `/health`, `/pair`, `/webhook`)

### 4.3 Memory System (Full-Stack Search Engine)
- **Zero external dependencies** (no Pinecone, no Elasticsearch, no LangChain)
- Backends: SQLite, Lucid, PostgreSQL, Markdown, None
- Embedding: None, OpenAI, Custom
- Hybrid search: vector_weight + keyword_weight
- Auto save/recall/manage via tools

### 4.4 Security
- Pairing code required on first connect
- Strict sandboxing
- Channel allowlists (deny-by-default)
- Workspace scoping
- Autonomy levels: readonly, supervised, full
- Forbidden paths
- Allowed commands whitelist
- Secrets encryption at rest

### 4.5 Runtime
- **Native** (default)
- **Docker** (containerized shell execution)
  - Memory limit, CPU limit, read-only rootfs
  - Network isolation

### 4.6 Identity System (AIEOS)
- OpenClaw format (markdown files)
- AIEOS format (JSON - AI Entity Object Specification)

### 4.7 Subscription Auth
- Multi-account, encrypted at rest
- OpenAI Codex OAuth (device code flow + browser flow)
- Anthropic setup-token
- Profile management (`<provider>:<profile_name>`)

### 4.8 Service Management
- systemd / OpenRC auto-detect
- install/start/stop/status/uninstall
- Daemon mode
- Cron scheduling (list/add/add-at/add-every/once/remove/update/pause/resume)

### 4.9 Browser Automation
- agent_browser (default)
- rust-native (WebDriver)
- computer_use (sidecar HTTP)
- Domain allowlist

### 4.10 Open-Skills
- Community skills sync (opt-in)
- Prompt injection mode: full or compact

### 4.11 Composio Integration
- 1000+ OAuth apps
- Encrypted API key storage

### 4.12 Python Companion (zeroclaw-tools)
- LangGraph-based tool calling
- For providers with inconsistent tool calling

## 5. Configuration

File: `~/.zeroclaw/config.toml`
- Hot-reload: provider, model, temperature, api_key, api_url, reliability
- TOML format

## 6. Commands

```
onboard        → First-time setup
agent          → Run AI agent
gateway        → HTTP API server
daemon         → Background service
service        → Install/manage system service
doctor         → Health check
status         → System status
cron           → Scheduled tasks
models         → List/refresh available models
providers      → List providers
channel        → Channel management
integrations   → Integration status
skills         → Skills management
migrate        → Migration tools
completions    → Shell completions (bash/fish/zsh/powershell/elvish)
hardware       → Hardware info
peripheral     → Peripheral management
auth           → Authentication management
```

## 7. Key Takeaways cho BizClaw

1. **Trait-driven architecture** là core pattern → phải giữ nguyên trong Rust
2. **Config-driven** (TOML) → hot-reload, zero code changes
3. **Memory system tự xây** → không phụ thuộc external services
4. **Security-first** → sandbox, allowlist, encryption
5. **Multi-channel** → Telegram, Discord, WhatsApp, CLI, Webhook
6. **Provider-agnostic** → swap LLM backends dễ dàng
7. **Service management** → systemd/OpenRC integration
8. **Single binary** → deploy anywhere
