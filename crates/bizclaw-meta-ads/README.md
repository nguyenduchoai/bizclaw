# BizClaw Meta Ads Integration

Meta Facebook/Instagram Ads API integration for Vietnamese businesses.

## Features

- **Campaign Management**: Create, pause, resume campaigns
- **Ad Set Targeting**: Precise audience targeting
- **Ad Creative**: Image/video/carousel ads
- **Performance Analytics**: Real-time insights and reporting
- **Audience Management**: Custom audiences, lookalike audiences

## Quick Start

```rust
use bizclaw_meta_ads::{MetaAdsConfig, CampaignManager};

let config = MetaAdsConfig::new("access_token", "act_123456789");
let manager = CampaignManager::new(config);

// Create campaign
let campaign = manager.create_campaign(CreateCampaignRequest {
    name: "Spring Sale 2025".to_string(),
    objective: "CONVERSIONS".to_string(),
    status: "PAUSED".to_string(),
    daily_budget: Some(100000.0),
    lifetime_budget: None,
    start_time: Some("2025-03-01T00:00:00Z".to_string()),
    end_time: None,
}).await?;
```

## Ad Creative Examples

```rust
use bizclaw_meta_ads::{AdCreativeInput, CreativeType};

let creative = AdCreativeInput {
    name: "Spring Sale Ad".to_string(),
    creative_type: "image".to_string(),
    title: Some("Giảm giá 30%!".to_string()),
    body: Some("Mua ngay hôm nay để nhận ưu đãi!".to_string()),
    image_url: Some("https://example.com/sale.jpg".to_string()),
    call_to_action_type: Some("SHOP_NOW".to_string()),
    page_id: Some("123456789".to_string()),
    link: Some("https://shop.example.com".to_string()),
    video_url: None,
    product_set_id: None,
};
```

## Modules

- `types` - Core data types
- `config` - Configuration and settings
- `campaigns` - Campaign management
- `adsets` - Ad set targeting
- `ads` - Ad creative management
- `insights` - Performance analytics
- `audiences` - Audience management

## API Documentation

https://developers.facebook.com/docs/marketing-apis/

## Meta Platforms

- Facebook Ads
- Instagram Ads
- Messenger Ads
- Audience Network

## License

MIT
