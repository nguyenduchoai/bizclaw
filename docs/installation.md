# BizClaw - Hướng Dẫn Cài Đặt Chi Tiết

## Mục lục

1. [Yêu cầu hệ thống](#1-yêu-cầu-hệ-thống)
2. [macOS](#2-macos)
3. [Windows](#3-windows)
4. [Linux](#4-linux)
5. [Docker](#5-docker)
6. [Build từ Source](#6-build-từ-source)
7. [Cài đặt nâng cao](#7-cài-đặt-nâng-cao)

---

## 1. Yêu cầu hệ thống

| Hạng mục | Tối thiểu | Khuyến nghị |
|----------|-----------|-------------|
| **RAM** | 512MB | 4GB+ |
| **Ổ cứng** | 500MB | 2GB+ |
| **OS** | macOS 10.14+, Windows 10+, Ubuntu 18.04+ | macOS 12+, Windows 11, Ubuntu 22.04+ |
| **Network** | Internet để sử dụng AI cloud | Internet ổn định |

---

## 2. macOS

### Cách 1: Cài đặt từ DMG (Khuyến nghị)

1. Tải `BizClaw.dmg` từ [bizclaw.vn/download](https://bizclaw.vn/download)
2. Mở file `.dmg`
3. Kéo icon **BizClaw** vào thư mục **Applications**
4. Mở BizClaw từ Launchpad

### Cách 2: Cài đặt từ Terminal

```bash
# Tải phiên bản mới nhất
curl -sSL https://bizclaw.vn/install.sh | bash

# Hoặc cài qua Homebrew (sắp ra mắt)
brew install bizclaw
```

### Cho phép ứng dụng chạy (nếu bị chặn)

1. Vào **System Settings** → **Privacy & Security**
2. Cuộn xuống tìm **BizClaw**
3. Nhấn **"Open Anyway"**

### Gỡ cài đặt

```bash
# Xóa ứng dụng
rm -rf /Applications/BizClaw.app

# Xóa dữ liệu (tùy chọn)
rm -rf ~/.bizclaw
```

---

## 3. Windows

### Cách 1: Cài đặt từ EXE (Khuyến nghị)

1. Tải `BizClaw-Setup.exe` từ [bizclaw.vn/download](https://bizclaw.vn/download)
2. Double-click vào file đã tải
3. Nhấn **"Next"** để tiếp tục
4. Chọn thư mục cài đặt (mặc định: `C:\Program Files\BizClaw`)
5. Nhấn **"Install"**
6. Sau khi hoàn tất, nhấn **"Finish"**

### Cách 2: Cài đặt từ ZIP

1. Tải `BizClaw-win.zip`
2. Giải nén vào thư mục mong muốn
3. Double-click `BizClaw.exe` để chạy

### Tạo shortcut Desktop

1. Sau khi cài đặt, right-click **BizClaw.exe**
2. Chọn **"Create shortcut"**
3. Di chuyển shortcut ra Desktop

### Gỡ cài đặt

1. Vào **Settings** → **Apps** → **Installed apps**
2. Tìm **BizClaw**
3. Nhấn **"Uninstall"**

---

## 4. Linux

### Cách 1: AppImage (Khuyến nghị)

```bash
# Tải AppImage
wget https://bizclaw.vn/downloads/bizclaw.AppImage

# Cho phép execute
chmod +x bizclaw.AppImage

# Chạy
./bizclaw.AppImage
```

### Cách 2: DEB Package (Ubuntu/Debian)

```bash
# Tải package
wget https://bizclaw.vn/downloads/bizclaw.deb

# Cài đặt
sudo dpkg -i bizclaw.deb

# Nếu có lỗi dependency, chạy:
sudo apt-get install -f
```

### Cách 3: RPM Package (Fedora/RHEL)

```bash
sudo rpm -i https://bizclaw.vn/downloads/bizclaw.rpm
```

### Cách 4: Cài đặt qua Script

```bash
curl -sSL https://bizclaw.vn/install.sh | sudo bash
```

### Desktop Entry (Tùy chọn)

Để BizClaw xuất hiện trong Application Menu:

```bash
# Tạo desktop entry
sudo bash -c 'cat > /usr/share/applications/bizclaw.desktop << EOF
[Desktop Entry]
Name=BizClaw
Comment=AI Agent Platform
Exec=/path/to/bizclaw.AppImage
Icon=/path/to/bizclaw.png
Terminal=false
Type=Application
Categories=Network;Utility;
EOF'
```

### Gỡ cài đặt

```bash
# DEB
sudo apt-get remove bizclaw

# RPM
sudo rpm -e bizclaw

# Xóa dữ liệu
rm -rf ~/.bizclaw
```

---

## 5. Docker

### Yêu cầu

- Docker Engine 20.10+
- Docker Compose 2.0+

### Cài đặt nhanh

```bash
# Clone repository
git clone https://github.com/nguyenduchoai/bizclaw-cloud.git
cd bizclaw

# Chạy với Docker Compose
docker-compose up -d

# Truy cập dashboard
open http://localhost:3000
```

### Docker Compose Files

**Standalone (1 tenant):**

```yaml
# docker-compose.standalone.yml
version: '3.8'
services:
  bizclaw:
    image: bizclaw/bizclaw:latest
    ports:
      - "3000:3000"
    volumes:
      - ./data:/root/.bizclaw
    environment:
      - BIZCLAW_MODE=standalone
```

**Production (Multi-tenant):**

```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  bizclaw:
    image: bizclaw/bizclaw:latest
    ports:
      - "3000:3000"
    volumes:
      - ./data:/root/.bizclaw
    environment:
      - BIZCLAW_MODE=production
      - DATABASE_URL=postgresql://user:pass@db:5432/bizclaw
    depends_on:
      - db

  db:
    image: postgres:15
    environment:
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
      - POSTGRES_DB=bizclaw
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

### Remote Access với Cloudflare Tunnel

```bash
# Chạy với tunnel
docker-compose -f docker-compose.standalone.yml up -d bizclaw cloudflared

# Lấy URL truy cập từ xa
docker logs cloudflared
```

### Commands hữu ích

```bash
# Xem logs
docker logs -f bizclaw

# Restart
docker-compose restart bizclaw

# Stop
docker-compose down

# Update image
docker pull bizclaw/bizclaw:latest
docker-compose up -d
```

---

## 6. Build từ Source

### Yêu cầu

- Rust 1.70+ ([rustup.rs](https://rustup.rs))
- Git
- CMake (cho một số dependencies)

### Các bước

```bash
# Clone repository
git clone https://github.com/nguyenduchoai/bizclaw-cloud.git
cd bizclaw

# Build release
cargo build --release

# Chạy desktop app
./target/release/bizclaw-desktop

# Hoặc chạy server
./target/release/bizclaw init
./target/release/bizclaw serve
```

### Cross-compile cho Raspberry Pi

```bash
# Cài đặt target ARM
rustup target add armv7-unknown-linux-gnueabihf

# Build
cargo build --release --target armv7-unknown-linux-gnueabihf
```

---

## 7. Cài đặt nâng cao

### One-Click Install VPS

```bash
curl -sSL https://bizclaw.vn/install.sh | sudo bash -s -- \
  --domain bot.yourdomain.com \
  --admin-email you@email.com
```

### Cấu hình Config.toml

```toml
# ~/.bizclaw/config.toml

[server]
host = "0.0.0.0"
port = 3000

[security]
encryption = true
vault_enabled = true

[providers]
default = "openai"

[channels]
telegram_enabled = true
zalo_enabled = true
```

### Kết nối Cloudflare Tunnel

```bash
# Cài cloudflared
brew install cloudflared  # macOS
# hoặc: curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o /usr/local/bin/cloudflared

# Chạy với tunnel
./target/release/bizclaw serve --tunnel
```

### Environment Variables

| Variable | Mô tả | Default |
|----------|-------|---------|
| `BIZCLAW_PORT` | Port chạy server | `3000` |
| `BIZCLAW_HOST` | Host bind | `0.0.0.0` |
| `BIZCLAW_DATA` | Thư mục data | `~/.bizclaw` |
| `BIZCLAW_MODE` | Mode: standalone/production | `standalone` |
| `BIZCLAW_LOG` | Log level | `info` |

### SSL/TLS (Production)

```bash
# Sử dụng reverse proxy (nginx/caddy)

# Ví dụ Caddyfile
bot.yourdomain.com {
  reverse_proxy localhost:3000
  tls you@email.com
}
```

---

## Troubleshooting

### Lỗi thường gặp

| Lỗi | Nguyên nhân | Cách khắc phục |
|-----|-------------|----------------|
| `Port 3000 already in use` | Port bị chiếm | Đổi port: `BIZCLAW_PORT=3001` |
| `Permission denied` | Không có quyền | Chạy với `sudo` hoặc `chmod +x` |
| `Rust not found` | Chưa cài Rust | Chạy `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` |
| Docker container exit | Lỗi config | Xem logs: `docker logs bizclaw` |

### Lấy logs để hỗ trợ

```bash
# Logs BizClaw
tail -f ~/.bizclaw/logs/bizclaw.log

# Docker logs
docker logs --tail 100 bizclaw

# System info
./bizclaw info
```

---

## Liên hệ hỗ trợ cài đặt

- 📧 Email: support@bizclaw.vn
- 💬 Zalo OA: BizClaw
- 📘 Fanpage: [facebook.com/bizclaw.vn](https://www.facebook.com/bizclaw.vn)
