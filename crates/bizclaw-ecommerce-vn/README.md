# BizClaw E-commerce Vietnam MCP Gateway

Vietnamese e-commerce MCP integrations: **Sapo**, **Haravan**, **KiotViet**, **LadiSales**

## Features

- **Multi-platform support**: Connect to 4 major Vietnamese e-commerce platforms
- **Unified Customer View**: Merge customer data across platforms
- **Cross-platform Analytics**: Combined analytics and benchmarking
- **Credential Management**: AES-256-GCM encrypted credential storage
- **Real-time Inventory Sync**: Keep stock levels synchronized

## Supported Platforms

| Platform | Products | Tools | Status |
|----------|----------|-------|--------|
| **Sapo** | 105 | POS + Online + Web + Analytics | ✅ Production |
| **Haravan** | 70 | Smart Tools + RFM Analysis | ✅ Production |
| **KiotViet** | 36 | POS + Retail + F&B | ✅ Production |
| **LadiSales** | 9 | Products + Orders + Customers | 🔄 Beta |

## Quick Start

```rust
use bizclaw_ecommerce_vn::VnEcomGateway;

let gateway = VnEcomGateway::new();

// Connect to platforms
gateway.connect_sapo("mystore", "api_key", "api_secret").await?;
gateway.connect_haravan("access_token").await?;
gateway.connect_kiotviet("client_id", "client_secret", "retailer").await?;

// Get unified orders
let orders = gateway.get_all_orders(None).await?;

// Search customers across platforms
let customers = gateway.search_customer("0912").await?;
```

## Modules

- `gateway` - Main gateway for managing connections
- `credentials` - Secure credential storage
- `analytics` - Cross-platform analytics
- `unified` - Unified customer view
- `sapo` - Sapo adapter
- `haravan` - Haravan adapter
- `kiotviet` - KiotViet adapter
- `ladisales` - LadiSales adapter

## MCP Integration

Each platform can be used as an MCP server:

```bash
# Sapo
npx -y sapo-mcp@latest --mode=pos-online,web,analytics

# Haravan
npx -y haravan-mcp mcp -t <token>

# KiotViet
node dist/cli.js mcp --client-id X --client-secret Y --retailer Z
```

## License

MIT
