# BizClaw Social

Social media integrations for BizClaw platform.

## Features

- **Multi-platform support**: Zalo OA, TikTok, Facebook, Instagram, Shopee
- **Async API clients**: Full async/await support
- **Scheduler**: Job queue với scheduling capabilities
- **Content adaptation**: Auto-format content for each platform
- **Type-safe**: Full Rust type safety

## Usage

### Zalo OA

```rust
use bizclaw_social::{ZaloClient, SocialContent, Platform};

let client = ZaloClient::new("your_access_token".to_string());

// Send message
client.send_text_message("user_id", "Hello from BizClaw!").await?;

// Get page info
let info = client.get_page_info().await?;
println!("Followers: {}", info.followers_count);
```

### TikTok

```rust
use bizclaw_social::{TikTokClient, TikTokShopClient};

let client = TikTokClient::new("client_key".to_string(), "client_secret".to_string());

// Get auth URL
let auth_url = client.generate_auth_url("https://myapp.com/callback", "state");

// Get user info
let user = client.get_user_info().await?;
```

### Multi-platform Posting

```rust
use bizclaw_social::{
    MultiPlatformPoster, SocialContent, Platform, ZaloClient, FacebookClient
};

let poster = MultiPlatformPoster::new()
    .with_zalo(ZaloClient::new("zalo_token".to_string()))
    .with_facebook(FacebookClient::new("fb_token".to_string(), Some("page_id".to_string())));

let content = SocialContent::builder()
    .text("Check out our new product!")
    .hashtags(vec!["bizclaw", "startup"])
    .platform(Platform::ZaloOA)
    .build();

// Broadcast to all platforms
let results = poster.broadcast(content).await;

for (platform, result) in results {
    match result {
        Ok(post_id) => println!("Posted to {:?}: {}", platform, post_id),
        Err(e) => eprintln!("Failed to post to {:?}: {}", platform, e),
    }
}
```

### Scheduling

```rust
use bizclaw_social::{SocialScheduler, ScheduledPost, Platform, SocialContent};
use chrono::{Duration, Utc};

let scheduler = SocialScheduler::new();

// Schedule a post
let content = SocialContent::builder()
    .text("Scheduled post!")
    .platform(Platform::ZaloOA)
    .build();

let post = ScheduledPost::new(
    Platform::ZaloOA,
    content,
    Utc::now() + Duration::hours(2),
);

let post_id = scheduler.schedule(post)?;

// Get pending posts
let pending = scheduler.get_pending();
```

## Platform Support

| Platform | Status | Features |
|----------|--------|----------|
| Zalo OA | ✅ Complete | Messages, images, QR links, followers |
| TikTok | ✅ Complete | Auth, user info, video upload, publishing |
| TikTok Shop | ✅ Complete | Products, orders |
| Facebook | ✅ Complete | Pages, posts, photos, insights |
| Instagram | ✅ Complete | Business accounts, media, carousel |
| Shopee | 🚧 Coming | Products, orders |

## License

MIT
