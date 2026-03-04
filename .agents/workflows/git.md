---
description: Git workflow chuẩn cho BizClaw — tách Public (single-tenant) và Private (multi-tenant cloud)
---

# 🔀 BizClaw Git & Deploy Workflow

> **QUAN TRỌNG:** BizClaw có 2 repo riêng biệt. Mỗi lần push phải push đúng target.

## 📋 Quy tắc TUYỆT ĐỐI

### 2 Repo = 2 Mục đích

| | Public (`origin`) | Private (`cloud`) |
|--|--|--|
| **Repo** | `github.com/nguyenduchoai/bizclaw` | `github.com/nguyenduchoai/bizclaw-cloud` |
| **Remote** | `origin` | `cloud` |
| **Visibility** | 🌐 Public | 🔒 Private |
| **Mode** | Single-tenant ONLY | Multi-tenant + Cloud SaaS |
| **README** | `README.md` (single-tenant) | `README-CLOUD.md` (full cloud) |
| **Target** | Dev, doanh nhân tự cài | VPS deploy, SaaS platform |
| **Có `README-CLOUD.md`?** | ❌ KHÔNG | ✅ CÓ |
| **Có admin platform?** | Code có nhưng README không đề cập | Đầy đủ hướng dẫn |

### Khi Push Code

```bash
# Push lên PRIVATE (cloud) — luôn push trước
git push cloud master

# Push lên PUBLIC (origin) — chỉ khi muốn cập nhật public
# ⚠️ Đảm bảo README.md chỉ có single-tenant info
# ⚠️ Đảm bảo README-CLOUD.md KHÔNG có trong public repo
git push origin master
```

### Khi Sửa README

- **README.md** = Public, single-tenant, self-hosted
  - 3 cách cài đặt: Docker, Build, One-Click
  - Chạy trên Pi/Android/Laptop
  - KHÔNG đề cập: multi-tenant, admin platform, PostgreSQL, VPS deploy
  - Chỉ có bảng so sánh ngắn Self-Hosted vs Cloud → link bizclaw.vn

- **README-CLOUD.md** = Private, multi-tenant, cloud
  - Kiến trúc multi-tenant
  - VPS deploy guide (Docker, Nginx, SSL)
  - Domain management (bizclaw.vn, viagent.vn)
  - Pricing packages
  - Admin Dashboard features
  - Git workflow

## 🚀 Deploy lên VPS

### Quick Deploy

```bash
# 1. Push to private repo
git push cloud master

# 2. SSH pull on VPS
// turbo
sshpass -p 'PASSWORD' ssh root@116.118.2.98 "cd /opt/bizclaw && git pull"

# 3. Rebuild Docker
sshpass -p 'PASSWORD' ssh root@116.118.2.98 "cd /opt/bizclaw && docker build -t bizclaw_bizclaw ."

# 4. Restart both apps
sshpass -p 'PASSWORD' ssh root@116.118.2.98 "docker restart bizclaw-app viagent-app"
```

### VPS Info
- **IP:** 116.118.2.98
- **Credentials:** Stored at `~/.bizclaw/.credentials`
- **Containers:** `bizclaw-app` (port 3001), `viagent-app` (port 3002)

### Domains
| Domain | Type | Target |
|--------|------|--------|
| bizclaw.vn | Landing page | `/var/www/bizclaw-landing` |
| apps.bizclaw.vn | Admin Platform | port 3001 |
| viagent.vn | Landing page | `/var/www/viagent-landing` |
| apps.viagent.vn | Admin Platform | port 3002 |

## 🔄 Landing Page Update

Landing pages nằm trên VPS:
```bash
# Download → Edit → Upload
sshpass -p 'PASSWORD' scp root@VPS:/var/www/bizclaw-landing/index.html /tmp/landing.html
# ... edit ...
sshpass -p 'PASSWORD' scp /tmp/landing.html root@VPS:/var/www/bizclaw-landing/index.html
```

Cũng lưu bản sao trong repo tại `landing/index.html` và `deploy/viagent-landing/index.html`.

## ⚠️ Checklist trước khi Push Public

- [ ] `README.md` chỉ có single-tenant content
- [ ] `README-CLOUD.md` KHÔNG tồn tại
- [ ] Không có credentials/passwords trong code
- [ ] Landing page link đúng (bizclaw.vn)
