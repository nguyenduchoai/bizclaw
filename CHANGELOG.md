# Changelog

## [0.3.0] — 2026-03-05

### Added
- **Workflow Rules Engine**: 6 trigger types → 4 action types, visual builder in dashboard
- **Vector RAG**: Hybrid search (FTS5 keyword + Vector cosine similarity) for knowledge base
- **Scheduler++**: Cron, interval, one-time tasks with Telegram/Email/Webhook notifications
- **Android APK Build Script**: `android/build-apk.sh` (debug/release/clean)
- **InjectionScanner Integration**: Prompt injection detection active in agent pipeline
- **ShellTool Security**: Metacharacter blocking, dangerous pattern detection, env_clear, timeout
- **FileTool Security**: Path validation, traversal detection, write-protected sensitive files
- **ExecuteCodeTool Security**: Dangerous code pattern scanner (16 patterns)
- **AES-256-CBC**: Replaced ECB with CBC encryption for secrets (random IV per encryption)

### Changed
- Version bump: 0.2.0 → 0.3.0
- Test count: 144 → 342 tests passing
- Security headers: Runtime sandbox, HMAC-SHA256 key derivation
- Gateway: all std::sync::Mutex .lock().unwrap() → .unwrap_or_else() for poison recovery
- Agent: SecurityPolicy now checks both shell AND file tools (was shell-only)
- README updated with Workflow Rules, Scheduler, Vector RAG features

### Fixed
- **CRITICAL**: Tenant config loading — pass `--config` CLI flag + `BIZCLAW_CONFIG` env fallback
- **CRITICAL**: Docker networking — tenants bind `0.0.0.0` for port forwarding
- **CRITICAL**: CORS allow-all in production → restricted to 5 whitelisted domains
- **CRITICAL**: JWT secret now persistent via env var (was random per restart)
- SchedulerDb open() error handling

### Security
- AES-256-ECB → AES-256-CBC (random IV, HMAC-SHA256 key derivation)
- ShellTool: defense-in-depth (tool-level + agent-level validation)
- FileTool: forbidden paths, path traversal detection, write protection
- ExecuteCodeTool: dangerous pattern scanner
- InjectionScanner: guardrail injection into LLM context on suspicious prompts
- Mutex poisoning: 27 instances fixed across gateway
- CORS: production-only domain whitelist
- JWT: persistent random secret

## [0.2.0] — 2026-03-01

### Added
- Initial release with 19 crates
- 16 LLM providers, 9 channels, 13 tools
- Brain Engine (GGUF inference + SIMD)
- Knowledge RAG (FTS5)
- Multi-tenant admin platform
- Web Dashboard (20+ pages)
- Android FFI layer
