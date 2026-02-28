# 📱 BizClaw Android — AI Agent Platform

> **Biến điện thoại thành AI Agent chạy 24/7 — không chỉ chat, mà điều khiển cả điện thoại.**

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
| Kotlin files | 16 |
| Lines of Code | 2,750 |
| Screens | 4 (Chat, Agents, Settings, Dashboard) |
| Services | 2 (Daemon, Accessibility) |
| Permissions | 12 |
| FFI functions | 7 |
| Device tools | 10 |
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

**BizClaw Android** v0.3.0 — *Phone = AI Agent Server*
