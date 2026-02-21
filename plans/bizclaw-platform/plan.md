# BizClaw Platform - Master Plan

> **Version**: 1.0.0
> **Date**: 2026-02-21
> **Status**: Planning Phase
> **Author**: Bizino AI DEV

---

## 📋 Executive Summary

**BizClaw** là nền tảng AI Assistant Infrastructure mới, viết **100% bằng Rust**, kết hợp:
- **ZeroClaw** (trait-driven AI agent infrastructure) → core architecture & features
- **PicoLM** (minimal LLM inference engine, C11) → viết lại bằng Rust, tích hợp native như **local brain**
- **ZCA-JS** (unofficial Zalo API) → viết lại bằng Rust, tích hợp **Zalo Personal + Zalo OA** channels

### Vision
> *"Zero overhead. Zero cloud dependency. One binary. Local intelligence."*

BizClaw = ZeroClaw's architecture (Rust, trait-driven, secure-by-default) + PicoLM's local inference (mmap, quantization, SIMD) + ZCA-JS's Zalo integration (messaging, groups, business) **unified in one monorepo**, one binary, one ecosystem.

### Giá trị cốt lõi
1. **100% Rust** — Memory safe, fearless concurrency, single static binary
2. **Local-First AI** — Chạy offline hoàn toàn với PicoLM built-in
3. **Hybrid Intelligence** — Local LLM + Cloud LLM seamless switching
4. **Deploy Anywhere** — $10 board → Enterprise server
5. **Trait-Driven** — Swap any component via config, zero code changes
6. **Upstream-Compatible** — Dễ dàng sync features mới từ ZeroClaw
7. **Vietnam-Native** — Zalo Personal + Zalo OA channels, Vietnamese market ready

---

## 🔍 Gap Analysis: ZeroClaw vs BizClaw

| Capability | ZeroClaw | BizClaw (Target) |
|-----------|----------|-------------------|
| Language | Rust ✅ | Rust ✅ |
| Local LLM | Via llama-server/Ollama (external) | **PicoLM built-in** (native Rust) ✅ |
| Offline mode | Partial (needs external LLM server) | **Full offline** (self-contained) ✅ |
| Memory system | SQLite/Postgres/Lucid | All + **Local vector with PicoLM embeddings** ✅ |
| Channels | Telegram, Discord, WhatsApp, CLI | All + **Zalo Personal, Zalo OA** (zca-js rewrite) ✅ |
| Identity | OpenClaw/AIEOS | All + **BizClaw identity** ✅ |
| Vietnam market | N/A | **Vietnamese NLP, channels, payments** ✅ |
| License | Apache-2.0 + MIT | Apache-2.0 + MIT (compatible) ✅ |

---

## 🏗️ Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         BizClaw Binary                               │
│                                                                      │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │   Agent Engine    │  │   Gateway API    │  │   Service Daemon   │ │
│  │   (Core Loop)     │  │   (HTTP/WS)      │  │   (systemd/etc)   │ │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬──────────┘ │
│           │                      │                      │            │
│  ┌────────┴──────────────────────┴──────────────────────┴──────┐    │
│  │                    Trait-Driven Core Layer                    │    │
│  │                                                              │    │
│  │  ┌───────────┐ ┌──────────┐ ┌────────┐ ┌──────────────────┐│    │
│  │  │ Provider   │ │ Channel  │ │ Memory │ │ Security Policy  ││    │
│  │  │ Trait      │ │ Trait    │ │ Trait  │ │ Trait            ││    │
│  │  └─────┬─────┘ └────┬─────┘ └───┬────┘ └────────┬─────────┘│    │
│  │        │             │           │               │          │    │
│  │  ┌─────┴─────┐ ┌────┴────┐ ┌────┴────┐ ┌───────┴────────┐│    │
│  │  │ Tool      │ │Observer │ │ Runtime │ │ Identity       ││    │
│  │  │ Trait     │ │ Trait   │ │ Adapter │ │ Config         ││    │
│  │  └───────────┘ └─────────┘ └─────────┘ └────────────────┘│    │
│  └──────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐    │
│  │                  bizclaw-brain (PicoLM Rust)                  │    │
│  │                                                              │    │
│  │  ┌───────────┐ ┌──────────┐ ┌─────────┐ ┌────────────────┐│    │
│  │  │ GGUF      │ │ mmap     │ │ SIMD    │ │ Quantization   ││    │
│  │  │ Parser    │ │ Engine   │ │ Kernels │ │ Q4_K/Q6_K/etc  ││    │
│  │  └───────────┘ └──────────┘ └─────────┘ └────────────────┘│    │
│  │  ┌───────────┐ ┌──────────┐ ┌─────────┐ ┌────────────────┐│    │
│  │  │ Tokenizer │ │ Attention│ │ Sampler │ │ Grammar JSON   ││    │
│  │  │ BPE       │ │ Flash    │ │ Top-p/k │ │ Constraint     ││    │
│  │  └───────────┘ └──────────┘ └─────────┘ └────────────────┘│    │
│  └──────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### Monorepo Structure

```
bizclaw/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── README.md
├── LICENSE-APACHE
├── LICENSE-MIT
├── config.example.toml
│
├── crates/
│   ├── bizclaw-core/             # Core traits, types, config
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs         # TOML config (serde)
│   │       ├── error.rs          # Unified error types
│   │       ├── traits/
│   │       │   ├── mod.rs
│   │       │   ├── provider.rs   # LLM Provider trait
│   │       │   ├── channel.rs    # Communication Channel trait
│   │       │   ├── memory.rs     # Memory Backend trait
│   │       │   ├── tool.rs       # Tool Execution trait
│   │       │   ├── observer.rs   # Observability trait
│   │       │   ├── runtime.rs    # Runtime Adapter trait
│   │       │   ├── security.rs   # Security Policy trait
│   │       │   ├── identity.rs   # Identity Config trait
│   │       │   └── tunnel.rs     # Tunnel trait
│   │       └── types/
│   │           ├── mod.rs
│   │           ├── message.rs    # Chat messages, roles
│   │           ├── tool_call.rs  # Tool call/response types
│   │           └── model.rs      # Model info types
│   │
│   ├── bizclaw-brain/            # 🧠 PicoLM rewrite in Rust
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Public API
│   │       ├── gguf.rs           # GGUF format parser
│   │       ├── mmap.rs           # Memory-mapped model loading
│   │       ├── model.rs          # LLaMA forward pass
│   │       ├── tensor.rs         # Matrix ops (matmul, rmsnorm, softmax, rope, silu)
│   │       ├── quant.rs          # Quantization kernels (Q4_K, Q6_K, Q3_K, Q2_K)
│   │       ├── simd/
│   │       │   ├── mod.rs
│   │       │   ├── neon.rs       # ARM NEON SIMD intrinsics
│   │       │   ├── sse2.rs       # x86 SSE2 intrinsics
│   │       │   └── avx2.rs       # x86 AVX2 intrinsics (new!)
│   │       ├── tokenizer.rs      # BPE tokenizer
│   │       ├── sampler.rs        # Temperature + Top-p/Top-k sampling
│   │       ├── attention.rs      # Flash Attention (online softmax)
│   │       ├── kv_cache.rs       # FP16 KV Cache + persistence
│   │       ├── grammar.rs        # JSON grammar constraints
│   │       ├── rope.rs           # Rotary Position Embeddings
│   │       └── thread_pool.rs    # Multi-threaded matrix multiply
│   │
│   ├── bizclaw-providers/        # LLM Provider implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── openai.rs         # OpenAI / OpenRouter
│   │       ├── anthropic.rs      # Anthropic Claude
│   │       ├── ollama.rs         # Ollama (local/remote)
│   │       ├── llamacpp.rs       # llama-server
│   │       ├── brain.rs          # 🧠 bizclaw-brain (local PicoLM)
│   │       └── custom.rs         # Custom OpenAI-compatible
│   │
│   ├── bizclaw-channels/         # Communication channels
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── cli.rs            # Interactive CLI
│   │       ├── telegram.rs       # Telegram bot
│   │       ├── discord.rs        # Discord bot
│   │       ├── whatsapp.rs       # WhatsApp Business
│   │       ├── webhook.rs        # Generic webhook
│   │       └── zalo/             # 🇻🇳 Zalo channels (zca-js rewrite)
│   │           ├── mod.rs        # ZaloChannel trait impl
│   │           ├── personal.rs   # Zalo Personal account mode
│   │           ├── official.rs   # Zalo OA (Official Account) mode
│   │           └── client/       # Pure Rust Zalo Web protocol client
│   │               ├── mod.rs    # ZaloClient struct
│   │               ├── auth.rs   # Cookie login, QR login, multi-account
│   │               ├── session.rs # Session mgmt, cookie jar, keep-alive
│   │               ├── crypto.rs # Zalo encryption (AES/RSA reverse-eng)
│   │               ├── messaging.rs # send/receive/forward/delete/sticker
│   │               ├── groups.rs # 20+ group management APIs
│   │               ├── friends.rs # Friend requests, contacts, blocking
│   │               ├── business.rs # Catalog, products (ZBusiness)
│   │               ├── listener.rs # WebSocket event listener (message/reaction/undo/group)
│   │               └── models.rs # 19 data models (Message, Group, User, etc.)
│   │
│   ├── bizclaw-memory/           # Memory backends
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── sqlite.rs         # SQLite backend
│   │       ├── postgres.rs       # PostgreSQL backend
│   │       ├── markdown.rs       # Markdown files
│   │       ├── vector.rs         # Vector search engine
│   │       └── noop.rs           # No-op backend
│   │
│   ├── bizclaw-tools/            # Built-in tools
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── shell.rs          # Shell command execution
│   │       ├── file.rs           # File read/write
│   │       ├── browser.rs        # Browser automation
│   │       └── registry.rs       # Tool registry
│   │
│   ├── bizclaw-security/         # Security policies
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── sandbox.rs        # Sandboxing
│   │       ├── allowlist.rs      # Command/path allowlists
│   │       ├── secrets.rs        # Encrypted secrets
│   │       └── auth.rs           # Auth profiles (OAuth, tokens)
│   │
│   ├── bizclaw-runtime/          # Runtime adapters
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── native.rs         # Native process
│   │       └── docker.rs         # Docker container
│   │
│   ├── bizclaw-gateway/          # HTTP/WS Gateway
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs         # Axum HTTP server
│   │       ├── routes.rs         # API routes
│   │       └── pairing.rs        # Pairing flow
│   │
│   └── bizclaw-agent/            # Agent engine (core loop)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── engine.rs         # Main agent loop
│           ├── context.rs        # Context management
│           ├── skills.rs         # Skills system
│           └── planner.rs        # Task planning
│
├── src/
│   └── main.rs                   # CLI entry point (clap)
│
├── tests/
│   ├── integration/
│   └── benchmarks/
│
├── docs/
│   ├── README.md
│   ├── architecture.md
│   ├── configuration.md
│   ├── providers.md
│   ├── brain.md                  # PicoLM/Brain documentation
│   └── security.md
│
├── scripts/
│   ├── bootstrap.sh              # One-click setup
│   ├── install.sh                # Installer
│   └── download-model.sh         # Model downloader
│
└── .github/
    └── workflows/
        ├── ci.yml                # CI/CD
        └── release.yml           # Release builds
```

---

## 🧠 bizclaw-brain: PicoLM Rust Rewrite Strategy

### Tại sao viết lại bằng Rust thay vì FFI/wrapper?

| Approach | Pros | Cons |
|----------|------|------|
| **FFI wrapper (giữ C)** | Nhanh, ít effort | Unsafe boundaries, 2 languages, build complex |
| **Subprocess (stdin/stdout)** | Simple, isolated | Latency overhead, serialization cost |
| **Rust rewrite** ✅ | Pure Rust, safe, integrated | Nhiều effort hơn, nhưng long-term win |

**Quyết định: Full Rust Rewrite** vì:
1. **Safety**: Rust eliminates memory bugs (buffer overflows, use-after-free)
2. **Integration**: Native function call, không có IPC/serialization overhead
3. **Unified build**: Một `cargo build`, một binary
4. **SIMD**: Rust `std::arch` hỗ trợ NEON/SSE2/AVX2 natively
5. **Async**: Tokio integration cho streaming inference
6. **Testing**: Rust test ecosystem tốt hơn C
7. **Maintainability**: 2,500 lines C → ~3,000-3,500 lines Rust (safe, readable)

### PicoLM → bizclaw-brain Mapping

| PicoLM (C) | bizclaw-brain (Rust) | Notes |
|------------|---------------------|-------|
| `picolm.c` | `lib.rs` + `engine.rs` | Public API, generation loop |
| `model.h/c` | `model.rs` + `gguf.rs` + `mmap.rs` | Split for clarity |
| `tensor.h/c` | `tensor.rs` | Use `ndarray` or manual ops |
| `quant.h/c` | `quant.rs` + `simd/` | Platform-specific SIMD |
| `tokenizer.h/c` | `tokenizer.rs` | BPE implementation |
| `sampler.h/c` | `sampler.rs` | Temperature + Top-p |
| `grammar.h/c` | `grammar.rs` | JSON constraint system |
| N/A | `attention.rs` | Flash Attention extracted |
| N/A | `kv_cache.rs` | KV Cache management |
| N/A | `rope.rs` | RoPE extracted |
| N/A | `thread_pool.rs` | Rayon/custom pool |

### Key Rust Crates cho brain

```toml
[dependencies]
memmap2 = "0.9"         # Memory-mapped files (replaces mmap)
rayon = "1.10"          # Parallel iteration (replaces pthreads)
half = "2.4"            # FP16 type support
byteorder = "1.5"       # Binary parsing (GGUF)
serde = { version = "1", features = ["derive"] }

# SIMD via std::arch - no external dep needed
# #[cfg(target_arch = "aarch64")] → NEON
# #[cfg(target_arch = "x86_64")]  → SSE2/AVX2
```

### Integration API (bizclaw-brain as Provider)

```rust
// crates/bizclaw-brain/src/lib.rs
pub struct BrainEngine {
    model: MmapModel,
    tokenizer: BpeTokenizer,
    kv_cache: KvCache,
    config: BrainConfig,
}

impl BrainEngine {
    /// Load model from GGUF file
    pub fn load(model_path: &Path, config: BrainConfig) -> Result<Self>;

    /// Generate text completion (streaming)
    pub fn generate(&mut self, prompt: &str, params: GenerateParams) -> impl Stream<Item = String>;

    /// Generate with JSON grammar constraint (for tool calling)
    pub fn generate_json(&mut self, prompt: &str, schema: &JsonSchema) -> Result<serde_json::Value>;

    /// Get embeddings (for memory/vector search)
    pub fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

// crates/bizclaw-providers/src/brain.rs
impl Provider for BrainProvider {
    async fn chat(&self, messages: &[Message], tools: &[Tool]) -> Result<Response> {
        let prompt = self.format_chat_template(messages);
        if tools.is_empty() {
            // Regular text generation
            let response = self.engine.lock().await.generate(&prompt, self.params).collect().await;
            Ok(Response::text(response))
        } else {
            // Tool calling with JSON grammar
            let json = self.engine.lock().await.generate_json(&prompt, &tool_schema)?;
            Ok(Response::tool_calls(parse_tool_calls(json)?))
        }
    }
}
```

---

## 🔄 Upstream Sync Strategy (ZeroClaw Compatibility)

### Tại sao cần upstream sync?

ZeroClaw đang phát triển rất nhanh (Harvard/MIT community). BizClaw nên:
1. **Track upstream releases** — Cherry-pick features/security fixes
2. **Maintain API compatibility** — Same trait interfaces
3. **Extend, don't fork** — Add features on top, don't modify core traits

### Sync Architecture

```
ZeroClaw (upstream)
│
├── src/traits/        →  bizclaw-core/src/traits/    (1:1 compatible)
├── src/providers/     →  bizclaw-providers/          (extend with brain)
├── src/channels/      →  bizclaw-channels/           (extend with more)
├── src/memory/        →  bizclaw-memory/             (extend with brain embeddings)
├── src/security/      →  bizclaw-security/           (keep compatible)
└── config.toml format →  config.toml format          (superset)

BizClaw additions:
├── bizclaw-brain/       →  NEW (PicoLM rewrite)
├── providers/brain.rs   →  NEW (local inference provider)
├── channels/zalo/       →  NEW (zca-js rewrite in Rust)
└── Extended config      →  [brain] + [channel.zalo] sections added
```

### Quy trình sync

```
1. Monitor ZeroClaw releases (GitHub Releases/Tags)
2. Compare trait definitions → Update bizclaw-core if changed
3. Port new providers/channels/tools → Add to respective crates
4. Test compatibility → Ensure BizClaw extensions don't break
5. Update version → bizclaw v0.X.Y maps to zeroclaw v0.A.B
```

### Version Mapping

```
bizclaw v0.1.0 → Based on zeroclaw v0.X (current state as of Feb 2026)
bizclaw v0.2.0 → Sync with next zeroclaw release
...
```

---

## 📦 Configuration Design

### BizClaw config.toml (superset of ZeroClaw)

```toml
# ~/.bizclaw/config.toml

# ===== Provider Config (ZeroClaw compatible) =====
api_key = "sk-..."
default_provider = "brain"        # "brain" = local PicoLM 🧠
default_model = "tinyllama-1.1b"
default_temperature = 0.7

# ===== Brain Config (NEW - BizClaw exclusive) =====
[brain]
enabled = true
model_path = "~/.bizclaw/models/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"
threads = 4                       # Number of inference threads
max_tokens = 256                  # Max generation length
context_length = 2048             # Context window size
cache_dir = "~/.bizclaw/cache"    # KV cache persistence
auto_download = true              # Auto-download model if missing
temperature = 0.7
top_p = 0.9
json_mode = false                 # Enable grammar-constrained JSON by default

# Fallback: if brain fails, try cloud provider
[brain.fallback]
provider = "openrouter"
model = "anthropic/claude-sonnet-4-6"

# ===== Memory (ZeroClaw compatible + brain extensions) =====
[memory]
backend = "sqlite"
auto_save = true
embedding_provider = "brain"       # Use bizclaw-brain for embeddings! 🧠
vector_weight = 0.7
keyword_weight = 0.3

# ===== Gateway (ZeroClaw compatible) =====
[gateway]
port = 3000
host = "127.0.0.1"
require_pairing = true

# ===== Security (ZeroClaw compatible) =====
[autonomy]
level = "supervised"
workspace_only = true
allowed_commands = ["git", "npm", "cargo", "ls", "cat", "grep"]
forbidden_paths = ["/etc", "/root", "/proc", "/sys", "~/.ssh"]

# ===== Runtime (ZeroClaw compatible) =====
[runtime]
kind = "native"

# ===== Tunnel (ZeroClaw compatible) =====
[tunnel]
provider = "none"

# ===== Secrets (ZeroClaw compatible) =====
[secrets]
encrypt = true

# ===== Identity (ZeroClaw compatible) =====
[identity]
format = "openclaw"

# ===== Zalo Channel Config (NEW - BizClaw exclusive) =====
[channel.zalo]
enabled = true
mode = "personal"                 # "personal" or "official"

# Personal mode (via zca protocol - reverse-engineered Zalo Web)
[channel.zalo.personal]
cookie_path = "~/.bizclaw/zalo/cookie.json"
imei = ""                          # z_uuid from browser DevTools
user_agent = ""                    # navigator.userAgent from browser
self_listen = false                # Listen to own messages
auto_reconnect = true              # Reconnect on disconnect
reconnect_delay_ms = 5000
proxy = ""                         # Optional: socks5://host:port

# Official Account mode (Zalo OA API - future)
# [channel.zalo.official]
# app_id = ""
# secret_key = ""
# oa_id = ""
# webhook_url = ""

# Rate limiting to avoid account ban
[channel.zalo.rate_limit]
max_messages_per_minute = 20
max_messages_per_hour = 200
cooldown_on_error_ms = 30000

# Allowlist: only respond in these threads (empty = respond to all)
[channel.zalo.allowlist]
user_ids = []                       # Specific user IDs
group_ids = []                      # Specific group IDs
block_strangers = true              # Ignore non-friends
```

---

## 🚀 Phased Implementation Plan

### Phase 1: Foundation (Tuần 1-3) 🔴 CRITICAL

**Mục tiêu**: Scaffold monorepo, core traits, basic CLI

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| Cargo workspace setup | root | P0 | 1d |
| Core traits definition | bizclaw-core | P0 | 2d |
| Config system (TOML) | bizclaw-core | P0 | 1d |
| Error handling | bizclaw-core | P0 | 0.5d |
| Type definitions | bizclaw-core | P0 | 1d |
| CLI entry point (clap) | src/main.rs | P0 | 1d |
| Basic agent engine | bizclaw-agent | P0 | 3d |
| OpenAI provider | bizclaw-providers | P0 | 2d |
| CLI channel | bizclaw-channels | P0 | 1d |
| No-op memory | bizclaw-memory | P0 | 0.5d |

**Milestone**: `bizclaw agent -m "Hello"` works with OpenAI provider

### Phase 2: Brain Engine (Tuần 4-8) 🔴 CRITICAL

**Mục tiêu**: Port PicoLM sang Rust, integrate as local provider

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| GGUF parser | bizclaw-brain | P0 | 3d |
| mmap engine | bizclaw-brain | P0 | 2d |
| BPE tokenizer | bizclaw-brain | P0 | 2d |
| Quantization kernels (Q4_K_M) | bizclaw-brain | P0 | 4d |
| Tensor operations (matmul, rmsnorm, softmax) | bizclaw-brain | P0 | 3d |
| RoPE implementation | bizclaw-brain | P0 | 1d |
| Flash Attention | bizclaw-brain | P0 | 2d |
| KV Cache (FP16) | bizclaw-brain | P0 | 2d |
| Sampler (temperature, top-p) | bizclaw-brain | P1 | 1d |
| JSON grammar constraints | bizclaw-brain | P1 | 2d |
| SIMD: SSE2 kernels | bizclaw-brain | P1 | 2d |
| SIMD: NEON kernels | bizclaw-brain | P1 | 2d |
| Thread pool (rayon) | bizclaw-brain | P1 | 1d |
| KV Cache persistence | bizclaw-brain | P2 | 1d |
| Brain provider integration | bizclaw-providers | P0 | 2d |
| Model download script | scripts/ | P1 | 1d |

**Milestone**: `bizclaw agent --provider brain -m "Hello"` works offline!

### Phase 3: Multi-Provider & Memory (Tuần 9-11) 🟡 HIGH

**Mục tiêu**: Full provider support, persistent memory

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| Anthropic provider | bizclaw-providers | P0 | 2d |
| Ollama provider | bizclaw-providers | P1 | 1d |
| LlamaCpp provider | bizclaw-providers | P1 | 1d |
| Custom provider | bizclaw-providers | P2 | 1d |
| SQLite memory | bizclaw-memory | P0 | 2d |
| Vector search engine | bizclaw-memory | P1 | 3d |
| Brain embeddings integration | bizclaw-memory | P1 | 2d |
| Provider fallback system | bizclaw-providers | P1 | 1d |

**Milestone**: Hybrid mode (brain → cloud fallback) + persistent memory

### Phase 4A: Zalo Channels (Tuần 12-15) 🔴 HIGH — Vietnam Market

**Mục tiêu**: Full Zalo integration (Personal + OA foundation)

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| Zalo encryption reverse-engineering | bizclaw-channels/zalo | P0 | 3-4d |
| Zalo auth (cookie + QR login) | bizclaw-channels/zalo | P0 | 3-4d |
| Zalo session & keep-alive | bizclaw-channels/zalo | P0 | 1-2d |
| Core messaging (send/receive/reply) | bizclaw-channels/zalo | P0 | 3-4d |
| WebSocket event listener | bizclaw-channels/zalo | P0 | 2-3d |
| Group management (20+ APIs) | bizclaw-channels/zalo | P1 | 2-3d |
| Friend management (contacts, block) | bizclaw-channels/zalo | P1 | 1-2d |
| Stickers, reactions, media | bizclaw-channels/zalo | P1 | 2d |
| Business features (catalog, products) | bizclaw-channels/zalo | P2 | 2-3d |
| Zalo Channel trait integration | bizclaw-channels/zalo | P0 | 1-2d |
| Rate limiting (anti-ban) | bizclaw-channels/zalo | P0 | 1d |
| Data models (19 types) | bizclaw-channels/zalo | P0 | 1d |

**Milestone**: Zalo Personal bot chạy offline với brain!

> ⚠️ **Warning**: Zalo Personal API is unofficial. BizClaw includes
> warning dialogs and rate limiting to minimize ban risk.

### Phase 4B: Other Channels & Security (Tuần 16-18) 🟡 HIGH

**Mục tiêu**: Telegram, Discord, security hardening

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| Telegram channel | bizclaw-channels | P0 | 3d |
| Discord channel | bizclaw-channels | P1 | 2d |
| Webhook channel | bizclaw-channels | P1 | 2d |
| Sandbox system | bizclaw-security | P0 | 2d |
| Allowlist system | bizclaw-security | P0 | 1d |
| Secrets encryption | bizclaw-security | P0 | 2d |
| Auth profiles | bizclaw-security | P1 | 2d |

**Milestone**: Telegram + Zalo + Discord bots all running with brain

### Phase 5: Gateway & Services (Tuần 19-21) 🟢 MEDIUM

**Mục tiêu**: HTTP API, service management, tunneling

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| Axum HTTP server | bizclaw-gateway | P0 | 2d |
| Pairing flow | bizclaw-gateway | P0 | 1d |
| API routes | bizclaw-gateway | P0 | 2d |
| systemd integration | bizclaw-runtime | P1 | 1d |
| Docker runtime | bizclaw-runtime | P2 | 2d |
| Tunnel support | bizclaw-core | P2 | 2d |

**Milestone**: Full gateway API, daemonized service

### Phase 6: Tools & Skills (Tuần 22-24) 🟢 MEDIUM

**Mục tiêu**: Tool execution, skills system

| Task | Crate | Priority | Effort |
|------|-------|----------|--------|
| Shell tool | bizclaw-tools | P0 | 2d |
| File tool | bizclaw-tools | P0 | 1d |
| Tool registry | bizclaw-tools | P0 | 1d |
| Skills system | bizclaw-agent | P1 | 3d |
| Open-skills sync | bizclaw-agent | P2 | 2d |
| Browser tool | bizclaw-tools | P2 | 3d |

**Milestone**: Agent có thể sử dụng tools, execute commands

### Phase 7: Polish & Release (Tuần 25-28) 🟢 MEDIUM

**Mục tiêu**: Stabilization, documentation, release

| Task | Priority | Effort |
|------|----------|--------|
| Integration tests | P0 | 3d |
| Zalo channel E2E tests | P0 | 2d |
| Benchmarks (vs ZeroClaw, vs PicoLM) | P1 | 2d |
| Cross-compilation (ARM, RISC-V) | P1 | 2d |
| Documentation | P0 | 3d |
| Zalo setup guide (cookie extraction, QR) | P0 | 1d |
| CI/CD (GitHub Actions) | P0 | 2d |
| Release binaries | P0 | 1d |
| Bootstrap/install scripts | P1 | 1d |
| README, examples | P0 | 2d |
| SIMD: AVX2 kernels | P2 | 2d |

**Milestone**: v0.1.0 Release — Production-ready binary with Zalo support

---

## 📊 Effort Summary

| Phase | Duration | Focus |
|-------|----------|-------|
| Phase 1: Foundation | 3 tuần | Core architecture |
| Phase 2: Brain Engine | 5 tuần | PicoLM Rust rewrite |
| Phase 3: Providers & Memory | 3 tuần | Multi-provider, persistence |
| Phase 4A: **Zalo Channels** | **4 tuần** | **zca-js Rust rewrite, Personal + OA** |
| Phase 4B: Other Channels & Security | 3 tuần | Telegram, Discord, hardening |
| Phase 5: Gateway & Services | 3 tuần | HTTP API, daemon |
| Phase 6: Tools & Skills | 3 tuần | Extensibility |
| Phase 7: Polish & Release | 4 tuần | Quality, docs, release |
| **Total** | **~28 tuần (7 tháng)** | |

### MVP Timeline (Phase 1-2): **8 tuần**
> CLI agent chạy offline với local brain

### Vietnam MVP (Phase 1-2 + 4A): **15 tuần**
> CLI agent + Zalo Personal bot chạy offline

### Usable Product (Phase 1-4B): **18 tuần**
> Multi-channel (Zalo + Telegram + Discord) + hybrid intelligence + security

---

## 🛡️ Risk Assessment

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| SIMD performance gap (Rust vs C) | Medium | Low | Benchmark sớm, fallback to C intrinsics nếu cần |
| GGUF format updates | Low | Medium | Track llama.cpp releases |
| ZeroClaw breaking changes | Medium | Medium | Pin upstream version, test on sync |
| Inference quality regression | High | Low | Same algorithms, extensive testing |
| Cross-compilation issues | Medium | Medium | CI/CD matrix build sớm |
| Model compatibility | Medium | Low | Focus TinyLlama first, expand later |
| **Zalo account ban** | **High** | **Medium** | **Rate limiting, warning system, proxy support** |
| **Zalo protocol changes** | **High** | **Medium** | **Monitor zca-js updates, protocol versioning** |
| **Zalo encryption changes** | **High** | **Low** | **Modular crypto module, easy to update** |

---

## 🎯 Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Brain inference speed | Within 90% of PicoLM C | Benchmark tok/s on same hardware |
| RAM usage (brain) | <50MB (TinyLlama 1.1B) | `/usr/bin/time -l` |
| Binary size | <10MB (static) | `ls -lh target/release/bizclaw` |
| Cold start | <200ms | CLI `time bizclaw agent -m "hi"` |
| Provider switch latency | <50ms | Config hot-reload benchmark |
| Test coverage | >80% | `cargo tarpaulin` |
| Cross-platform | ARM64, x86_64, RISC-V | CI matrix |

---

## 💡 Key Design Decisions

### 1. Monorepo vs Multi-repo
**Decision**: Monorepo (Cargo workspace)
**Rationale**: Single `cargo build`, shared types, easier testing, atomic commits

### 2. Async runtime
**Decision**: Tokio
**Rationale**: De facto standard, excellent ecosystem, brain inference in blocking spawn

### 3. HTTP framework
**Decision**: Axum
**Rationale**: Tokio-native, tower middleware, type-safe routing

### 4. CLI framework
**Decision**: Clap (derive)
**Rationale**: Standard Rust CLI, excellent docs, subcommand support

### 5. Config format
**Decision**: TOML (ZeroClaw compatible)
**Rationale**: Rust ecosystem standard, human-readable, serde support

### 6. Brain integration model
**Decision**: Native library (in-process)
**Rationale**: Zero IPC overhead, shared memory, atomic binary

### 7. SIMD implementation
**Decision**: `std::arch` intrinsics + generic fallback
**Rationale**: No external deps, compile-time platform detection

---

## 📝 Next Steps

1. **Xác nhận plan** → User review & approve
2. **Init workspace** → `cargo init`, setup monorepo
3. **Phase 1**: Core traits → Config → CLI → Basic agent
4. **Phase 2**: Brain engine (tâm điểm dự án)
5. **Phase 4A**: Zalo channels (tâm điểm Vietnam market)
6. **Continuous**: Track ZeroClaw + zca-js releases

---

## 🇻🇳 Zalo Integration: Technical Deep-Dive

### Source Projects

| Component | Source | Language | Strategy |
|-----------|--------|----------|----------|
| Zalo Personal API | [zca-js](https://github.com/RFS-ADRENO/zca-js) | TypeScript | Clean-room Rust rewrite |
| Zalo OA API | [Zalo Developers](https://developers.zalo.me) | REST API | Native Rust client |

### zca-js → BizClaw Mapping (85+ APIs)

| Category | zca-js APIs | bizclaw-zalo module | Priority |
|----------|-------------|---------------------|----------|
| Auth | loginQR, login(cookie), multi-account | `auth.rs` | P0 |
| Messaging | sendMessage, sendSticker, sendVideo, sendVoice, forwardMessage, deleteMessage, undo | `messaging.rs` | P0 |
| Listener | message, reaction, undo, group_event | `listener.rs` | P0 |
| Friends | sendFriendRequest, acceptFriend, blockUser, findUser, getAllFriends | `friends.rs` | P1 |
| Groups | createGroup, addUser, removeUser, changeGroupName, 20+ APIs | `groups.rs` | P1 |
| Stickers | getStickers, getStickersDetail, sendSticker | `messaging.rs` | P1 |
| Business | getBizAccount, createCatalog, createProduct, uploadProductPhoto | `business.rs` | P2 |
| Auto-Reply | createAutoReply, updateAutoReply, getAutoReplyList | `business.rs` | P2 |
| Chat Mgmt | deleteChat, pinConversations, muteChat, archiveChat | `messaging.rs` | P2 |
| Profile | updateProfile, changeAvatar, updateSettings | `auth.rs` | P3 |
| Polls | createPoll, lockPoll, getPollDetail | `groups.rs` | P3 |
| Labels | getLabels, updateLabels | `messaging.rs` | P3 |

### Rust Crates for Zalo Client

```toml
[dependencies]
reqwest = { version = "0.12", features = ["cookies", "json", "socks"] }  # HTTP client + cookies + proxy
tokio-tungstenite = "0.24"      # WebSocket for event listener
aes = "0.8"                       # AES encryption (Zalo protocol)
rsa = "0.9"                       # RSA encryption (Zalo protocol) 
base64 = "0.22"                   # Base64 encoding
sha2 = "0.10"                     # SHA-256 hashing
hmac = "0.12"                     # HMAC signatures
qrcode = "0.14"                   # QR code generation (for QR login)
serde_json = "1"                  # JSON parsing
tokio = { version = "1", features = ["full"] }
tracing = "0.1"                   # Logging
```

### Anti-Ban Safety System

```rust
// Built-in rate limiter for Zalo channel
pub struct ZaloRateLimiter {
    max_per_minute: u32,     // default: 20
    max_per_hour: u32,       // default: 200
    cooldown_on_error: Duration,  // default: 30s
    jitter: bool,            // Random delays between messages
}

impl ZaloChannel {
    async fn send_with_rate_limit(&self, msg: OutgoingMessage) -> Result<()> {
        self.rate_limiter.acquire().await?;
        // Add random jitter (500ms-2000ms) to appear human-like
        let jitter = rand::thread_rng().gen_range(500..2000);
        tokio::time::sleep(Duration::from_millis(jitter)).await;
        self.client.send_message(msg).await
    }
}
```

### Zalo OA (Official Account) — Future Phase

Zalo OA sử dụng **official REST API** (developers.zalo.me), khác với Personal:
- **OAuth 2.0** authentication
- **Webhook** cho inbound messages
- **Official API** → không bị ban
- Sẽ implement ở Phase 5+ khi Personal channel stable

---

> *"Intelligence shouldn't require a data center. BizClaw proves it."*
