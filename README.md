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
**Nền Tảng AI Cho Doanh Nghiệp Một Người (OPC) Việt Nam** — Retail Edition

*Một người, một đội quân AI, vận hành toàn bộ cửa hàng Retail.*

<p align="center">
  <a href="https://github.com/nguyenduchoai/bizclaw/actions"><img src="https://img.shields.io/github/actions/workflow/status/nguyenduchoai/bizclaw/ci?style=flat-square" alt="Build"></a>
  <a href="https://github.com/nguyenduchoai/bizclaw/releases"><img src="https://img.shields.io/github/v/release/nguyenduchoai/bizclaw?style=flat-square" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square" alt="License"></a>
</p>

---

## 🎯 Retail OPC Platform - 100% Ready

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    BIZCLAW RETAIL OPC PLATFORM                             │
├─────────────────┬─────────────────┬─────────────────┬─────────────────────┤
│   POINT OF SALE │    PAYMENTS      │   INVENTORY     │   CUSTOMERS         │
├─────────────────┼─────────────────┼─────────────────┼─────────────────────┤
│ POS Agent       │ VietQR Agent    │ Stock Manager  │ CRM Agent          │
│ Receipt Gen     │ MoMo/ZaloPay    │ Alerts        │ Support Ticket    │
│ Barcode Scan    │ Banking API     │ Warehouse     │ Customer Segment  │
├─────────────────┴─────────────────┴─────────────────┴─────────────────────┤
│                         ACCOUNTING & COMPLIANCE                            │
├─────────────────┬─────────────────┬─────────────────┬─────────────────────┤
│   ACCOUNTING     │    INVOICING    │   BANKING       │   REPORTING       │
├─────────────────┼─────────────────┼─────────────────┼─────────────────────┤
│ Ledger          │ VAT Invoice     │ Bank Transfer  │ Sales Report    │
│ Tax Calculator   │ E-Invoice API  │ Balance Check │ Daily Summary    │
│ Profit/Loss     │ VietQR Payment │ Batch Pay    │ Customer Analytics│
└─────────────────┴─────────────────┴─────────────────┴─────────────────────┘
```

### ✅ 100% OPC Modules Complete

| Module | Agent/Crate | Status |
|--------|-------------|--------|
| **Point of Sale** | bizclaw-pos | ✅ |
| **Payments** | bizclaw-payment | ✅ |
| **Inventory** | bizclaw-inventory | ✅ |
| **Customer Support** | bizclaw-support | ✅ |
| **Accounting** | bizclaw-accounting | ✅ |
| **E-Invoice** | bizclaw-einvoice | ✅ |
| **Banking** | bizclaw-banking | ✅ |
| **Proposals** | bizclaw-proposal | ✅ |
| **Marketing** | bizclaw-outreach | ✅ |
| **Analytics** | bizclaw-analytics | ✅ |

---

## ✨ Tính Năng Hoàn Chỉnh

### 🏪 Retail Operations
- **POS Agent** - Bán hàng tại quầy, quét mã vạch
- **Inventory** - Theo dõi tồn kho, cảnh báo hết hàng
- **Customer** - CRM, phân loại khách hàng VIP/Regular

### 💳 Payments & Banking
- **VietQR** - Thanh toán QR code tức thì
- **MoMo/ZaloPay** - Ví điện tử
- **Banking API** - Chuyển khoản tự động
- **E-Invoice** - Hóa đơn điện tử VNPT/Viettel/MISA

### 📊 Accounting & Compliance
- **Bookkeeping** - Sổ sách kế toán kép
- **VAT Reports** - Báo cáo thuế hàng quý
- **Financial Statements** - Bảng cân đối, P&L tự động

### 📈 Marketing & Sales
- **Outreach** - Zalo, Email marketing tự động
- **Analytics** - Dashboard KPIs, báo cáo doanh thu
- **Proposals** - Báo giá, hợp đồng

---

## 💰 Tiết Kiệm Chi Phí

| Trước BizClaw | Sau BizClaw | Tiết Kiệm |
|---------------|------------|-----------|
| Thu ngân: 8M | POS Agent | -100% |
| Kế toán: 5M | Accounting Agent | -100% |
| Nhân viên kho: 6M | Inventory Agent | -100% |
| CSKH: 5M | Support Agent | -100% |
| Marketing: 7M | Outreach Agent | -80% |
| **Tổng: 31M/tháng** | **~2M/tháng** | **-94%** |
>>>>>>> public/master

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
# Chạy
./target/release/bizclaw-gateway
# Chạy POS
./target/release/bizclaw run --agent pos

# API Gateway
./target/release/bizclaw serve --port 3000
```

### Docker

```bash
docker run -d --name bizclaw-retail \
  -p 3000:3000 -p 8080:8080 \
  -v bizclaw-data:/data \
  nguyenduchoai/bizclaw:latest
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
## 📁 59+ Rust Crates

```
bizclaw/
├── Retail Suite
│   ├── bizclaw-pos/              # Point of Sale
│   ├── bizclaw-payment/          # VietQR, MoMo, ZaloPay
│   ├── bizclaw-inventory/        # Stock management
│   ├── bizclaw-support/         # Customer tickets
│   ├── bizclaw-accounting/      # Bookkeeping
│   ├── bizclaw-proposal/        # Quotes
│   ├── bizclaw-einvoice/        # VN e-invoice
│   └── bizclaw-banking/         # Bank transfers
├── Marketing Suite
│   ├── bizclaw-outreach/        # Zalo, Email
│   ├── bizclaw-analytics/       # Dashboard
│   └── bizclaw-channels/       # Multi-channel
├── AI Suite
│   ├── bizclaw-brain/          # Local AI (llama.cpp)
│   ├── bizclaw-agent/          # Multi-agent
│   └── bizclaw-memory/         # Vector storage
└── desktop/                    # Tauri app
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

## � Configuration

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

```toml
# config.toml
[retail]
store_name = "Cửa Hàng Mẫu"
tax_code = "0123456789"
address = "123 Đường ABC, Quận 1, TP.HCM"

[banking]
vietinbank_account = "1234567890"
vietqr_enabled = true

[invoice]
provider = "vnpt"
username = "your_username"
password = "your_password"

[inventory]
low_stock_alert = 10
auto_restock = true
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

## 📚 Documentation

| Tài Liệu | Mô Tả |
|----------|--------|
| [OPC Platform Plan](docs/BIZCLAW_OPC_PLATFORM_VIETNAM.md) | Business plan |
| [Demo Case Study](docs/DEMO_OPC_CASE_STUDY.md) | MINA's Boutique |
| [Competitive Analysis](docs/COMPETITIVE_ANALYSIS_BIZCLAW_VS_AGENCY_AGENTS.md) | vs competitors |

---

## 🛣️ Roadmap

- [x] **Q1 2025**: Retail OPC Suite ✅
- [ ] **Q2 2025**: Desktop App + Mobile POS
- [ ] **Q3 2025**: E-commerce Integration
- [ ] **Q4 2025**: Enterprise Features

---

## 📄 License

MIT License - Tự do sử dụng cho mục đích thương mại.

---

**Made with ❤️ for Vietnamese businesses**
<p align="center">
  <strong>Made with ❤️ for Vietnamese Retailers</strong>
</p>
