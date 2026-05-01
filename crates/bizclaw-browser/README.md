# BizClaw Browser - AI Browser Harness

Browser automation toolkit cho AI agents với CDP và Lightpanda support.

## Features

- **CDP Client**: Chrome DevTools Protocol integration
- **Lightpanda**: Native AI-optimized browser (Zig-based)
- **Stealth Mode**: Anti-detection với fingerprint spoofing
- **Human Behavior**: Simulate human interaction patterns
- **Captcha Solving**: Multiple provider integration
- **Proxy Rotation**: Automatic proxy management

## Quick Start

### CDP Chrome

```rust
use bizclaw_browser::CdpClient;

let client = CdpClient::connect("ws://localhost:9222").await?;
let page = client.goto("https://example.com").await?;
let content = page.content().await?;
```

### Lightpanda (Recommended for AI)

```rust
use bizclaw_browser::{LightpandaClient, LightpandaConfig};

let mut client = LightpandaClient::new(LightpandaConfig::default());
client.start().await?;

let session = client.new_session().await?;
client.navigate("https://example.com").await?;

let semantic_tree = client.get_semantic_tree().await?;
let markdown = client.get_markdown().await?;
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Hermes Agent                        │
└─────────────────────┬───────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────┐
│                Browser Tools                         │
├─────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │ CDP Client  │  │ Lightpanda  │  │   Stealth   │ │
│  │   (Chrome)  │  │    (Zig)    │  │   Manager   │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘ │
│         │                │                │         │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐ │
│  │   CDP       │  │  Native     │  │  Fingerprint│ │
│  │   Events    │  │  Web APIs   │  │  Spoofing   │ │
│  └─────────────┘  └─────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────┘
```

## Modules

### CDP Client

Chrome DevTools Protocol client cho browser automation:

```rust
use bizclaw_browser::{CdpClient, SessionConfig};

let config = SessionConfig {
    viewport: ViewportConfig {
        width: 1920,
        height: 1080,
        ..Default::default()
    },
    user_agent: Some(custom_ua),
    ..Default::default()
};

let client = CdpClient::connect_with_config("ws://localhost:9222", config).await?;
```

### Lightpanda

Native browser cho AI agents - nhẹ hơn và nhanh hơn:

- **16x less RAM** so với Chrome
- **9x faster** page loads
- Native MCP server
- Semantic tree extraction
- Markdown generation

Xem [docs/LIGHTPANDA.md](docs/LIGHTPANDA.md) để biết thêm.

### Stealth Mode

Anti-detection measures:

```rust
use bizclaw_browser::{StealthConfig, StealthManager};

let config = StealthConfig {
    enabled: true,
    remove_webdriver: true,
    spoof_fingerprint: true,
    canvas_noise: true,
    webgl_spoofing: true,
    human_delays: true,
    ..Default::default()
};

let stealth = StealthManager::new(cdp_client, config);
stealth.apply_all().await?;
```

### Human Behavior Engine

Simulate human-like interaction:

```rust
use bizclaw_browser::{HumanBehaviorEngine, HumanBehaviorConfig};

let config = HumanBehaviorConfig {
    min_keystroke_delay_ms: 50,
    max_keystroke_delay_ms: 150,
    min_click_delay_ms: 100,
    max_click_delay_ms: 400,
    ..Default::default()
};

let engine = HumanBehaviorEngine::new(config);
```

### Captcha Solving

Multi-provider captcha solving:

```rust
use bizclaw_browser::{CaptchaSolver, CaptchaProviderConfig};

let solver = CaptchaSolver::new(CaptchaProviderConfig {
    provider: LlmProvider::OpenAI,
    api_key: "your-api-key",
    ..Default::default()
});

let solution = solver.solve(image_base64).await?;
```

## Browser Options

| Browser | Use Case | Pros | Cons |
|---------|----------|------|------|
| **Lightpanda** | AI Agents, Scraping | Fast, Light, MCP Native | Limited ecosystem |
| **Chrome CDP** | Full automation | Full Chrome features | Heavy resource |
| **Firefox CDP** | Alternative | Different fingerprint | Less common |

## Installation

```toml
[dependencies]
bizclaw-browser = { path = "../crates/bizclaw-browser" }
```

## Examples

Xem `examples/` directory cho complete examples:

- `cdp_test.rs` - CDP client demo
- `stealth_eval.rs` - Stealth mode evaluation

## Testing

```bash
cargo test -p bizclaw-browser
```

## License

MIT
