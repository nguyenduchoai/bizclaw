# 🦞 BizClaw - AI Agent Platform cho Doanh Nghiệp Việt Nam

**Nền tảng AI Agent tự-host, kết nối Zalo, Telegram, Meta Ads, SAPO, Haravan... giúp tự động hóa bán hàng, chăm sóc khách hàng 24/7.**

---

## 🎯 BizClaw Là Gì?

BizClaw là **AI Agent platform** giúp doanh nghiệp Việt Nam:
- Tự động trả lời khách hàng trên Zalo, Telegram, Facebook
- Tạo báo cáo tự động
- Hoạt động 24/7, không cần nhân viên trực chat

### Điểm khác biệt

| Platform khác | BizClaw |
|--------------|---------|
| ChatGPT/Gemini | Agent có trí nhớ, kết nối business data |
| SaaS đắt tiền | Self-hosted, unlimited usage |
| Cần nhiều nhân viên | Tự động hóa 80% công việc |
| Dữ liệu ra cloud | 100% private, tự host |

---

## 🚀 Cài Đặt Nhanh

### Cách 1: Docker (Recommended)

```bash
# Tạo docker-compose.yml
cat > docker-compose.yml << 'EOF'
services:
  bizclaw:
    image: ghcr.io/nguyenduchoai/bizclaw:latest
    ports:
      - "3000:3000"  # Web Dashboard
      - "8080:8080"  # API Gateway
    volumes:
      - ./data:/data
    environment:
      - AI_PROVIDER=openai
      - OPENAI_API_KEY=sk-xxx
      - ADMIN_PASSWORD=your-secure-password
EOF

# Chạy
docker-compose up -d
```

Truy cập `http://localhost:3000`

### Cách 2: Build từ Source

```bash
# Yêu cầu: Rust 1.85+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone & build
git clone https://github.com/nguyenduchoai/bizclaw.git
cd bizclaw
cargo build --release

# Chạy
./target/release/bizclaw-gateway
```

---

## 💬 Kết Nối Kênh Chat

### Zalo Official Account (Zalo OA)

```toml
[channels.zalo]
enabled = true
app_id = "123456789012345678"
app_secret = "your-app-secret"
```

### Telegram Bot

```toml
[channels.telegram]
enabled = true
bot_token = "6123456789:AAFxxx"
```

### Facebook/Instagram

```toml
[channels.facebook]
enabled = true
page_access_token = "EAAxxx"
app_secret = "abc123"
```

---


## 📊 AI Providers

```toml
[ai]
default_provider = "openai"

[ai.providers]
[ai.providers.openai]
api_key = "sk-xxx"
model = "gpt-4o"

[ai.providers.anthropic]
api_key = "sk-ant-xxx"
model = "claude-sonnet-4-20250514"

[ai.providers.gemini]
api_key = "AIzaSyxxx"
model = "gemini-2.0-flash"
```

---

## 🔧 Tools Mặc Định

| Tool | Chức năng |
|------|-----------|
| `web_search` | Tìm kiếm Google/DuckDuckGo |
| `browser` | Điều khiển Chrome headless |
| `database` | Query SQL (PostgreSQL, MySQL, SQLite) |
| `file` | Đọc/ghi file |
| `shell` | Chạy commands |
| `http_request` | Gọi API |
| `social_post` | Đăng Facebook, Telegram |
| `zalo_tool` | Gửi Zalo, đọc tin nhắn |

---

## 📱 Use Cases Cụ Thể

### 1. Chatbot Chăm Sóc Khách Hàng

```
Khách hàng hỏi → BizClaw nhận → AI trả lời tự động
                                    ↓
                    Nếu cần tư vấn → Chuyển nhân viên
```


### 4. Báo Cáo Kinh Doanh

```
Agent → Query đơn hàng từ nhiều nền tảng
              ↓
        Tổng hợp doanh thu
              ↓
        Gửi báo cáo Telegram/Zalo cho老板
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    BizClaw Gateway                       │
│                    (HTTP/WebSocket)                    │
├──────────────┬──────────────┬──────────────┬──────────┤
│   Agent     │   Memory    │   Tools    │   Skills │
│   Engine    │   Brain     │   Registry │   Hub    │
├──────────────┴──────────────┴──────────────┴──────────┤
│              Channel Adapters                          │
├────────────┬────────────┬────────────┐
│   Zalo     │  Telegram  │  Facebook  │
│            │            │            │
└────────────┴────────────┴────────────┘
```

---

## 📁 Modules Chính

| Crate | Mô tả |
|-------|--------|
| `bizclaw-agent` | AI Agent engine với tool calling |
| `bizclaw-gateway` | HTTP API server |
| `bizclaw-channels` | Zalo, Telegram, Discord adapters |

| `bizclaw-memory` | SQLite, Vector, Structured storage |
| `bizclaw-mcp` | Model Context Protocol client |
| `bizclaw-resilience` | Rate limiting, Circuit breaker |
| `bizclaw-security` | Encryption, Vault, Sandboxing |

---

## 🔒 Security

```toml
[security]
encryption_key = "auto"  # AES-256-GCM tự động
prompt_injection_detection = true
api_key_vault = "encrypted"

[vault]
backend = "file"  # hoặc "postgres"
```

---

## 📈 Performance

| Metric | Value |
|--------|-------|
| Response time | < 100ms |
| Memory usage | ~50MB idle |
| Concurrent users | Unlimited |
| Token efficiency | Context compression tự động |

---

## 🤝 Contributing

```bash
# Fork & clone
git clone https://github.com/YOUR_USER/bizclaw.git
cd bizclaw

# Tạo feature branch
git checkout -b feature/your-feature

# Develop
cargo build --release

# Test
cargo test

# Commit & push
git add .
git commit -m "feat: add awesome feature"
git push origin feature/your-feature
```

---

## 📚 Tài Liệu

- [Setup Guide](docs/SETUP_GUIDE.md)
- [Architecture Overview](docs/ARCHITECTURE.md)
- [SME Quickstart](docs/sme-quickstart.md)
- [API Endpoints](docs/api/endpoints.md)

---

## 🆘 Support

- 📖 Documentation: [bizclaw.cloud/docs](https://bizclaw.cloud/docs)
- 💬 Issues: GitHub Issues
- 📧 Email: support@bizclaw.cloud

---

## 📄 License

MIT License - Tự do sử dụng cho mục đích thương mại.

---

**Made with ❤️ for Vietnamese businesses**
