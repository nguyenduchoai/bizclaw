# Changelog — BizClaw

## [2026-02-28] — v0.3.0 Edge-Ready Architecture

### Added
- **Provider Failover** — automatic fallback chain (primary → fallback₁ → fallback₂) with health tracking, cooldown, and atomic counters (~100 bytes/provider)
- **Lane-based Scheduler** — 4 priority lanes (main/cron/subagent/delegate) with per-lane concurrency limits, prevents agent floods on edge devices
- **Agent Discovery** — auto-generates `AGENTS.md` for context injection (full details ≤15 agents, compact table >15 agents, keyword search)
- **SKILL.md Injection** — loads domain expertise files into Hand execution context, supports YAML frontmatter stripping, flat + subdirectory patterns
- **Android/Edge FFI Layer** (`bizclaw-ffi` crate) — 7-function UniFFI surface (`start_daemon`, `stop_daemon`, `get_status`, `send_message`, `get_version`, `register_device_tools`, `execute_device_action`) with `catch_unwind` safety, 2-thread Tokio runtime for edge devices
- **Android Agent Platform** — 16 Kotlin files, 2750 LOC:
  - `BizClawDaemonService` — Foreground service 24/7, WakeLock, START_STICKY, auto-restart on task removal
  - `BizClawAccessibilityService` — Control ANY app: read screen, click, type, swipe, tap coordinates, navigation
  - `AppController` — High-level workflows: Facebook post/comment, Messenger reply, Zalo send, generic screen reading
  - `DeviceCapabilities` — Battery, storage, network, GPS, CPU/RAM, OEM battery killer detection (Xiaomi/Samsung/OPPO/Vivo/Huawei)
  - `BootReceiver` — Auto-start daemon after phone reboot
  - `DashboardScreen` — Device monitoring, daemon start/stop, device stats grid
  - Material 3 + Jetpack Compose UI, 4 screens (Chat, Agents, Settings, Dashboard)
- New crate: `bizclaw-ffi` (cdylib + rlib for Android .so / Raspberry Pi)
- New modules: `failover.rs`, `lanes.rs`, `discovery.rs`, `skills.rs`
- Total workspace crates: 16 → **18**
- Total tests: 227 → **240** (all passing)
- Zero clippy warnings

### Architecture (Edge-First Design)
- All new modules designed for <30MB RAM targets
- Provider failover uses lock-free atomics (no Mutex overhead)
- Lane scheduler: ~200 bytes per lane, fair priority scheduling
- FFI layer: `catch_unwind` wraps every export to prevent JVM/Dalvik crashes
- Target platforms: Linux x86_64, ARM64 (Raspberry Pi 4/5), Android (arm64-v8a)

## [2026-02-27] — Deploy v2 + 4 New Providers + PageIndex RAG

### Added
- **ByteDance ModelArk** provider — Seed 1.6, Doubao 1.5 Pro 256K/32K (`ARK_API_KEY`)
- **Mistral** provider — mistral-large, mistral-small (`MISTRAL_API_KEY`)
- **MiniMax** provider — MiniMax-Text-01 1M context (`MINIMAX_API_KEY`)
- **xAI (Grok)** provider — grok-3, grok-3-mini (`XAI_API_KEY`)
- **PageIndex MCP** integration — vectorless reasoning-based RAG (98.7% FinanceBench)
- Cross-compilation support via `cargo-zigbuild` (Mac → Linux x86_64)
- `deploy.sh` — automated VPS deployment script (SCP + SSH)
- OpenSSL vendored feature for Linux cross-compilation
- Provider aliases: `grok`→xai, `bytedance`/`doubao`/`ark`/`volcengine`→modelark

### Changed
- Total AI providers: 11 → **15** built-in
- Knowledge RAG description: FTS5/BM25 + PageIndex MCP (dual-mode)
- README: PageIndex as first MCP example

### Infrastructure
- VPS: `116.118.2.98` running binary v2 (commit `ee70345`)
- Processes: bizclaw-platform (port 3001) + 3 tenant gateways (10001, 10002, 10004)
- SSL: Active for apps.bizclaw.vn, bizclaw.vn, apps.viagent.vn, viagent.vn

---

## [2026-02-27] — Dashboard Complete (18/18 Pages)

### Added
- ChatPage with WebSocket streaming
- All dashboard pages completed (18/18)
- Orchestration UI with delegate form

### Fixed
- Clippy cleanup: 122→8 warnings
- No-cache headers for dashboard JS
- Auth fix for orchestration
