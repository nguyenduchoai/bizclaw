# Lightpanda Browser Integration

## Overview

Lightpanda là browser engine mới được viết bằng Zig, được thiết kế đặc biệt cho AI agents và automation tasks. Không phải Chromium fork - được xây dựng từ đầu cho máy.

## Key Advantages

| Metric | Chrome | Lightpanda | Improvement |
|--------|-------|------------|-------------|
| RAM Usage | ~800MB | ~50MB | **16x less** |
| Startup Time | ~3s | ~0.3s | **10x faster** |
| Page Load | Baseline | 9x faster | **9x faster** |

## Features

- **Native MCP Server**: Built-in MCP protocol support
- **Semantic Tree**: HTML structure as semantic tree
- **Markdown Extraction**: Automatic content extraction
- **CDP Support**: Compatible with Puppeteer/Playwright
- **No UI Rendering**: Pure headless operation

## Usage

```rust
use bizclaw_browser::{LightpandaClient, LightpandaConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = LightpandaConfig::default();
    let mut client = LightpandaClient::new(config);

    client.start().await?;

    let session_id = client.new_session().await?;

    client.navigate("https://example.com").await?;

    let semantic_tree = client.get_semantic_tree().await?;
    let markdown = client.get_markdown().await?;

    client.stop().await?;
    Ok(())
}
```

## MCP Tools

Lightpanda cung cấp native MCP tools:

```rust
// List available MCP tools
let tools = client.list_mcp_tools().await?;

// Call MCP tool
let result = client.call_mcp_tool("extract_data", json!({
    "selector": ".product-item"
})).await?;
```

## Semantic Tree

```rust
let tree = client.get_semantic_tree().await?;
println!("{}", tree.role);  // "button", "link", etc.
println!("{}", tree.name);  // Accessible name
println!("{}", tree.interactive);  // true/false
```

## AI Agent Workflows

### Web Scraping

```rust
async fn scrape_product_page(client: &mut LightpandaClient, url: &str) -> Result<Product> {
    client.navigate(url).await?;

    let markdown = client.get_markdown().await?;

    let semantic_tree = client.get_semantic_tree().await?;
    let price = semantic_tree.find_by_role("text")
        .and_then(|n| n.value.parse::<f64>().ok());

    Ok(Product { url: url.to_string(), markdown, price })
}
```

### Form Automation

```rust
async fn fill_form(client: &mut LightpandaClient) -> Result<()> {
    client.navigate("https://example.com/form").await?;

    client.fill("input[name='email']", "test@example.com").await?;
    client.fill("input[name='phone']", "0912345678").await?;
    client.click("button[type='submit']").await?;

    client.wait_for_navigation(5000).await?;
    Ok(())
}
```

### Multi-page Crawling

```rust
async fn crawl_site(client: &mut LightpandaClient, urls: Vec<String>) -> Result<Vec<Page>> {
    let mut pages = Vec::new();

    for url in urls {
        client.navigate(&url).await?;

        let markdown = client.get_markdown().await?;
        let cookies = client.get_cookies().await?;

        pages.push(Page { url, markdown, cookies });
    }

    Ok(pages)
}
```

## Configuration

```rust
let config = LightpandaConfig {
    port: 9222,
    cdp_port: 9223,
    mcp_enabled: true,
    mcp_port: Some(3030),
    headless: true,
    user_agent: Some("Mozilla/5.0...".to_string()),
    viewport: Some(ViewportConfig {
        width: 1920,
        height: 1080,
        device_scale_factor: Some(1.0),
    }),
    proxy: None,
};
```

## Stealth Mode

Kết hợp với stealth module để tránh detection:

```rust
use bizclaw_browser::{LightpandaClient, StealthConfig};

let mut client = LightpandaClient::new(LightpandaConfig::default());
client.start().await?;

let stealth = StealthManager::new(
    /* CDP client */,
    StealthConfig::default()
);

stealth.apply_all().await?;
```

## Comparison with CDP Chrome

| Feature | Chrome CDP | Lightpanda |
|---------|------------|-------------|
| Memory | High | Very Low |
| Speed | Fast | Faster |
| MCP Native | No | Yes |
| Semantic Tree | No | Yes |
| Markdown Export | Manual | Auto |
| Setup Complexity | Medium | Low |

## Installation

```bash
# Install Lightpanda binary
curl -fsSL https://get.lightpanda.io | bash

# Or via cargo
cargo install lightpanda
```

## Best Practices

1. **Session Management**: Reuse sessions when possible
2. **Cookie Persistence**: Save cookies between sessions
3. **Viewport**: Match target device viewport
4. **Error Handling**: Handle timeout và navigation errors
5. **Resource Cleanup**: Always call `stop()` when done

## License

MIT
