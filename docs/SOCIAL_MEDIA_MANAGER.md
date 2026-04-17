# Social Media Manager - Multi-Platform Integration

## Tổng quan

Unified API để quản lý tất cả các nền tảng mạng xã hội, tích hợp đầy đủ từ BrightBean Studio.

## Platform Support

| Platform | Publish | Comments | DMs | Insights |
|----------|---------|----------|-----|----------|
| **Facebook** | ✅ | ✅ | ✅ | ✅ |
| **Instagram** | ✅ | ✅ | ✅ | ✅ |
| **Instagram Personal** | ✅ | ✅ | ✅ | ✅ |
| **LinkedIn Personal** | ✅ | ✅ | - | ✅ |
| **LinkedIn Company** | ✅ | ✅ | - | ✅ |
| **TikTok** | ✅ | ✅ | - | ✅ |
| **YouTube** | ✅ | ✅ | - | ✅ |
| **Pinterest** | ✅ | - | - | ✅ |
| **Threads** | ✅ | ✅ | - | ✅ |
| **Bluesky** | ✅ | ✅ | - | ✅ |
| **Google Business** | ✅ | - | - | ✅ |

## Usage

```rust
use bizclaw_social::{SocialMediaManager, PlatformCredentials, PostContent};

let manager = SocialMediaManager::new();

// Register account
let creds = PlatformCredentials {
    platform: "facebook".to_string(),
    client_id: "...".to_string(),
    client_secret: "...".to_string(),
    access_token: "...".to_string(),
    refresh_token: None,
    expires_at: None,
    account_id: "page_123".to_string(),
    account_name: "My Page".to_string(),
    account_type: AccountType::Business,
};
manager.register_account(creds).await?;

// Post content
let content = PostContent {
    text: "Hello from BizClaw!".to_string(),
    media_urls: vec![],
    link_url: Some("https://example.com".to_string()),
    link_preview_text: None,
    scheduled_time: None,
};

let result = manager.post("facebook", "page_123", content).await?;
println!("Posted: {}", result.permalink);
```

## API Reference

### SocialMediaManager

#### `new()`
Tạo instance mới.

#### `register_account(creds)`
Đăng ký tài khoản mạng xã hội.

#### `post(platform, account_id, content)`
Đăng bài lên platform.

#### `get_comments(platform, account_id, post_id)`
Lấy danh sách bình luận.

#### `get_dms(platform, account_id)`
Lấy tin nhắn trực tiếp (Facebook, Instagram).

#### `get_insights(platform, account_id, post_id)`
Lấy thống kê bài viết.

### PlatformCredentials

```rust
pub struct PlatformCredentials {
    pub platform: String,           // "facebook", "instagram", etc.
    pub client_id: String,          // OAuth client ID
    pub client_secret: String,      // OAuth client secret
    pub access_token: String,       // Platform access token
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub account_id: String,        // Page/Account ID on platform
    pub account_name: String,      // Display name
    pub account_type: AccountType, // Personal, Business, Creator
}
```

### PostContent

```rust
pub struct PostContent {
    pub text: String,                    // Post text content
    pub media_urls: Vec<String>,         // Image/video URLs
    pub link_url: Option<String>,       // Link to include
    pub link_preview_text: Option<String>,
    pub scheduled_time: Option<DateTime<Utc>>,  // For scheduled posts
}
```

### PostResponse

```rust
pub struct PostResponse {
    pub post_id: String,          // Platform-specific post ID
    pub platform: String,         // Platform name
    pub permalink: String,         // Direct URL to post
    pub posted_at: DateTime<Utc>,  // When it was posted
}
```

### Insights

```rust
pub struct Insights {
    pub impressions: u64,    // Số lần hiển thị
    pub reach: u64,          // Số người tiếp cận
    pub engagements: u64,     // Tương tác
    pub likes: u64,          // Reactions
    pub comments: u64,       // Bình luận
    pub shares: u64,         // Chia sẻ
    pub saves: u64,           // Lưu lại
    pub clicks: u64,         // Số click
    pub followers: Option<u64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}
```

## OAuth Setup

### Facebook/Instagram

1. Tạo app tại [Meta Developer Console](https://developers.facebook.com)
2. Thêm products: Facebook Login, Instagram Graph API
3. Get Page Access Token via Graph API Explorer

### LinkedIn

1. Tạo app tại [LinkedIn Developers](https://developer.linkedin.com)
2. Configure OAuth 2.0 permissions
3. Get access token via OAuth flow

### TikTok

1. Tạo app tại [TikTok Developers](https://developers.tiktok.com)
2. Request content posting permissions
3. Get access token

### Google Business

1. Create project at [Google Cloud Console](https://console.cloud.google.com)
2. Enable My Business API
3. Setup OAuth consent screen
4. Get access token

## Configuration

Thêm vào `~/.bizclaw/config.toml`:

```toml
[social]
# Facebook
facebook_page_id = ""
facebook_access_token = ""

# Instagram
instagram_user_id = ""
instagram_access_token = ""

# LinkedIn
linkedin_company_id = ""
linkedin_access_token = ""

# TikTok
tiktok_client_key = ""
tiktok_access_token = ""

# YouTube
youtube_client_id = ""
youtube_access_token = ""

# Pinterest
pinterest_board_id = ""
pinterest_access_token = ""

# Threads
threads_user_id = ""
threads_access_token = ""

# Bluesky
bluesky_handle = ""
bluesky_app_password = ""

# Google Business
google_business_account_id = ""
google_business_access_token = ""
```

## Error Handling

```rust
match manager.post("facebook", "page_123", content).await {
    Ok(response) => println!("Posted: {}", response.permalink),
    Err(e) => {
        eprintln!("Post failed: {}", e);
        // Handle: retry, notify admin, etc.
    }
}
```

## Testing

```bash
cargo test -p bizclaw-social
```
