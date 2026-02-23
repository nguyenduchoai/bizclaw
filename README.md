# ⚡ BizClaw

> **Hạ tầng AI Agent nhanh, module hoá — viết hoàn toàn bằng Rust.**

BizClaw là nền tảng AI Agent kiến trúc trait-driven, có thể chạy **mọi nơi** — từ Raspberry Pi đến cloud server. Hỗ trợ nhiều LLM provider, kênh giao tiếp, và công cụ thông qua kiến trúc thống nhất, hoán đổi được.

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-89%20passing-brightgreen)]()
[![Crates](https://img.shields.io/badge/crates-14-success)]()
[![LOC](https://img.shields.io/badge/lines-24251-informational)]()

<!-- AUTO-GENERATED STATS — updated 2026-02-23 @ 3d15ca5 -->

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
| **🔌 3 Providers** | OpenAI, Anthropic, Ollama, llama.cpp, Brain, Gemini, DeepSeek, Groq |
| **💬 18 Channels** | CLI, Zalo Personal, Telegram, Discord, Email (IMAP/SMTP), Webhook |
| **🏢 Multi-Tenant** | Admin Platform, JWT Auth, Tenant Manager, Pairing Codes, Audit Log |
| **🌐 Web Dashboard** | Chat UI (VI/EN), WebSocket real-time, LobsterBoard-inspired widgets |
| **🛠️ 15 Tools** | Shell, File, Web Search, Group Summarizer, Calendar, Document Reader |
| **🔗 MCP** | Model Context Protocol client — kết nối MCP servers bên ngoài |
| **🔒 Security** | Command allowlist, AES-256, HMAC-SHA256, JWT + bcrypt |
| **💾 Memory** | SQLite + RAG-style retrieval, keyword search, relevance scoring |
| **⚡ SIMD** | ARM NEON, x86 SSE2/AVX2 auto-dispatch |
| **🤖 24 Agent Templates** | 6 danh mục, system prompt chuyên sâu, cài 1 click |
| **💾 Persistence** | agents.json auto-save/restore, không mất data khi restart |

### 🤖 Agent Gallery — 24 Mẫu sẵn có

Cài đặt agent chuyên biệt chỉ 1 click. Mỗi agent có **system prompt** tích hợp skill chuyên sâu:

| Danh mục | Agent | Skill tích hợp |
|----------|-------|---------------|
| 🧠 **Nền tảng** | Navigator, Optimizer, Architect, Manager | Tìm kiếm công cụ, tối ưu token, thiết kế hệ thống, quản lý dự án |
| ✍️ **Sáng tạo** | Brainstorm, Copywriter, Editor, Social Media | Ý tưởng sáng tạo, content marketing, biên tập, social media |
| 💻 **Lập trình** | Full-Stack Dev, Debugger, QA Tester, Security Auditor | React/Next.js, debug systematic, TDD, OWASP Top 10 |
| 🎨 **Thiết kế** | UI Designer, Data Viz, Brand Designer | 8px grid, D3.js/Chart.js, visual identity |
| 📈 **Marketing** | Growth Hacker, SEO Expert, Pricing Strategist, Ad Specialist | AARRR, SEO audit, SaaS pricing, Google/Meta Ads |
| 📋 **Năng suất** | Doc Writer, Data Analyst, Translator, Email Pro | API docs, SQL/Excel, VI↔EN, email sequences |

### 💰 Tiết kiệm token — Mỗi Agent chọn Provider riêng

Mỗi agent có thể chọn nhà cung cấp & mô hình riêng → tiết kiệm 60-80% chi phí:

\`\`\`
Agent "Translator"     → Ollama/qwen2.5 (miễn phí, local)
Agent "Full-Stack Dev" → Anthropic/claude-sonnet-4 (mạnh)
Agent "Social Media"   → Gemini/flash (nhanh, rẻ)
Agent "Editor"         → DeepSeek/chat (giá tốt)
\`\`\`

### 👥 Group Chat — Đội ngũ Agent cộng tác

Tạo nhóm nhiều agent cùng nhà cung cấp khác nhau làm việc cùng lúc. Gửi 1 câu hỏi → tất cả agent trong nhóm phản hồi theo chuyên môn.

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
| `bizclaw-providers` | 3 LLM providers | ✅ |
| `bizclaw-channels` | 18 channels | ✅ |
| `bizclaw-memory` | SQLite + RAG retrieval | ✅ |
| `bizclaw-tools` | 15 native tools | ✅ |
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
| **Lines of Code** | ~24251 |
| **Tests** | 89 passing |
| **Providers** | 3 |
| **Channels** | 18 |
| **Tools** | 15 + MCP |
| **Binary Size** | bizclaw 11M, platform 7.1M |
| **Last Updated** | 2026-02-23 (3d15ca5) |

---

## 🇬🇧 English

### Features

- **🧠 Brain Engine** — Local LLaMA inference via GGUF with SIMD
- **🔌 3 Providers** — OpenAI, Anthropic, Ollama, llama.cpp, Brain, Gemini, DeepSeek, Groq
- **💬 18 Channels** — CLI, Zalo, Telegram, Discord, Email, Webhook
- **🔗 MCP Support** — Connect any MCP server for unlimited tools
- **🏢 Multi-Tenant Platform** — Admin dashboard, JWT auth, tenant lifecycle
- **🌐 Web Dashboard** — Bilingual (VI/EN), real-time WebSocket chat
- **🛠️ 15 Tools** — Shell, File, Web Search, Calendar, Summarizer, DocReader
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

**BizClaw** v0.2.0 — *AI nhanh, mọi nơi. / Fast AI, everywhere.*
