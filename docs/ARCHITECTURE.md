# BizClaw Architecture

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Crate Architecture](#2-crate-architecture)
3. [Multi-Provider LLM System](#3-multi-provider-llm-system)
4. [Agent System](#4-agent-system)
5. [Memory Architecture](#5-memory-architecture)
6. [Security & Approval System](#6-security--approval-system)
7. [API Gateway](#7-api-gateway)
8. [Configuration](#8-configuration)
9. [Build & Deployment](#9-build--deployment)

---

## 1. System Overview

BizClaw is a multi-agent orchestration platform with integrated LLM support (local GGUF + cloud providers).

**Key Features:**
- Multi-agent system with parallel task execution
- Local GGUF model inference (llama.cpp)
- Cloud LLM providers (OpenAI, Anthropic, MiniMax, DeepSeek, Gemini, etc.)
- Multi-provider routing with fallback and rate limiting
- Human-in-the-loop approval gates
- Tool system with 40+ built-in tools
- Webhook integration for external systems
- SME workflow automation

---

## 2. Crate Architecture

### Core Crates

| Crate | Description | Dependencies |
|-------|-------------|--------------|
| `bizclaw-core` | Types, config, errors, traits | tokio, serde, thiserror |
| `bizclaw-brain` | Local GGUF inference engine | llama.cpp, mmap2, rayon |
| `bizclaw-providers` | LLM provider implementations | reqwest, async-trait |
| `bizclaw-memory` | 3-tier memory system | tokio |
| `bizclaw-tools` | Built-in tool implementations | varies by tool |
| `bizclaw-security` | Approval gates, audit logging | tokio, chrono |

### Agent Crates

| Crate | Description |
|-------|-------------|
| `bizclaw-agent` | Agent state machine, execution loop |
| `bizclaw-orchestrator` | Multi-agent coordination |
| `bizclaw-scheduler` | Task scheduling and queuing |
| `bizclaw-workflows` | Workflow definitions and execution |

### Integration Crates

| Crate | Description |
|-------|-------------|
| `bizclaw-gateway` | HTTP API server (axum) |
| `bizclaw-channels` | Slack, Discord, Teams integration |
| `bizclaw-webauth` | Authentication providers |
| `bizclaw-mcp` | Model Context Protocol server |
| `bizclaw-platform` | vSphere/VMware integration |
| `bizclaw-knowledge` | Knowledge base management |

---

## 3. Multi-Provider LLM System

### Provider Trait

All LLM providers implement the `Provider` trait from `bizclaw-core/src/traits/provider.rs`:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat(&self, messages: &[Message], tools: &[ToolDefinition], params: &GenerateParams) -> Result<ProviderResponse>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn health_check(&self) -> Result<bool>;
}
```

### Supported Providers

| Provider | Config Name | Auth Style | Extended Thinking |
|----------|-------------|------------|-------------------|
| OpenAI | `openai` | Bearer | OpenAI reasoning effort |
| Anthropic | `anthropic` | Bearer | Claude budget tokens |
| MiniMax | `minimax` | Bearer | No |
| DeepSeek | `deepseek` | Bearer | DeepSeek reasoner |
| Gemini | `gemini` | API Key | No |
| Groq | `groq` | Bearer | No |
| Ollama | `ollama` | None | No |
| Local GGUF | `brain` | None | No |
| Custom Endpoint | `custom:...` | Bearer/None | Varies |

### CloudRouter (Multi-Provider Routing)

Location: `crates/bizclaw-brain/src/cloud_router.rs`

**BrainMode (Operation Modes):**
```rust
pub enum BrainMode {
    CloudFirst,   // Try cloud providers first, fallback to local
    LocalFirst,   // Try local model first, fallback to cloud
    CloudOnly,    // Only use cloud providers
    LocalOnly,    // Only use local GGUF model
}
```

**RoutingStrategy (Provider Selection):**
```rust
pub enum RoutingStrategy {
    RoundRobin,     // Sequential rotation
    LeastLatency,   // Pick fastest provider
    CostAware,      // Prefer cheaper providers
    PriorityBased,  // Use provider priority field
}
```

**Rate Limiting:**
- Token bucket algorithm per provider
- Configurable: `requests_per_minute`, `tokens_per_minute`, `max_concurrent`

**Circuit Breaker States:**
```
Healthy → Degraded → Unhealthy → HalfOpen → Healthy
```

### Provider Registry

Location: `crates/bizclaw-providers/src/provider_registry.rs`

Pre-configured provider endpoints:
- OpenAI: `https://api.openai.com/v1`
- Anthropic: `https://api.anthropic.com/v1`
- MiniMax: `https://api.minimax.io/v1`
- DeepSeek: `https://api.deepseek.com/v1`
- Gemini: `https://generativelanguage.googleapis.com/v1beta`
- Groq: `https://api.groq.com/openai/v1`
- Ollama: `http://localhost:11434/v1`

### Message Role Handling

Different providers have different requirements:

| Provider | System Role | Notes |
|----------|-------------|-------|
| OpenAI | `system` | Standard |
| Anthropic | `system` field | Top-level, supports cache_control |
| MiniMax | `user` | System role converted to user role |
| DeepSeek | `system` | Standard |
| Gemini | `model`/`user` | Different format |

---

## 4. Agent System

### Agent Lifecycle

```
IDLE → PLANNING → EXECUTING → WAITING_APPROVAL → COMPLETED/FAILED
```

### AgentTeam

Location: `crates/bizclaw-orchestrator/src/coordinator.rs`

Multiple agents can work in parallel on different tasks.

### Tool Calling

Agents call tools through the `ToolCall` mechanism:
1. Model returns function call in response
2. ToolExecutor validates and runs tool
3. Tool result appended as tool message
4. Model continues with result

---

## 5. Memory Architecture

### 3-Tier Memory System

Location: `crates/bizclaw-memory/src/`

| Tier | Purpose | TTL |
|------|---------|-----|
| Working Memory | Current conversation | Session |
| Episodic Memory | Recent interactions | 24 hours |
| Semantic Memory | Long-term knowledge | Persistent |

### Brain Workspace

Location: `crates/bizclaw-memory/src/brain_workspace.rs`

Maintains agent context across conversations.

---

## 6. Security & Approval System

### Approval Gates

Location: `crates/bizclaw-security/src/approval.rs`

Certain tools (email, http_request, shell) can require human approval before execution.

**Flow:**
1. Agent calls sensitive tool
2. Action queued as "pending"
3. User notified via dashboard/chat
4. User approves/denies
5. Agent receives result

### Audit Logging

All approval decisions are logged with:
- Timestamp
- Action details
- Caller identity
- Decision (approved/denied)
- Session ID

---

## 7. API Gateway

### Server Architecture

Location: `crates/bizclaw-gateway/src/`

Built with **axum** web framework.

**Routes:**
- `/api/agents/*` - Agent management
- `/api/conversations/*` - Conversation history
- `/api/tools/*` - Tool registry
- `/api/webhooks/*` - Webhook handlers
- `/dashboard` - Web UI

### Database Schema

Location: `crates/bizclaw-db/src/` + SQLite

Tables:
- `agents` - Agent configurations
- `conversations` - Chat history
- `audit_log` - Security audit trail
- `skills` - Available skills
- `tenants` - Multi-tenant isolation

---

## 8. Configuration

### Config File: `~/.bizclaw/config.toml`

```toml
[llm]
provider = "minimax"           # Primary LLM provider
fallback_provider = "deepseek"  # Failover provider
api_key = "sk-..."           # API key (or use env var)

[brain]
threads = 4                  # CPU threads for GGUF
max_tokens = 256             # Max generation tokens
context_length = 2048        # Model context size
temperature = 0.7            # Generation temperature

[autonomy]
approval_required_tools = ["shell", "http_request", "email"]
auto_approve_timeout_secs = 300

[server]
host = "0.0.0.0"
port = 3000
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `MINIMAX_API_KEY` | MiniMax API key |
| `DEEPSEEK_API_KEY` | DeepSeek API key |
| `GEMINI_API_KEY` | Gemini API key |
| `BIZCLAW_API_KEY` | BizClaw auth key |

---

## 9. Build & Deployment

### Build Commands

```bash
# Development build
cargo build

# Release build
cargo build --release

# Single crate
cargo build --release -p bizclaw-brain

# Run server
RUST_LOG=debug cargo run --release -- serve --port 3000
```

### Version

- Current: `1.1.7`
- MSRV: Rust 1.85
- Edition: 2024

### Performance Notes

- Async runtime: tokio with full features
- HTTP client: reqwest with connection pooling
- Database: rusqlite (bundled SQLite)
- Local inference: llama.cpp with SIMD acceleration

---

## Appendix: Error Handling

All errors use `BizClawError` enum:

```rust
pub enum BizClawError {
    Brain(String),
    Provider(String),
    Tool(String),
    Config(String),
    Database(String),
    ApiKeyMissing(String),
    ProviderNotFound(String),
    // ...
}
```

---

*Last updated: 2026-04-14*
