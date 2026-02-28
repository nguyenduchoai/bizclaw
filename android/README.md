# BizClaw Android — Thin Client

Ứng dụng Android kết nối đến BizClaw server (bizclaw-gateway).

## Kiến trúc

```
Phone (Kotlin/Compose) ──HTTP/SSE──▶ BizClaw Server (Rust)
   └── Material You UI                 ├── 15 Providers
   └── Agent selector                  ├── Orchestrator
   └── Chat streaming (SSE)            ├── Knowledge RAG
   └── Encrypted key storage           └── Hands (24/7)
```

## Build

```bash
cd android
./gradlew :app:assembleDebug
```

## Server Connection

App kết nối đến bất kỳ bizclaw-gateway nào:
- **Local**: `http://localhost:3001`
- **VPS**: `https://apps.bizclaw.vn`
- **Raspberry Pi**: `http://raspberrypi.local:3001`
- **Custom**: any OpenAI-compatible endpoint

## API Endpoints Used

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/v1/chat/completions` | POST | Chat (non-streaming + SSE) |
| `/v1/models` | GET | List available models |
| `/api/v1/agents` | GET | List agents |
| `/api/v1/traces` | GET | LLM call history |
| `/api/v1/stats` | GET | Dashboard stats |
| `/health` | GET | Connection test |

## Tech Stack

- **Language**: Kotlin 2.1
- **UI**: Jetpack Compose + Material 3
- **Networking**: OkHttp 4 + SSE
- **Serialization**: kotlinx.serialization
- **Security**: EncryptedSharedPreferences (AES-256-GCM)
- **Min SDK**: 26 (Android 8.0)
- **Target SDK**: 35 (Android 15)
