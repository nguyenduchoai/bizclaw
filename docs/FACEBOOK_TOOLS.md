# Facebook Automation Tools

Hai công cụ tự động hóa Facebook cho BizClaw Agent.

## 1. Facebook Poster Tool

### Giới thiệu
Tự động đăng bài lên Facebook Page với:
- Lập lịch bài viết
- Retry mechanism
- Multi-account support
- Rate limit handling

### Cách sử dụng

```rust
use bizclaw_social::{FacebookPoster, FacebookPosterConfig};

let poster = FacebookPoster::new();

// Đăng ký tài khoản
let config = FacebookPosterConfig {
    page_id: "your_page_id".to_string(),
    access_token: "your_access_token".to_string(),
    agent_name: "marketing_bot".to_string(),
    auto_retry: true,
    max_retries: 3,
    retry_delay_secs: 60,
};

poster.register_account(config).await?;

// Đăng bài ngay
poster.post_now("marketing_bot", "Nội dung bài viết!", Some("https://image-url.com/pic.jpg")).await?;

// Lập lịch đăng bài
use chrono::{Duration, Utc};
let scheduled_time = Utc::now() + Duration::hours(2);
poster.schedule_post(
    "marketing_bot",
    "Bài viết được lên lịch".to_string(),
    None,
    scheduled_time,
).await?;
```

### API Reference

#### `register_account(config)`
Đăng ký tài khoản Facebook Page.

```rust
poster.register_account(config).await?;
```

#### `post_now(agent_name, content, image_url)`
Đăng bài ngay lập tức.

```rust
poster.post_now("marketing_bot", "Content", Some("image.jpg")).await?;
```

#### `schedule_post(agent_name, content, image_url, scheduled_time)`
Lập lịch đăng bài.

```rust
poster.schedule_post("bot", "Content", None, future_time).await?;
```

#### `cancel_post(post_id)`
Hủy bài đã lên lịch.

#### `get_post_status(post_id)`
Kiểm tra trạng thái bài viết.

#### `get_scheduled_posts(agent_name)`
Lấy danh sách bài đã lên lịch.

#### `get_metrics(post_id)`
Lấy metrics bài viết (impressions, reach, engagements).

### Retry Mechanism
- Mặc định: 3 lần retry với delay 60s
- Exponential backoff: 60s, 120s, 240s
- Cấu hình qua `auto_retry`, `max_retries`, `retry_delay_secs`

---

## 2. Facebook Inbox Collector

### Giới thiệu
Thu thập và tổng hợp tin nhắn từ Facebook Page inbox:
- Webhook integration
- Message filtering
- Auto-classification
- Agent routing

### Cách sử dụng

```rust
use bizclaw_social::{FacebookInboxCollector, InboxConfig};

// Khởi tạo
let collector = FacebookInboxCollector::new();

// Cấu hình
let config = InboxConfig {
    page_id: "your_page_id".to_string(),
    access_token: "your_access_token".to_string(),
    verify_token: "your_verify_token".to_string(),
    webhook_secret: "webhook_secret".to_string(),
    auto_reply: false,
    routing_enabled: true,
};
collector.configure(config).await?;

// Thêm routing rule
let rule = RoutingRule {
    id: uuid::Uuid::new_v4().to_string(),
    name: "Order Routing".to_string(),
    conditions: vec![RoutingCondition {
        field: "category".to_string(),
        operator: "equals".to_string(),
        value: "order".to_string(),
    }],
    action: RoutingAction {
        route_to: "sales_bot".to_string(),
        add_label: Some("order".to_string()),
        auto_reply: None,
    },
    priority: 1,
    enabled: true,
};
collector.add_routing_rule(rule).await?;

// Lấy tin nhắn với filter
let filters = MessageFilters {
    sender_id: None,
    keyword: Some("đơn hàng".to_string()),
    from_time: None,
    to_time: None,
    classification: None,
};
let messages = collector.get_messages(Some(filters)).await;
```

### Webhook Integration

```rust
// Verify webhook (for Facebook webhook setup)
let challenge = collector.verify_webhook(mode, token, challenge_str).await?;

// Handle webhook events
let events: WebhookEvent = serde_json::from_str(&payload)?;
let new_messages = collector.handle_webhook(events).await?;
```

### Message Classification
Tự động phân loại tin nhắn theo keywords:

| Category | Keywords (Tiếng Việt) |
|----------|---------------------|
| order | đơn hàng, mua, giá, ship, đặt |
| support | help, hỗ trợ, lỗi, problem, giúp |
| complaint | khiếu nại, không hài lòng, tệ, dở |
| inquiry | hỏi, thắc mắc, tư vấn, cho hỏi |
| feedback | góp ý, đề xuất, cải thiện, tốt hơn |
| general | (default) |

### Routing Rules
Tự động chuyển tin nhắn đến agent phù hợp:

```rust
let rule = RoutingRule {
    conditions: vec![
        RoutingCondition {
            field: "keyword".to_string(),
            operator: "contains".to_string(),
            value: "đặt hàng".to_string(),
        }
    ],
    action: RoutingAction {
        route_to: "sales_bot".to_string(),
        add_label: Some("order".to_string()),
        auto_reply: Some("Cảm ơn bạn đã đặt hàng...".to_string()),
    },
    // ...
};
```

### API Reference

#### `configure(config)`
Cấu hình collector.

#### `verify_webhook(mode, token, challenge)`
Verify webhook với Facebook.

#### `handle_webhook(event)`
Xử lý webhook event từ Facebook.

#### `add_routing_rule(rule)`
Thêm routing rule.

#### `get_messages(filters)`
Lấy tin nhắn với bộ lọc.

#### `get_conversations()`
Lấy danh sách cuộc trò chuyện.

#### `mark_as_read(message_id)`
Đánh dấu đã đọc.

#### `add_label(message_id, label)`
Thêm nhãn vào tin nhắn.

#### `get_statistics()`
Lấy thống kê inbox.

#### `fetch_messages_from_api(limit)`
Fetch tin nhắn trực tiếp từ Facebook API.

---

## Cài đặt Webhook

### 1. Tạo Facebook App
1. Vào [Meta Developer Console](https://developers.facebook.com)
2. Tạo App với type: "Business"
3. Thêm product "Webhooks"

### 2. Setup Webhook
```
URL: https://your-domain.com/webhook/facebook
Verify token: (your verify token)
```

### 3. Subscribe to:
- `conversations`
- `messages`

## Environment Variables

```bash
FACEBOOK_PAGE_ID=your_page_id
FACEBOOK_ACCESS_TOKEN=your_access_token
FACEBOOK_WEBHOOK_VERIFY_TOKEN=your_verify_token
FACEBOOK_APP_SECRET=your_app_secret
```

## Unit Tests

```bash
cargo test -p bizclaw-social
```
