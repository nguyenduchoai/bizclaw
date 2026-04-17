# BizClaw SME Workflows - Mẫu Workflows Cho Doanh Nghiệp

## 🎯 Workflow Là Gì?

**Workflow** là chuỗi các bước tự động mà AI thực hiện theo thứ tự. Mỗi bước có thể là một agent khác nhau, kết quả của bước trước sẽ chuyển sang bước sau.

---

## 📦 23 Mẫu Workflow Có Sẵn

### 1. Content Pipeline - Viết Content Tự Động

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   DRAFT     │───▶│   REVIEW    │───▶│   POLISH    │
│   Writer    │    │   Editor    │    │   Final     │
└─────────────┘    └─────────────┘    └─────────────┘
       │                                   │
       └──────────────📄────────────────────┘
              Bài viết hoàn chỉnh
```

**Use case:** Viết blog, bài đăng Facebook, nội dung marketing

**Cách dùng:**
1. Vào **🔄 Workflows** → **Content Pipeline**
2. Nhập chủ đề bài viết
3. AI tự động viết → chỉnh sửa → hoàn thiện

---

### 2. Lead Qualification - Sàng lọc Khách Hàng Tiềm Năng

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  NHẬN LIỆT  │───▶│  PHÂN TÍCH  │───▶│  PHÂN LOẠI  │
│   Lead mới  │    │  Nhu cầu    │    │  Hot/Warm/  │
│             │    │  Ngân sách   │    │  Cold lead  │
└─────────────┘    └─────────────┘    └─────────────┘
                                              │
                           ┌──────────────────┼──────────────────┐
                           ▼                  ▼                  ▼
                     ┌──────────┐       ┌──────────┐       ┌──────────┐
                     │  GỬI     │       │  LƯU     │       │  CHĂM    │
                     │  BÁO GIÁ │       │  CRM     │       │  SÓC SAU │
                     └──────────┘       └──────────┘       └──────────┘
```

**Use case:** Xử lý inquiry từ website, phân loại khách hàng

---

### 3. Customer Support Pipeline - Hỗ Trợ Khách Hàng

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   TIẾP     │───▶│   PHÂN     │───▶│   GIẢI     │
│   NHẬN     │    │   TÁCH     │    │   QUYẾT    │
│   Ticket   │    │   Vấn đề   │    │   Trả lời  │
└─────────────┘    └─────────────┘    └─────────────┘
                                             │
                           ┌─────────────────┼─────────────────┐
                           ▼                 ▼                 ▼
                     ┌──────────┐     ┌──────────┐     ┌──────────┐
                     │  ĐƠN     │     │  KỸ      │     │  HOÀN    │
                     │  GIẢN    │     │  THUẬT   │     │  THÀNH   │
                     └──────────┘     └──────────┘     └──────────┘
```

**Use case:** Trả lời ticket hỗ trợ, FAQ tự động

---

### 4. Order Processing - Xử Lý Đơn Hàng

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   NHẬN     │───▶│   XÁC      │───▶│   CẬP      │
│   ĐƠN      │    │   NHẬN     │    │   NHẬT     │
│   MỚI      │    │   TỰ ĐỘNG  │    │   STATUS   │
└─────────────┘    └─────────────┘    └─────────────┘
                                             │
                           ┌─────────────────┼─────────────────┐
                           ▼                 ▼                 ▼
                     ┌──────────┐     ┌──────────┐     ┌──────────┐
                     │  GỬI     │     │  KÍCH    │     │  THÔNG   │
                     │  XÁC     │     │  HOẠT    │     │  BÁO     │
                     │  NHẬN    │     │  SHIPPER │     │  KHÁCH   │
                     └──────────┘     └──────────┘     └──────────┘
```

**Use case:** F&B, bán lẻ, e-commerce

---

### 5. Booking Automation - Đặt Lịch Tự Động

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   NHẬN     │───▶│   KIỂM     │───▶│   XÁC      │
│   YÊU CẦU  │    │   TRA      │    │   NHẬN     │
│   ĐẶT LỊCH │    │   Trùng?   │    │   LỊCH HẸN │
└─────────────┘    └─────────────┘    └─────────────┘
                           │                   │
                           ▼                   ▼
                     ┌──────────┐       ┌──────────┐
                     │  GỬI    │       │  GỬI    │
                     │  LỊCH   │       │  THÔNG  │
                     │  MỚI    │       │  BÁO    │
                     └──────────┘       └──────────┘
```

**Use case:** Spa, clinic, tư vấn, dịch vụ B2B

---

### 6. Social Media Auto-Poster - Đăng Bài Tự Động

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   TẠO     │───▶│   TẠO     │───▶│   ĐĂNG     │
│   Ý TƯỞNG  │    │   NỘI     │    │   LÊN      │
│   Content  │    │   DUNG    │    │   MẠNG XÃ  │
└─────────────┘    └─────────────┘    └─────────────┘
                           │
                           ▼
                     ┌─────────────┐
                     │   SCHEDULE  │
                     │   8h/12h/   │
                     │   20h       │
                     └─────────────┘
```

**Channels:** Facebook, Zalo, Instagram, Telegram

---

### 7. Research Pipeline - Nghiên Cứu Thị Trường

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   GATHER    │───▶│   ANALYZE   │───▶│   REPORT   │
│   Thu thập  │    │   Phân tích │    │   Tổng    │
│   dữ liệu  │    │   Xu hướng  │    │   hợp     │
└─────────────┘    └─────────────┘    └─────────────┘
                                              │
                           ┌──────────────────┴──────────────────┐
                           ▼                                     ▼
                     ┌──────────┐                         ┌──────────┐
                     │  SLIDE   │                         │  EMAIL   │
                     │  ĐẸP    │                         │  GỬI    │
                     └──────────┘                         └──────────┘
```

**Use case:** Báo cáo thị trường, phân tích đối thủ

---

## 🔧 Tạo Workflow Tùy Chỉnh

### 6 Loại Step

| Type | Icon | Mô tả |
|------|------|-------|
| **Sequential** | ➡️ | Steps chạy tuần tự, output step trước → input step sau |
| **FanOut** | 🔀 | Multiple steps chạy song song cùng lúc |
| **Collect** | 📥 | Gom kết quả từ nhiều steps (All/Best/Vote/Merge) |
| **Conditional** | 🔀 | If/else branching - rẽ nhánh theo điều kiện |
| **Loop** | 🔁 | Lặp lại đến khi đạt điều kiện dừng |
| **Transform** | ✨ | Biến đổi template - format output |

### Cách tạo Workflow mới

1. Vào **🔄 Workflows** → **"+ Tạo Workflow mới"**
2. Đặt tên và mô tả
3. Kéo thả các step vào canvas
4. Kết nối các step bằng cách kéo line
5. Cấu hình từng step:
   - Chọn **Agent** thực hiện
   - Thiết lập **input/output**
   - Đặt **điều kiện** (nếu có)
6. Nhấn **"Lưu và Chạy"**

### Ví dụ: Workflow "Trả Lời FAQ"

```
Step 1: RECEIVE (Agent: Support)
├── Input: Tin nhắn khách
└── Output: Câu hỏi được phân loại

Step 2: SEARCH (Agent: Knowledge)
├── Input: Câu hỏi từ Step 1
└── Output: Câu trả lời từ knowledge base

Step 3: CONDITIONAL
├── Nếu có câu trả lời → Step 4a
└── Nếu không có → Step 4b

Step 4a: SEND_ANSWER (Agent: Support)
└── Output: Gửi câu trả lời cho khách

Step 4b: ESCALATE (Agent: Manager)
└── Output: Báo cho quản lý xử lý
```

---

## 📊 Workflow Templates Theo Ngành

### 🛒 E-Commerce / Bán Lẻ

| Workflow | Mô tả |
|----------|-------|
| Lead Qualification | Sàng lọc khách hàng tiềm năng từ Facebook/Shopee |
| Order Processing | Xử lý đơn hàng tự động |
| Customer Support | Trả lời câu hỏi về sản phẩm, đơn hàng |
| Review Response | Phản hồi đánh giá khách hàng |
| Stock Alert | Cảnh báo khi hết hàng |

### 🍜 F&B / Nhà Hàng

| Workflow | Mô tả |
|----------|-------|
| Table Booking | Đặt bàn tự động qua Zalo |
| Order Taking | Nhận đơn từ khách hàng |
| Menu Daily | Gửi menu hàng ngày cho khách quen |
| Review Management | Phản hồi đánh giá Google/TripAdvisor |
| Promotions | Gửi ưu đãi theo dịp lễ |

### ✈️ Du Lịch / Khách Sạn

| Workflow | Mô tả |
|----------|-------|
| Booking Sync | Đồng bộ booking từ Booking.com, Agoda |
| Guest Welcome | Tin nhắn chào mừng tự động |
| Checkout Reminder | Nhắc nhở checkout |
| Review Request | Yêu cầu đánh giá sau khi ở |
| Upsell | Đề xuất dịch vụ thêm (spa, airport transfer) |

### 🏢 Dịch Vụ B2B

| Workflow | Mô tả |
|----------|-------|
| Lead Nurturing | Nuôi dưỡng lead qua nhiều giai đoạn |
| Quote Generation | Tạo báo giá tự động |
| Meeting Scheduler | Đặt lịch hẹn tư vấn |
| Contract Follow-up | Theo dõi hợp đồng |
| Renewal Reminder | Nhắc nhở gia hạn dịch vụ |

---

## ⏰ Scheduler - Lên Lịch Workflow

### Các loại lịch

| Loại | Mô tả | Ví dụ |
|------|-------|-------|
| **Cron** | Lặp theo cron expression | `0 8 * * *` = 8h sáng hàng ngày |
| **Interval** | Lặp theo khoảng thời gian | Mỗi 10 phút |
| **One-time** | Chạy 1 lần vào thời điểm nhất định | 2026-05-01 10:00 |

### Ví dụ Cron Expression

| Expression | Ý nghĩa |
|------------|---------|
| `0 8 * * *` | 8:00 AM hàng ngày |
| `0 */6 * * *` | Mỗi 6 giờ |
| `0 9-17 * * 1-5` | 9AM-5PM, Thứ 2-6 |
| `0 10 * * 1` | 10AM Thứ 2 hàng tuần |

---

## 📱 Workflow Channels

BizClaw hỗ trợ nhiều kênh để nhận input và gửi output:

| Kênh | Nhận input | Gửi output |
|------|-------------|------------|
| 💬 Zalo OA | ✅ | ✅ |
| ✈️ Telegram | ✅ | ✅ |
| 🎮 Discord | ✅ | ✅ |
| 📧 Email | ✅ | ✅ |
| 🔗 Webhook | ✅ | ✅ |
| 💻 WebChat | ✅ | ✅ |

---

## 🔔 Workflow Notifications

Cấu hình thông báo kết quả workflow:

```toml
[[workflow.notifications]]
channel = "zalo"
recipients = ["0909123456"]
on_success = true
on_failure = true

[[workflow.notifications]]
channel = "email"
recipients = ["admin@company.com"]
on_failure = true
```

---

## 💡 Best Practices

### Nên làm

- ✅ Bắt đầu với workflow đơn giản, sau đó mở rộng
- ✅ Đặt tên step rõ ràng, dễ hiểu
- ✅ Test workflow với input nhỏ trước
- ✅ Monitor logs sau khi deploy

### Không nên

- ❌ Tạo workflow quá phức tạp (max 10-15 steps)
- ❌ Để workflow chạy quá thường xuyên (tốn API)
- ❌ Bỏ qua error handling
- ❌ Hard-code credentials trong workflow

---

## 📞 Hỗ Trợ

- 📧 Email: support@bizclaw.vn
- 💬 Zalo OA: BizClaw
- 📘 Fanpage: [facebook.com/bizclaw.vn](https://www.facebook.com/bizclaw.vn)

---

*BizClaw v1.1.7 - Tự động hóa doanh nghiệp với AI* 🚀
