---
description: 💾 Lưu kiến thức dự án - The Infinite Memory Keeper
---

# WORKFLOW: /save-brain - Complete Documentation System

> **Role**: Antigravity Librarian  
> **Mission**: Chống lại "Context Drift" - đảm bảo AI không bao giờ quên
> **Principle**: "Code thay đổi → Docs thay đổi NGAY LẬP TỨC"

// turbo-all

---

## 🎯 Mục đích

Workflow này đảm bảo kiến thức dự án được lưu trữ **vĩnh viễn** và **có cấu trúc**, cho phép:
- Khôi phục context sau khi session kết thúc
- Onboard thành viên mới nhanh chóng
- Tránh lặp lại lỗi đã gặp
- Duy trì consistency trong thiết kế

---

## Phase 1: Change Analysis 📊

### 1.1 Interactive Discovery
```
"Hôm nay chúng ta đã thay đổi những gì quan trọng?"
- Hoặc: "Để em tự quét các file vừa sửa?"
```

### 1.2 Auto-Scan Changes
```bash
# Xem các file thay đổi gần đây
git diff --name-only HEAD~5
git log --oneline -10
```

### 1.3 Classify Changes
| Type | Description | Action |
|------|-------------|--------|
| **MAJOR** | Thêm module, thay đổi DB, API mới | Update Architecture + Schema |
| **MINOR** | Sửa bug, refactor | Note log + Changelog |
| **PATCH** | Typo, formatting | Chỉ commit message |

---

## Phase 2: Documentation Update 📝

### 2.1 System Architecture
**File**: `docs/architecture/system_overview.md`

Update khi có:
- ✅ Module mới
- ✅ Third-party API integration
- ✅ Database changes
- ✅ Service dependencies

### 2.2 Database Schema
**File**: `docs/database/schema.md`

```markdown
## Tables

### users
| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| email | VARCHAR(255) | Unique, indexed |
| created_at | TIMESTAMP | Auto-generated |

### Relationships
- users 1:N orders
- orders N:1 products
```

### 2.3 API Documentation ⚠️ CRITICAL
**File**: `docs/api/endpoints.md`

```markdown
# API Documentation

## Authentication
### POST /api/auth/login
- **Description:** User login
- **Body:** `{ email: string, password: string }`
- **Response:** `{ token: string, user: User }`
- **Errors:** 
  - 401: Invalid credentials
  - 429: Rate limited

## Resources
### GET /api/users
- **Description:** List users with pagination
- **Auth:** Required (Admin only)
- **Query:** `?page=1&limit=10&sort=created_at`
- **Response:** `{ data: User[], meta: Pagination }`
```

### 2.4 Business Logic
**File**: `docs/business/rules.md`

```markdown
# Business Rules

## Pricing
- Đơn hàng > 500k VND: Free shipping
- Discount tối đa: 50% giá gốc

## Points System  
- 1 điểm = 1,000 VND chi tiêu
- Điểm hết hạn sau 365 ngày
- Admin có thể override điểm

## Access Control
- Admin: Full access
- Manager: CRUD products, view orders
- Staff: View only
```

### 2.5 Spec Status
- Move specs `Draft` → `Implemented`
- Update nếu có deviation từ plan

---

## Phase 3: Codebase Documentation 📚

### 3.1 README Update
```markdown
## Quick Start

### Prerequisites
- Node.js >= 18.0.0
- PostgreSQL >= 14
- Redis >= 6

### Environment Variables
| Variable | Description | Required |
|----------|-------------|----------|
| DATABASE_URL | PostgreSQL connection | ✅ |
| REDIS_URL | Redis connection | ✅ |
| JWT_SECRET | Auth secret | ✅ |

### Installation
\`\`\`bash
npm install
npm run db:migrate
npm run dev
\`\`\`
```

### 3.2 CHANGELOG.md
```markdown
# Changelog

## [2026-01-16]

### Added
- Tích hợp thanh toán VNPay
- API endpoint `/api/payments/vnpay`
- Webhook handler cho payment callbacks

### Changed
- Cập nhật Order schema thêm `payment_status`
- Refactor PaymentService với Strategy Pattern

### Fixed
- Lỗi race condition khi xử lý concurrent payments
- Memory leak trong WebSocket connection

### Security
- Thêm rate limiting cho payment endpoints
- Validate HMAC signature từ VNPay
```

### 3.3 Inline Documentation
Kiểm tra và thêm JSDoc cho functions phức tạp:

```typescript
/**
 * Process payment with retry logic and idempotency
 * @param orderId - Unique order identifier
 * @param amount - Payment amount in VND
 * @param method - Payment method (vnpay|momo|bank)
 * @returns PaymentResult with transaction details
 * @throws PaymentError if all retries fail
 * @example
 * const result = await processPayment('order-123', 500000, 'vnpay');
 */
async function processPayment(
  orderId: string, 
  amount: number, 
  method: PaymentMethod
): Promise<PaymentResult> {
  // Implementation
}
```

---

## Phase 4: Knowledge Items Sync 🧠

### 4.1 Update KIs
Lưu vào `.gemini/antigravity/knowledge/` hoặc tương đương:

- **Patterns mới**: Design patterns được áp dụng
- **Gotchas**: Bugs đã gặp và cách fix
- **Integrations**: Third-party service configs
- **Decisions**: Architecture decisions và rationale

### 4.2 KI Template
```markdown
# [Topic] Knowledge Item

## Summary
[1-2 sentences mô tả]

## Key Points
- Point 1
- Point 2

## Implementation Details
[Code snippets, configs]

## Gotchas
- ⚠️ Known issue 1
- ⚠️ Known issue 2

## References
- [Link to docs]
- [Related PRD]
```

---

## Phase 5: Deployment Config 🚀

### 5.1 Environment Variables
Update `.env.example`:

```bash
# App
NODE_ENV=development
PORT=3000

# Database  
DATABASE_URL=postgresql://user:pass@localhost:5432/db

# Auth
JWT_SECRET=your-secret-here
JWT_EXPIRES_IN=7d

# External Services
VNPAY_TMN_CODE=
VNPAY_HASH_SECRET=
VNPAY_URL=https://sandbox.vnpayment.vn

# NEW VARIABLES (Added 2026-01-16)
REDIS_URL=redis://localhost:6379
SENTRY_DSN=
```

### 5.2 Infrastructure Notes
```markdown
## Server Configuration
- **Provider:** AWS / GCP / DigitalOcean
- **Instance:** t3.medium (2 vCPU, 4GB RAM)
- **Region:** ap-southeast-1 (Singapore)

## Scheduled Tasks
| Task | Schedule | Command |
|------|----------|---------|
| DB Backup | 0 3 * * * | `pg_dump ...` |
| Cache Clear | 0 */6 * * * | `redis-cli FLUSHDB` |
| Report Gen | 0 8 * * 1 | `npm run reports` |
```

---

## Phase 6: Confirmation ✅

### 6.1 Summary Report
```
📝 Em đã cập nhật bộ nhớ. Các file đã update:

✅ docs/architecture/system_overview.md
✅ docs/api/endpoints.md  
✅ docs/database/schema.md
✅ CHANGELOG.md
✅ .env.example

📊 Statistics:
- Lines added: 245
- Files modified: 5
- New documentation: 3 sections
```

### 6.2 Confirmation Message
```
"Giờ đây em đã ghi nhớ kiến thức này vĩnh viễn."
"Anh có thể tắt máy yên tâm. Mai dùng /recap là em nhớ lại hết."
```

---

## ⚠️ NEXT STEPS

```
1️⃣ Xong buổi làm việc? → Nghỉ ngơi thôi!
2️⃣ Mai quay lại? → /recap để nhớ lại context
3️⃣ Cần làm tiếp? → /plan hoặc /code
4️⃣ Deploy lên production? → /deploy
```

---

## 💡 BEST PRACTICES

| Thời điểm | Action |
|-----------|--------|
| Sau mỗi tính năng lớn | `/save-brain` |
| Cuối mỗi ngày làm việc | `/save-brain` |
| Trước khi nghỉ phép | `/save-brain` |
| Sau khi fix critical bug | `/save-brain` |
| Trước khi handover | `/save-brain` |

---

*"Knowledge is power, but only if it's documented."*
