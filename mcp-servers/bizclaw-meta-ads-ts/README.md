# BizClaw Meta Ads MCP - TypeScript

Meta Facebook/Instagram Ads MCP Server cho doanh nghiệp Việt Nam.

## Features

- **Campaign Management**: Tạo, xem, cập nhật campaigns
- **Ad Set Management**: Quản lý targeting và budget
- **Ad Management**: Tạo và theo dõi quảng cáo
- **Lead Generation**: Tạo form thu thập leads
- **Custom Audiences**: Tạo và quản lý đối tượng mục tiêu
- **Analytics**: Báo cáo chi tiết và benchmark

## Installation

```bash
npm install
npm run build
```

## Configuration

1. Tạo Meta App tại [Meta Developers](https://developers.facebook.com/)
2. Lấy Access Token từ Graph API Explorer
3. Lấy Ad Account ID (format: `act_XXXXXX`)

## Usage

```bash
# Start server
npm start

# Development mode
npm run dev
```

## MCP Tools

### Campaign Management

```typescript
// Configure credentials
await mcp.callTool("configure_meta_ads", {
  accessToken: "YOUR_ACCESS_TOKEN",
  adAccountId: "act_123456789"
});

// Get campaigns
await mcp.callTool("get_campaigns", {
  status: "ACTIVE",
  objective: "CONVERSIONS"
});

// Create campaign
await mcp.callTool("create_campaign", {
  name: "Summer Sale 2024",
  objective: "CONVERSIONS",
  dailyBudget: 500000
});
```

### Ad Set Management

```typescript
// Create ad set
await mcp.callTool("create_adset", {
  campaignId: "CAMP_ID",
  name: "Target 25-35 Female",
  optimizationGoal: "OFFSITE_CONVERSIONS",
  billingEvent: "IMPRESSIONS",
  dailyBudget: 200000
});
```

### Ad Management

```typescript
// Create ad with creative
await mcp.callTool("create_ad", {
  adsetId: "ADSET_ID",
  creative: {
    object_type: "PAGE",
    page_id: "PAGE_ID"
  }
});
```

### Lead Generation

```typescript
// Create lead form
await mcp.callTool("create_lead_form", {
  name: "Đăng ký tư vấn",
  headline: "Nhận tư vấn miễn phí",
  description: "Điền thông tin để nhận tư vấn",
  ctaType: "SIGN_UP",
  fields: ["email", "full_name", "phone_number"],
  privacyUrl: "https://example.com/privacy"
});

// Get leads
await mcp.callTool("get_leads", {
  formId: "FORM_ID",
  limit: 100
});
```

### Analytics

```typescript
// Get performance report
await mcp.callTool("get_ad_report", {
  datePreset: "last_30d",
  breakdowns: ["age", "gender"]
});

// Get benchmarks
await mcp.callTool("get_performance_benchmark", {
  industry: "ecommerce",
  objective: "CONVERSIONS"
});
```

## Vietnamese Market Features

- Định dạng tiền tệ VND
- Benchmarks cho thị trường Việt Nam
- Target Vietnamese demographics
- CPA optimization cho local products

## License

MIT
