# 📱 BizClaw Android — AI Agent Platform

> **Biến điện thoại thành AI Agent chạy 24/7 — không chỉ chat, mà điều khiển cả điện thoại.**
> 
> **v0.6.2**: SME Business Dashboard — Auto-reply, Multi-platform posting, Email aggregation, Group monitoring

## 🏗️ Kiến trúc

```
┌─────────────────────────────────────────────────────┐
│  BizClaw Android App (Kotlin/Compose)                │
│                                                      │
│  ┌────────────────────────────────────────────┐      │
│  │  UI Layer (Material 3 + Jetpack Compose)   │      │
│  │  ChatScreen → AgentsScreen → Dashboard     │      │
│  │  SettingsScreen → DashboardScreen          │      │
│  └──────────────────┬─────────────────────────┘      │
│                     │                                │
│  ┌──────────────────┴─────────────────────────┐      │
│  │  Service Layer                              │      │
│  │  ├─ BizClawDaemonService (Foreground 24/7)  │      │
│  │  ├─ BizClawAccessibilityService (App ctrl)  │      │
│  │  ├─ AppController (Facebook/Messenger/Zalo) │      │
│  │  ├─ DeviceCapabilities (Battery/GPS/Net)    │      │
│  │  └─ BootReceiver (Auto-start on reboot)     │      │
│  └──────────────────┬─────────────────────────┘      │
│                     │ JNI/UniFFI                     │
│  ┌──────────────────┴─────────────────────────┐      │
│  │  bizclaw-ffi.so (Rust Native Library)       │      │
│  │  7 FFI functions, 2-thread Tokio runtime    │      │
│  │  <30MB RAM, arm64-v8a                       │      │
│  └────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────┘
```

## ⚡ Tính năng

### 🤖 Embedded Rust Engine
- Rust engine chạy trực tiếp trên phone (không cần server)
- Foreground Service với `START_STICKY` — auto-restart khi bị kill
- WakeLock giữ CPU hoạt động, agent chạy 24/7
- Auto-start sau reboot (BootReceiver)

### 📱 Device Tools
Agent có thể truy cập phần cứng phone:

| Tool | Mô tả |
|------|--------|
| `device.battery` | Level, charging, temperature |
| `device.storage` | Free/used space |
| `device.network` | WiFi/Cellular, SSID, online status |
| `device.location` | GPS coordinates |
| `device.cpu` | Cores, available RAM |
| `device.notification` | Push notification |
| `device.vibrate` | Rung điện thoại |
| `device.clipboard` | Copy text |
| `device.flashlight` | Bật/tắt đèn flash |
| `device.open_url` | Mở URL trên browser |

### ♿ Accessibility Service — Điều khiển App
**Agent có thể điều khiển BẤT KỲ app nào trên điện thoại:**

| Action | API | Mô tả |
|--------|-----|--------|
| `readScreen()` | Đọc | Lấy tất cả text, button, input trên màn hình |
| `clickByText(text)` | Click | Chạm element bất kỳ theo text |
| `typeText(text)` | Gõ | Nhập text vào input field |
| `typeIntoField(hint, text)` | Gõ | Nhập text vào field theo hint/placeholder |
| `tapAt(x, y)` | Chạm | Chạm tọa độ cụ thể |
| `swipe(...)` | Vuốt | Scroll, pull-to-refresh |
| `pressBack()` | Navigation | Phím Back |
| `pressHome()` | Navigation | Phím Home |
| `scrollDown()` | Scroll | Cuộn xuống |
| `pressEnter()` | Submit | Gửi tin nhắn / submit |

### 🛡️ Smart Agent Features (v0.5.0)

**Stuck Detection (5 modes):**

| Stuck Type | Trigger | Recovery |
|------------|---------|----------|
| Screen Frozen | Màn hình không đổi 3 rounds | Thử scroll, đổi approach |
| Action Loop | Lặp cùng action 3+ lần | Dừng lại, tìm element khác |
| Navigation Drift | Spam back/home/scroll | Interact trực tiếp |
| Repeated Failures | 3 failures liên tiếp | Đọc screen trước |
| Repetition Cycle | A-B-A-B pattern | Phá vỡi cycle hoàn toàn |

**Vision Fallback:**
Khi AccessibilityService không đọc được (WebView, Flutter, Game):
```
Accessibility tree empty? → Screenshot → Vision LLM → UI elements + tọa độ
```

**⚡ Flow Runner (No-LLM Macros):**
Chuỗi hành động chạy instant, $0 cost:
- `cross_post` — Đăng bài lên Facebook + Zalo + Instagram cùng lúc
- `broadcast` — Gửi tin nhắn cho nhiều người (Zalo/Messenger/Telegram)
- `sales_post` — Đăng bán hàng + gửi cho danh sách khách
- Custom flows — Tự tạo và lưu trữ

**🔗 Workflow Engine (Multi-App Chain):**
Kết hợp LLM + macro chạy nhiều app tự động:
```
Step 1: [🤖 Agent] Shopee → "Check đơn hàng mới"
Step 2: [⚡ Flow] Zalo → Báo cáo cho Boss
Step 3: [⚡ Flow] Cross-post lên 3 nền tảng
```

**📊 Screen Diff:**
Chỉ gửi *thay đổi* cho LLM, tiết kiệm ~80% token:
```
Round N vs N+1: +2 new, -1 removed, 15 unchanged
→ Gửi 3 elements thay vì 17 → tiết kiệm 120 tokens
```

| Tool | Mô tả |
|------|--------|
| `screen_read_smart()` | Accessibility first, vision fallback |
| `screen_read_diff()` | Chỉ gửi thay đổi, giảm token |
| `screen_capture()` | Screenshot → Vision AI |
| `flow_run()` | Chạy macro instant |
| `flow_list()` | Xem danh sách flows |

### ⚡ Pre-parsed Commands (v0.6.0)
Local commands chạy **không cần LLM**, tức thì, $0 cost:

| Command | Mô tả |
|---------|-------|
| `/status` | Xem session stats, context utilization |
| `/compact` | Trigger context compaction (80% threshold) |
| `/help` | Danh sách commands |
| `/clear` | Xóa conversation history |
| `/model` | Xem model info |
| `/tools` | Danh sách tools |
| `/health` | Check system health |

### 🔄 Session Compaction (v0.6.0)
Auto-compact khi context > 80%:
- Giữ 10 messages gần nhất
- Summarize older messages bằng LLM
- Off-load full transcript ra file
- Free ~60% context window

### 📝 Feedback Collection (v0.6.0)
Self-learning từ user feedback:
- Thumbs up/down sau mỗi response
- Track helpfulness score per agent
- Generate optimization prompts từ failed interactions
- Sync với gateway để train model

### 🎙️ Meeting Assistant (v0.6.1)
AI-powered meeting recording và transcription:

| Feature | Mô tả |
|---------|--------|
| 🎤 Audio Recording | One-tap record với timer + waveform animation |
| 📝 Transcription | Whisper API, Google STT, hoặc Ollama local |
| 🤖 AI Recap | Tạo recap từ transcript thực tế |
| ✅ Action Items | Trích xuất task, assignee, deadline, priority |
| 📅 Calendar | Tích hợp calendar, tạo reminder từ action items |
| 📱 Auto-send | Gửi recap qua Zalo/Email tự động |

**Supported STT Providers:**
- OpenAI Whisper API (recommended)
- Google Speech-to-Text
- Local Ollama + Whisper

### 💬 Smart Auto-Reply (v0.6.2)
AI-powered auto-reply cho Zalo/Chat:

| Feature | Mô tả |
|---------|--------|
| AI Responses | LLM-generated contextual replies |
| Template Responses | Canned responses cho FAQ |
| Sentiment Detection | Phát hiện khách hàng khó tính |
| Escalation | Chuyển chat phức tạp cho agent |
| Rate Limiting | Tránh spam response |

### 📧 Email Aggregation (v0.6.2)
Multi-account email với digest:

| Feature | Mô tả |
|---------|--------|
| Gmail/Outlook/OAuth | OAuth2 integration |
| Smart Grouping | Thread detection, topic clustering |
| Priority Filtering | Urgent, VIP, Promotions |
| Digest Template | Daily/Weekly summary |

### 📱 Multi-Platform Posting (v0.6.2)
Unified Post Manager cho:

| Platform | Status |
|----------|--------|
| Facebook Page/Group | ✅ |
| Zalo OA | ✅ |
| Instagram | ✅ |
| LinkedIn | ✅ |
| TikTok | 🔜 |
| Custom Webhook | ✅ |

### 📊 Unified Dashboard (v0.6.2)
Tổng hợp metrics từ tất cả services:

- **Zalo Metrics**: Chats, auto-reply, CSAT
- **Email Metrics**: Unread, urgent, pending
- **Social Metrics**: Posts, reach, engagement
- **Action Items**: Tasks từ meetings, escalations
- **Alerts**: Priority notifications
- **Health Score**: 0-100% system health

### 📘 App Workflows

**Facebook:**
```
Agent → openApp("com.facebook.katana")
     → clickByText("Bạn đang nghĩ gì")
     → typeText("Nội dung bài viết")
     → clickByText("Đăng")
```

**Messenger:**
```
Agent → openApp("com.facebook.orca")
     → clickByText("Tên người nhận")
     → typeIntoField("Aa", "Nội dung tin nhắn")
     → clickByText("Gửi")
```

**Zalo:**
```
Agent → openApp("com.zing.zalo")
     → clickByText("Tên liên hệ")
     → typeIntoField("Nhắn tin", "Nội dung")
     → pressEnter()
```

## 🛠️ Setup

### Build
```bash
# Cần Android Studio + SDK 35
cd android
./gradlew assembleDebug
```

### Bật Accessibility Service
```
Settings → Accessibility → BizClaw Agent → Enable ✅
```

### Bật Daemon Service
```
Mở app → Dashboard → Nhấn "Khởi động Agent"
```

## 📦 Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Kotlin 2.1, Rust |
| UI | Jetpack Compose, Material 3, Material You |
| Architecture | MVVM, Foreground Service, Accessibility Service |
| Native | bizclaw-ffi (cdylib via JNI/UniFFI) |
| Network | OkHttp 4.12, SSE streaming |
| Security | EncryptedSharedPreferences |
| Serialization | kotlinx.serialization |
| Min SDK | 26 (Android 8.0) |
| Target SDK | 35 (Android 15) |

## 📊 Android Stats

| Metric | Value |
|--------|-------|
| Kotlin files | 22 |
| Lines of Code | 5,400+ |
| Screens | 4 (Chat, Agents, Settings, Dashboard) |
| Services | 2 (Daemon, Accessibility) |
| Permissions | 12 |
| FFI functions | 7 |
| Device tools | 10 |
| Agent tools | 24 |
| Flow actions | 24 |
| Stuck detection modes | 5 |
| Vision providers | 3 (Gemini, OpenAI, Ollama) |
| Supported apps | Facebook, Messenger, Zalo, any app |

## ⚠️ OEM Battery Killer

App tự phát hiện hãng phone và cảnh báo:

| Hãng | Hướng dẫn |
|------|-----------|
| Xiaomi/Redmi | Bật AutoStart + tắt Battery Optimization |
| Samsung | Thêm vào 'Unmonitored apps' |
| OPPO/Realme | Bật AutoStart + Allow background activity |
| Vivo | Allow AutoStart + High background power |
| Huawei/Honor | Tắt 'Manage automatically' |
| OnePlus | Tắt Battery Optimization |

---

**BizClaw Android** v0.5.0 — *Phone = AI Agent Server — Smart Automation Platform*
