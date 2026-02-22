# ⚡ BizClaw

> **Hạ tầng AI Agent nhanh, module hoá — viết hoàn toàn bằng Rust.**

BizClaw là nền tảng AI Agent kiến trúc trait-driven, có thể chạy **mọi nơi** — từ Raspberry Pi đến cloud server. Hỗ trợ nhiều LLM provider, kênh giao tiếp, và công cụ thông qua kiến trúc thống nhất, hoán đổi được.

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-71%20passing-brightgreen)]()
[![Crates](https://img.shields.io/badge/crates-14-success)]()
[![LOC](https://img.shields.io/badge/lines-17553-informational)]()

<!-- AUTO-GENERATED STATS — updated 2026-02-22 @ 60c9626 -->

---

## 🇻🇳 Tiếng Việt

### 🚀 100% Tự Host — Không phụ thuộc Cloud

- **100% Độc lập:** Clone về là chạy — laptop, VPS, hay Raspberry Pi. Không token khoá, không telemetry.
- **Dữ liệu nội bộ:** Chat history, API Keys mã hoá AES-256 lưu local.
- **Offline AI:** Brain Engine chạy LLM offline (Llama, DeepSeek) — tối ưu cho 512MB RAM.

### 🎯 Tính năng

| Hạng mục | Chi tiết |
|----------|----------|
| **🧠 Brain Engine** | LLaMA inference: GGUF, mmap, quantization, Flash Attention, FP16 KV Cache |
| **🔌 10 Providers** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, OpenRouter, Ollama, llama.cpp, Brain, Custom |
| **💬 17 Channels** | CLI, Zalo Personal, Telegram, Discord, Email (IMAP/SMTP), Webhook |
| **🏢 Multi-Tenant** | Admin Platform, JWT Auth, Tenant Manager, Pairing Codes, Audit Log |
| **🌐 Web Dashboard** | Chat UI (VI/EN), WebSocket real-time, LobsterBoard-inspired widgets |
| **🛠️ 6 Tools** | Shell, File, Web Search, Group Summarizer, Calendar, Document Reader |
| **🔗 MCP** | Model Context Protocol client — kết nối MCP servers bên ngoài |
| **🔒 Security** | Command allowlist, AES-256, HMAC-SHA256, JWT + bcrypt |
| **💾 Memory** | SQLite + RAG-style retrieval, keyword search, relevance scoring |
| **⚡ SIMD** | ARM NEON, x86 SSE2/AVX2 auto-dispatch |

### 🏗️ Kiến trúc

```
┌─────────────────────────────────────────────────┐
│                 bizclaw (CLI)                     │
│          ┌──────────────────┐                     │
│          │  bizclaw-agent   │ ← RAG Memory        │
│          │  Multi-round     │   + Tool Calling     │
│          │  Tool Calling    │   (max 3 rounds)     │
│          └──────┬───────────┘                     │
│    ┌────────────┼─────────────┐                   │
│    ▼            ▼             ▼                   │
│ Providers    Channels       Tools                 │
│                             ├── Native Tools      │
│                             └── MCP Client        │
│                                  ↕ JSON-RPC       │
│                             MCP Servers (外部)     │
│    ┌────────────┼─────────────┐                   │
│    ▼            ▼             ▼                   │
│ Memory       Security      Gateway               │
│          ┌──────────────────┐                     │
│          │  bizclaw-brain   │                     │
│          │  GGUF + SIMD     │                     │
│          └──────────────────┘                     │
└─────────────────────────────────────────────────┘
```

### 🚀 Bắt đầu nhanh

```bash
# Clone và build
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
cargo build --release

# Cài đặt (wizard tương tác)
./target/release/bizclaw init

# Chat ngay
./target/release/bizclaw agent --interactive

# Mở Web Dashboard
./target/release/bizclaw serve
```

### 🏢 Chế độ triển khai

| Mode | Binary | Use Case |
|------|--------|----------|
| **Standalone** | `bizclaw` only | 1 bot, cá nhân, test |
| **Platform** | `bizclaw` + `bizclaw-platform` | Nhiều bots, agency, production |

### 🔗 MCP (Model Context Protocol) Support

BizClaw hỗ trợ kết nối **MCP Servers** — mở rộng tools không giới hạn mà không cần rebuild:

```toml
# config.toml
[[mcp_servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]

[[mcp_servers]]
name = "database"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres"]
env = { DATABASE_URL = "postgresql://..." }
```

- Tools từ MCP servers xuất hiện tự động trong Agent
- Hỗ trợ JSON-RPC 2.0 qua stdio
- Mỗi tenant có thể cấu hình MCP servers riêng

### 🧠 Ollama / Brain Engine — Shared Models

Ollama models được **dùng chung** giữa tất cả tenants. Pull 1 lần → tất cả dùng được.

```bash
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull tinyllama     # ~1.5GB
ollama pull llama3.2      # ~3.8GB
```

### 📦 Crate Map

| Crate | Mô tả | Status |
|-------|--------|--------|
| `bizclaw-core` | Traits, types, config, errors | ✅ |
| `bizclaw-brain` | GGUF inference + SIMD | ✅ |
| `bizclaw-providers` | 9 LLM providers | ✅ |
| `bizclaw-channels` | 17 channels | ✅ |
| `bizclaw-memory` | SQLite + RAG retrieval | ✅ |
| `bizclaw-tools` | 6 native tools | ✅ |
| `bizclaw-mcp` | MCP client (JSON-RPC) | ✅ |
| `bizclaw-security` | AES-256, Sandbox | ✅ |
| `bizclaw-agent` | Agent loop + tool calling | ✅ |
| `bizclaw-gateway` | Axum HTTP + WS + Dashboard | ✅ |
| `bizclaw-runtime` | Process adapters | ✅ |
| `bizclaw-platform` | Multi-tenant admin | ✅ |

### 📊 Stats

| Metric | Value |
|--------|-------|
| **Language** | 100% Rust |
| **Crates** | 14 |
| **Lines of Code** | ~17553 |
| **Tests** | 71 passing |
| **Providers** | 10 |
| **Channels** | 17 |
| **Tools** | 6 + MCP |
| **Binary Size** | bizclaw 11M, platform 7.1M |
| **Last Updated** | 2026-02-22 |

---

## 🇬🇧 English

### Features

- **🧠 Brain Engine** — Local LLaMA inference via GGUF with SIMD
- **🔌 10 Providers** — OpenAI, Anthropic, Gemini, DeepSeek, Groq, OpenRouter, Ollama, llama.cpp, Brain, Custom
- **💬 17 Channels** — CLI, Zalo, Telegram, Discord, Email, Webhook
- **🔗 MCP Support** — Connect any MCP server for unlimited tools
- **🏢 Multi-Tenant Platform** — Admin dashboard, JWT auth, tenant lifecycle
- **🌐 Web Dashboard** — Bilingual (VI/EN), real-time WebSocket chat
- **🛠️ 6 Tools** — Shell, File, Web Search, Calendar, Summarizer, DocReader
- **🔒 Security** — AES-256, Command allowlists, sandbox, HMAC-SHA256
- **💾 RAG Memory** — SQLite with keyword search and relevance scoring

### Quick Start

```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw && cargo build --release
./target/release/bizclaw init
./target/release/bizclaw agent --interactive
```

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

**BizClaw** v0.1.0 — *AI nhanh, mọi nơi. / Fast AI, everywhere.*
