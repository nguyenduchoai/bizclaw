# OpenClaw Analysis Report

> **Source**: https://github.com/openclaw/openclaw
> **Language**: TypeScript (84.4%), Swift (11.6%), Kotlin (1.5%)
> **Runtime**: Node.js ≥22
> **License**: Custom
> **Contributors**: 712
> **Releases**: 48

## 1. OpenClaw là gì?

OpenClaw là **nguồn gốc** (upstream) của cả hệ sinh thái *Claw:
- **OpenClaw** (TypeScript/Node.js) → Original, full-featured, 712 contributors
- **ZeroClaw** (Rust) → Rewrite nhẹ hơn, focused on performance
- **PicoClaw** (Go) → Ultra-lightweight cho $10 boards
- **PicoLM** (C) → Local inference engine cho PicoClaw

## 2. Kiến trúc cốt lõi OpenClaw

```
Channels (15+ channels)
  │
  ▼
┌───────────────────────────────┐
│          Gateway              │
│     (WS Control Plane)       │
│  ws://127.0.0.1:18789        │
└──────────────┬────────────────┘
               │
   ├─ Pi Agent (RPC)         ← core agent loop
   ├─ CLI (openclaw …)       ← command-line
   ├─ WebChat UI              ← web interface
   ├─ macOS/iOS/Android apps  ← companion apps
   └─ Nodes (devices)        ← device-local actions
```

### Key Subsystems:

1. **Gateway** (WS Control Plane)
   - Single WebSocket server (port 18789)
   - Sessions, presence, config, cron, webhooks
   - Control UI + WebChat served directly
   - Tailscale Serve/Funnel integration

2. **Agent** (Pi Agent Runtime)
   - RPC mode with tool streaming and block streaming
   - Session model: main (direct), group isolation, activation modes
   - Agent loop: receive → think → tool_call → respond

3. **Channels** (15 channels!)
   - WhatsApp (Baileys), Telegram (grammY), Slack (Bolt), Discord (discord.js)
   - Google Chat, Signal (signal-cli), BlueBubbles (iMessage)
   - Microsoft Teams, Matrix, WebChat
   - **Zalo** (extension), **Zalo Personal** (extension)
   - macOS/iOS/Android nodes

4. **Tools**
   - Browser control (CDP/Chrome)
   - Canvas (A2UI push/reset)
   - Nodes (camera, screen record, location)
   - Cron + wakeups + webhooks
   - Gmail Pub/Sub
   - Skills platform (ClawHub)

5. **Security**
   - DM pairing (dmPolicy="pairing")
   - Allowlist per channel
   - Docker sandboxing for non-main sessions
   - Tool allowlist/denylist

6. **Configuration**
   - `~/.openclaw/openclaw.json` (JSON format)
   - Model + defaults + channel configs
   - Workspace: `~/.openclaw/workspace/`
   - Skills: `~/.openclaw/workspace/skills/<skill>/SKILL.md`
   - Prompt files: AGENTS.md, SOUL.md, TOOLS.md

7. **Multi-agent**
   - Multi-agent routing per channel/account/peer
   - Agent-to-agent communication (sessions_* tools)
   - Session isolation per workspace

## 3. So sánh OpenClaw vs ZeroClaw

| Feature | OpenClaw | ZeroClaw |
|---------|----------|----------|
| Language | TypeScript (Node.js) | Rust |
| Binary size | ~390MB (Node.js runtime) | <5MB (static binary) |
| RAM usage | ~200-400MB | <5MB |
| Channels | 15 | 5 (Telegram, Discord, WhatsApp, CLI, webhook) |
| Has Zalo | ✅ Extension | ❌ |
| Local LLM | ❌ (cloud only) | Via external llama-server |
| Gateway UI | ✅ WebChat + Control UI | Basic |
| Mobile apps | ✅ iOS/Android nodes | ❌ |
| Voice | ✅ Voice Wake + Talk Mode | ❌ |
| Browser tool | ✅ CDP Chrome | Via Playwright |
| Sessions | ✅ Multi-agent routing | Basic |
| Security | ✅ DM pairing, sandbox | ✅ Comprehensive |
| Skills | ✅ ClawHub registry | ✅ open-skills |

## 4. Cốt lõi cần giữ cho BizClaw IoT

### PHẢI CÓ (Core — 512MB Pi):
1. **Gateway WS Control Plane** — Single WS server, nhỏ gọn
2. **Agent Loop** — receive → think → tool_call → respond
3. **Session Model** — main + group isolation
4. **Channel abstraction** — trait-driven, pluggable
5. **Tool system** — shell, file, extensible
6. **Security** — DM pairing, allowlist, sandbox
7. **Config** — TOML (simpler than JSON)
8. **CLI surface** — onboard, agent, channel, doctor

### NÊN CÓ (Fit 512MB Pi):
1. **Local Brain** (PicoLM) — UNIQUE selling point
2. **Zalo channels** — Vietnam market
3. **Skills system** — workspace skills
4. **Chat commands** — /status, /new, /reset, /compact
5. **Model failover** — brain → cloud fallback
6. **Presence + typing indicators**
7. **WebChat** — lightweight web UI

### KHÔNG CẦN (quá nặng cho Pi):
1. ❌ Canvas/A2UI — macOS only, heavy
2. ❌ Voice Wake/Talk Mode — require ElevenLabs
3. ❌ Browser control — Chrome too heavy for Pi
4. ❌ macOS/iOS/Android apps — companion apps
5. ❌ Gmail Pub/Sub — enterprise feature
6. ❌ Tailscale integration — optional
7. ❌ Docker sandboxing — Pi lacks resources

## 5. Memory Budget cho Pi 512MB

```
Total RAM:          512 MB
OS + services:     ~100 MB
BizClaw binary:      <5 MB  (Rust static binary)
Brain engine:       ~45 MB  (TinyLlama 1.1B Q4_K_M, mmap)
SQLite memory:       <5 MB
Zalo client:        <10 MB  (HTTP + WS connections)
Gateway WS:          <5 MB
Available:         ~340 MB  (headroom for context, other channels)
```

## 6. Kết luận

BizClaw IoT cần giữ **6 subsystem cốt lõi** từ OpenClaw:
1. Gateway (WS control plane)
2. Agent loop (RPC mode)
3. Session model
4. Channel abstraction
5. Tool system
6. Security (pairing + allowlist)

Và thêm **2 USP** mà OpenClaw không có:
1. **Local Brain** (PicoLM Rust) — offline AI
2. **Native Zalo** (Rust) — no Node.js dependency
