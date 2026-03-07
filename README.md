# ⚡ BizClaw — Trợ lý AI Cá Nhân cho Doanh Nghiệp

<p align="center">
  <img src="docs/images/hero-banner.png" alt="BizClaw — AI Agent bỏ túi" width="800">
</p>

<p align="center">
  <strong>AI Agent riêng — chạy trên thiết bị của bạn. Dữ liệu 100% thuộc về bạn.</strong><br>
  Raspberry Pi ($0) • Android (24/7) • Laptop / Mini PC
</p>

> **BizClaw** là nền tảng AI Agent self-hosted, viết hoàn toàn bằng Rust. Chạy trên bất kỳ thiết bị nào từ 512MB RAM — từ Raspberry Pi bỏ túi đến laptop cá nhân. Không cần cloud, không cần server.

[![Rust](https://img.shields.io/badge/Rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-342%20passing-brightgreen)]()
[![Version](https://img.shields.io/badge/version-v0.3.1-purple)]()
[![Website](https://img.shields.io/badge/🌐_Website-bizclaw.vn-blue)](https://bizclaw.vn)
[![Facebook](https://img.shields.io/badge/📘_Fanpage-bizclaw.vn-1877F2?logo=facebook)](https://www.facebook.com/bizclaw.vn)

---

## 🎯 BizClaw dành cho ai?

| Đối tượng | Lợi ích |
|-----------|---------|
| 🧑‍💼 **Doanh nhân nhỏ** | AI trả lời khách hàng 24/7 qua Zalo, Telegram, Email |
| 💻 **Developer** | Self-hosted AI agent với 16 providers, 13 tools, MCP support |
| 🏪 **Cửa hàng / Quán** | AI hỗ trợ báo giá, lịch hẹn — chạy trên Raspberry Pi $0/tháng |
| 📱 **Người dùng Android** | Agent tự post Facebook, reply Messenger — offline 100% |

> 🔒 **Không telemetry. Không tracking. Không tạo tài khoản trên server trung gian.** Dữ liệu chat, API Keys mã hoá AES-256 trên ổ cứng của bạn.

---

## 🚀 Cài đặt — 3 cách

### Cách 1: Build từ Source (Khuyên dùng)

```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw && cargo build --release

# Cài đặt (wizard tương tác — chọn provider, API key)
./target/release/bizclaw init

# Chạy agent + dashboard
./target/release/bizclaw serve
# → Dashboard tại http://localhost:3000
```

### Cách 2: Docker Standalone

```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw && docker-compose -f docker-compose.standalone.yml up -d
# → Dashboard tại http://localhost:3000
```

### Cách 3: One-Click Install (VPS / Raspberry Pi)

```bash
curl -sSL https://bizclaw.vn/install.sh | sudo bash
# → Dashboard tại http://<IP>:3000
```

> 💡 **Chỉ cần 1 file `config.toml`** — không cần PostgreSQL, không cần Nginx, không cần domain.
> Database sử dụng SQLite embedded, mọi thứ chạy trong 1 binary duy nhất.

---

## ✨ Tính năng chính

| Hạng mục | Chi tiết |
|----------|----------|
| **🔌 16 Providers** | OpenAI, Anthropic, Gemini, DeepSeek, Groq, OpenRouter, Together, MiniMax, xAI (Grok), Mistral, BytePlus ModelArk, Ollama, llama.cpp, Brain Engine, CLIProxy, vLLM |
| **💬 9 Channels** | CLI, Telegram, Discord, Email (IMAP/SMTP), Webhook, WhatsApp, Zalo (Personal + Official Account) |
| **🛠️ 13 Tools** | Shell, File, Edit File, Glob, Grep, Web Search, HTTP, Config Manager, Execute Code (9 ngôn ngữ), Plan, Group Summarizer, Calendar, Doc Reader |
| **🔗 MCP** | Model Context Protocol — kết nối MCP servers bên ngoài, mở rộng tools không giới hạn |
| **🧠 Brain Engine** | GGUF inference offline: mmap, quantization, Flash Attention, SIMD (ARM NEON, x86 AVX2) |
| **🤖 Multi-Agent** | Tạo nhiều agent với vai trò khác nhau, gán vào kênh. Orchestrator quản lý N agents |
| **📚 Knowledge RAG** | Hybrid search (FTS5 + Vector cosine similarity). Upload tài liệu → AI tự trả lời |
| **🖐️ Autonomous Hands** | Agent chạy background 24/7 — Research, Analytics, Content, Monitoring, Security. Tự retry, tự report |
| **🔄 Workflows** | Tạo workflow multi-step (Sequential, FanOut, Collect, Conditional, Loop, Transform). Chạy pipeline AI tự động |
| **⏰ Scheduler** | Cron, interval, one-time tasks. Agent tự chạy background, gửi kết quả qua Telegram/Zalo/Email/Webhook |
| **🎨 51 Agent Templates** | Gallery 13 danh mục nghiệp vụ: HR, Sales, Finance, Marketing, Legal, IT... cài 1 click |
| **🌐 Web Dashboard** | 20+ trang UI (VI/EN), WebSocket real-time, Full CRUD, Dark/Light mode |
| **🔒 Bảo mật** | AES-256, Command allowlist, Runtime sandbox, HMAC-SHA256, rate limiting |

---

## 🖐️ Autonomous Hands — Nhân viên AI ca đêm

> Agent tự chạy, tự retry, tự báo cáo qua Telegram/Zalo/Email — không cần ai hỏi!

| Hand | Nhiệm vụ | Phases |
|------|-----------|--------|
| 🔍 **Research Hand** | Thu thập thông tin, phân tích, tạo báo cáo | Gather → Analyze → Report |
| 📊 **Analytics Hand** | Thống kê, xử lý trends, report hàng ngày | Collect → Process → Report |
| ✍️ **Content Hand** | Sáng tạo nội dung, self-review, kiểm duyệt | Ideate → Create → Review |
| 🛡️ **Monitor Hand** | Giám sát hệ thống, cảnh báo anomaly | Check → Alert |
| 🔄 **Sync Hand** | Đồng bộ dữ liệu giữa các hệ thống | Fetch → Reconcile → Push |
| 📣 **Outreach Hand** | Soạn tin outreach, review, gửi đa kênh | Prepare → Review → Send |
| 🔒 **Security Hand** | Quét bảo mật, phân tích lỗ hổng | Scan → Analyze → Report |

```
  🕐 Cron: 0 */6 * * *  →  🤚 Hand chạy tự động
                                │
                    ┌───────────┼───────────┐
                    ▼           ▼           ▼
              Phase 1       Phase 2     Phase 3
              GATHER        ANALYZE     REPORT
                    │           │           │
                    └───────────┼───────────┘
                                ▼
                    📡 Notify → Telegram / Zalo / Email
```

---

## 🔄 Workflows — Pipeline AI Tự Động

> Kéo thả workflow multi-step, mỗi step gán 1 agent role. Output step trước → Input step sau.

**6 loại Step:**

| Type | Icon | Mô tả |
|------|------|--------|
| **Sequential** | ➡️ | Steps chạy tuần tự |
| **FanOut** | 🔀 | Multiple steps chạy song song |
| **Collect** | 📥 | Gom kết quả (All/Best/Vote/Merge) |
| **Conditional** | 🔀 | If/else branching |
| **Loop** | 🔁 | Lặp lại đến khi đạt điều kiện |
| **Transform** | ✨ | Biến đổi template |

**Ví dụ Workflow "Content Pipeline":**
```
  Input: "Viết blog về AI"
    │
    ├─ Step 1: Draft (Writer)     → Viết bản nháp
    ├─ Step 2: Review (Editor)    → Chỉnh sửa, góp ý
    └─ Step 3: Polish (Proofreader) → Hoàn thiện, kiểm lỗi
    │
    ▼
  Output: Bài viết hoàn chỉnh
```

**6 Template mẫu có sẵn:** Content Pipeline, Expert Consensus, Quality Gate, Research Pipeline, Translation Pipeline, Code Review Pipeline.

---

## 🤖 Multi-Agent System

> Tạo đội ngũ agent với vai trò khác nhau — mỗi agent có provider/model/prompt riêng.

```
  ┌─────────────────── Orchestrator ───────────────────┐
  │                                                     │
  │  🧑‍💼 Agent "Sales"     │ Ollama/qwen3    │ Zalo    │
  │  📊 Agent "Analytics"  │ DeepSeek/chat   │ Email   │
  │  ✍️ Agent "Content"    │ Gemini/flash    │ Telegram│
  │  ⚖️ Agent "Legal"      │ Claude          │ Web     │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

- **Gán Agent ↔ Channel**: Mỗi agent gán vào 1+ kênh (Telegram, Zalo, Discord, Webhook, Web)
- **Provider riêng**: Mỗi agent chọn provider/model tối ưu cho vai trò
- **Group Chat**: Nhiều agent cộng tác giải quyết 1 vấn đề

---

## 🍓📱💻 Chạy trên mọi thiết bị

| Thiết bị | Chi phí | Use Case |
|----------|---------|----------|
| 🍓 **Raspberry Pi** | **$0/tháng** | Doanh nghiệp nhỏ, cá nhân — binary 12MB, 512MB RAM |
| 📱 **Android** | **$0/tháng** | Agent bỏ túi, điều khiển Facebook/Zalo — 24/7 |
| 💻 **Laptop / Mini PC** | **$0/tháng** | Agent mạnh mẽ, chạy nhiều model cùng lúc |

```
  Cùng 1 codebase Rust →  cargo build  →  chạy trên thiết bị bạn chọn
         │
   ┌─────┼──────────────┐
   ▼     ▼              ▼
  🍓 Pi  📱 Android      💻 Laptop
  $0     $0             $0
  1 agent Agent bỏ túi   Multi-agent
  Offline 24/7           Brain Engine
```

---

## 🧠 Ollama — Chạy AI Miễn Phí

> Không cần API key. Chạy model AI trên thiết bị, 100% miễn phí.

```bash
# Cài Ollama (1 lệnh)
curl -fsSL https://ollama.ai/install.sh | sh

# Pull model theo tài nguyên
ollama pull qwen3:0.6b    # 500MB — Pi 4, VPS $5
ollama pull qwen3          # 4.7GB — laptop, VPS 4GB+
ollama pull llama3.2       # 3.8GB — phổ biến nhất
```

| Thiết bị | RAM | Model khuyên dùng |
|----------|-----|-------------------|
| 🍓 Raspberry Pi 4 | 2-4GB | `qwen3:0.6b`, `tinyllama` |
| 💻 Laptop | 8GB+ | `qwen3`, `llama3.2` |
| 📱 Android | 4GB+ | GGUF via llama.cpp |

---

## 💰 Mỗi Agent chọn Provider riêng — Tiết kiệm 60-80%

> Thay vì dùng 1 provider đắt tiền cho mọi việc, hãy tối ưu theo từng vai trò:

```
  Agent           │  Provider             │  Chi phí      │  Lý do
  ────────────────┼───────────────────────┼───────────────┼─────────
  Dịch thuật      │  Ollama/qwen3         │  $0 (local)   │  Free
  Full-Stack Dev  │  Anthropic/claude     │  $$$          │  Mạnh
  Social Media    │  Gemini/flash         │  $            │  Nhanh
  Kế toán         │  DeepSeek/chat        │  $$           │  Giá tốt
  Helpdesk        │  Groq/llama-3.3-70b   │  $            │  Nhanh
  Nội bộ          │  Brain Engine         │  $0 (offline) │  Bảo mật
```

---

## 📱 Android Agent — Không chỉ chat, mà ĐIỀU KHIỂN

<p align="center">
  <img src="docs/images/android-agent-platform.png" alt="BizClaw Android" width="500">
</p>

<p align="center">
  <a href="releases/bizclaw-android-debug.apk"><b>📥 Tải xuống APK (Bản Test / Debug)</b></a><br>
  <i>Chạy trực tiếp mô hình AI trên thiết bị — Không chờ đợi, Không cần server</i>
</p>

| Mode | Mô tả |
|------|--------|
| 📱 LOCAL | llama.cpp on-device, AI điều khiển apps, $0, 100% offline |
| 🌐 REMOTE | Kết nối agent từ xa, chat & điều khiển |
| 🔀 HYBRID | Engine local + agent cloud cùng lúc |

**20 Device Tools:**

| Category | Tools |
|----------|-------|
| 📱 Social | `facebook_post`, `facebook_comment`, `messenger_reply`, `zalo_send` |
| 🖥️ Screen | `screen_read`, `screen_click`, `screen_type`, `screen_tap`, `screen_swipe` |
| 🔧 System | `open_app`, `open_url`, `device_info`, `press_back`, `press_home`, `notifications` |

> Tất cả chạy 100% offline. Không cần server, không API key.

---

## 🔗 MCP Support

```toml
# config.toml — mở rộng tools bằng MCP servers
[[mcp_servers]]
name = "pageindex"
command = "npx"
args = ["-y", "@pageindex/mcp"]

[[mcp_servers]]
name = "github"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
```

---

## 🌐 Web Dashboard — 20+ Trang UI

> Dashboard Preact/HTM build-less, WebSocket real-time, hỗ trợ VI/EN, Dark/Light mode.

| Trang | Chức năng |
|-------|-----------|
| 📊 **Dashboard** | Tổng quan hệ thống: clock, uptime, provider, SIMD, quick actions |
| 💬 **WebChat** | Chat trực tiếp với AI, markdown, code highlighting, streaming |
| 🤖 **AI Agent** | CRUD multi-agent, gán provider/model/channel cho từng agent |
| 📚 **Knowledge** | Upload tài liệu, quản lý RAG, xoá document |
| 📡 **Channels** | Kết nối Telegram, Zalo, Discord, Webhook — trạng thái real-time |
| ⚙️ **Settings** | Cấu hình agent, identity, safety rules |
| 🔌 **Providers** | Kiểm tra 16 providers, trạng thái kết nối, models available |
| 🛠️ **Tools** | 13 native tools + MCP tools — enable/disable từng tool |
| 🔗 **MCP Servers** | Quản lý MCP server connections |
| 🖐️ **Autonomous Hands** | Tạo/sửa/xoá Hands, monitor runs, cost tracking |
| 🔄 **Workflows** | Tạo/sửa/chạy workflow multi-step, 6 template có sẵn |
| 🎯 **Skills Market** | Browse 10+ skills, install/uninstall |
| 🎨 **Gallery** | 51 agent templates theo 13 ngành |
| ⏰ **Scheduler** | Lên lịch task, cron/interval, retry config |
| 📈 **LLM Traces** | Xem lịch sử request/response AI |
| 💰 **Cost Tracking** | Theo dõi chi phí API theo agent |
| ⚡ **Activity Feed** | Realtime feed: messages, events, errors |
| 🔀 **Orchestration** | Delegate task giữa agents, pipeline view |
| 📖 **Wiki & Guide** | Hướng dẫn sử dụng tích hợp trong dashboard |
| 📁 **Config File** | Xem/sửa `config.toml` trực tiếp |

---

## 🏗️ Kiến trúc

```
┌──────────────────────────────────────────────────────┐
│          bizclaw (Gateway + Dashboard)                │
│  ┌────────────────────────────────────────────┐       │
│  │ Axum HTTP + WebSocket + Dashboard UI (20+) │       │
│  │ SQLite gateway.db (embedded)               │       │
│  └────────────────────┬───────────────────────┘       │
│    ┌──────────────────┼──────────────────┐            │
│    ▼                  ▼                  ▼            │
│  bizclaw-agent      bizclaw-agent      bizclaw-agent  │
│  (Orchestrator manages N agents)                      │
│    ┌──────────────────┼──────────────────┐            │
│    ▼                  ▼                  ▼            │
│  16 Providers      9 Channels       13 Tools + MCP    │
│    ▼                  ▼                  ▼            │
│  Memory            Hands             Knowledge        │
│  (SQLite+FTS5)    (Autonomous)      (RAG+Vector)      │
│    ▼                  ▼                  ▼            │
│  Workflows         Scheduler         Security         │
│  (6 step types)   (Cron+Retry)      (AES+Sandbox)     │
│    ▼                                                  │
│  Brain Engine (GGUF+SIMD) — offline inference         │
└──────────────────────────────────────────────────────┘
```

---

## 📦 Crate Map

| Crate | Mô tả | Status |
|-------|--------|--------|
| `bizclaw-core` | Traits, types, config, errors | ✅ |
| `bizclaw-brain` | GGUF inference + SIMD | ✅ |
| `bizclaw-providers` | 16 LLM providers | ✅ |
| `bizclaw-channels` | 9 channel types (incl. Zalo OA) | ✅ |
| `bizclaw-memory` | SQLite + FTS5, Brain workspace | ✅ |
| `bizclaw-tools` | 13 native tools + MCP bridge | ✅ |
| `bizclaw-mcp` | MCP client (JSON-RPC 2.0) | ✅ |
| `bizclaw-security` | AES-256, Sandbox, Allowlists | ✅ |
| `bizclaw-agent` | Think-Act-Observe loop | ✅ |
| `bizclaw-gateway` | HTTP + WS + Dashboard (20+ pages) | ✅ |
| `bizclaw-knowledge` | Knowledge RAG (FTS5 + Vector) | ✅ |
| `bizclaw-scheduler` | Scheduled tasks + Retry + Notifications | ✅ |
| `bizclaw-runtime` | Agent runtime lifecycle | ✅ |
| `bizclaw-hands` | Autonomous Hands (7 types) | ✅ |
| `bizclaw-workflows` | Workflow orchestration (6 step types) | ✅ |
| `bizclaw-skills` | Agent skill modules | ✅ |
| `bizclaw-ffi` | Android FFI layer | ✅ |

---

## ☁️ Muốn dùng bản Cloud?

> Nếu bạn không muốn tự cài đặt, hãy đăng ký **BizClaw Cloud** — chúng tôi lo hạ tầng, bạn chỉ cần cấu hình AI.

| | Self-Hosted (Repo này) | ☁️ Cloud |
|--|--|--|
| **Cài đặt** | Tự cài trên thiết bị | Không cần cài — dùng ngay |
| **Hạ tầng** | Bạn quản lý | Chúng tôi quản lý |
| **Cập nhật** | Tự pull & build | Tự động cập nhật |
| **Hỗ trợ** | Community (GitHub) | Hỗ trợ ưu tiên |
| **Giá** | **Miễn phí** | Theo gói |

👉 **Tìm hiểu thêm tại [bizclaw.vn](https://bizclaw.vn)** — tab "Cloud"

---

## 📊 Stats

| Metric | Value |
|--------|-------|
| **Language** | 100% Rust + Kotlin (Android) |
| **Crates** | 17 |
| **Lines of Code** | ~50,000+ |
| **Tests** | 342 passing |
| **Binary Size** | bizclaw ~12MB |
| **Dashboard Pages** | 20+ |
| **Agent Templates** | 51 |
| **Last Updated** | 2026-03-06 |

---

## 🇬🇧 English

### What is BizClaw?

BizClaw is a **self-hosted AI Agent platform** built entirely in Rust. Run AI agents on your own device — no cloud, no third-party servers. Your data stays with you.

### Quick Start

```bash
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw && cargo build --release
./target/release/bizclaw init
./target/release/bizclaw serve
# Open http://localhost:3000
```

### Key Features

- **16 AI Providers** — OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama, and more
- **9 Channels** — CLI, Telegram, Discord, Email, Webhook, WhatsApp, Zalo (Personal + OA)
- **13 Tools** + MCP support for unlimited extensions
- **Multi-Agent** — Create teams of agents with different roles, assign to channels
- **Autonomous Hands** — Background agents that run 24/7: Research, Content, Analytics, Monitoring, Security
- **Workflows** — Multi-step AI pipelines: Sequential, FanOut, Collect, Loop, Conditional, Transform
- **51 Agent Templates** — Pre-built for HR, Sales, Finance, Marketing, Legal, IT
- **Knowledge RAG** — Hybrid FTS5 + Vector search. Upload docs, AI answers from your content
- **Scheduler** — Cron, interval, one-time tasks with retry and multi-channel notification (Telegram, Zalo OA, Email)
- **20+ Dashboard Pages** — Full CRUD UI with VI/EN, WebSocket real-time, Dark/Light mode
- **Android Agent** — On-device LLM with 20 device tools, runs 100% offline
- **AES-256 Security** — Encrypted credentials, command allowlists, runtime sandbox

> ☁️ **Want the hosted version?** Visit [bizclaw.vn](https://bizclaw.vn) — Cloud tab.

---

## 🔗 Links

| | |
|--|--|
| 🌐 **Website** | [https://bizclaw.vn](https://bizclaw.vn) |
| 📘 **Fanpage** | [https://www.facebook.com/bizclaw.vn](https://www.facebook.com/bizclaw.vn) |
| 💻 **GitHub** | [https://github.com/nguyenduchoai/bizclaw](https://github.com/nguyenduchoai/bizclaw) |

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

**BizClaw** v0.3.1 — *AI riêng, chạy mọi nơi. / Your own AI, runs everywhere.*
