import { Tool } from "@modelcontextprotocol/sdk/types.js";

export const TOOL_DESCRIPTIONS = {
  configure_meta_ads: "Configure Meta Ads API credentials (access token và ad account ID)",
  get_campaigns: "Get danh sách campaigns từ Meta Ads",
  get_campaign_insights: "Get insights/performance data cho một campaign",
  create_campaign: "Tạo mới một Meta Ads campaign",
  create_adset: "Tạo mới một ad set trong campaign",
  get_adsets: "Get danh sách ad sets",
  create_ad: "Tạo mới một ad",
  get_ads: "Get danh sách ads",
  get_ad_insights: "Get performance insights cho ads",
  get_ad_creative: "Get creative details của một ad",
  create_lead_form: "Tạo lead form cho lead generation campaigns",
  get_leads: "Get danh sách leads từ một form",
  get_custom_audiences: "Get danh sách custom audiences",
  create_custom_audience: "Tạo mới custom audience",
  get_pixel_events: "Get events từ Meta Pixel",
  get_ad_report: "Get báo cáo chi tiết cho ad account",
  get_performance_benchmark: "Get performance benchmarks cho thị trường Việt Nam",
  get_budget_optimization: "Get budget optimization recommendations",
};

export const tools: Tool[] = [
  {
    name: "configure_meta_ads",
    description: TOOL_DESCRIPTIONS.configure_meta_ads,
    inputSchema: {
      type: "object",
      properties: {
        accessToken: {
          type: "string",
          description: "Meta Graph API Access Token",
        },
        adAccountId: {
          type: "string",
          description: "Ad Account ID (format: act_XXXXXX)",
        },
      },
      required: ["accessToken", "adAccountId"],
    },
  },
  {
    name: "get_campaigns",
    description: TOOL_DESCRIPTIONS.get_campaigns,
    inputSchema: {
      type: "object",
      properties: {
        status: {
          type: "string",
          description: "Filter by status (ACTIVE, PAUSED, ARCHIVED)",
        },
        objective: {
          type: "string",
          description: "Filter by objective (CONVERSIONS, LINK_CLICKS, etc.)",
        },
        limit: {
          type: "number",
          description: "Số lượng campaigns trả về (mặc định: 50)",
        },
      },
    },
  },
  {
    name: "get_campaign_insights",
    description: TOOL_DESCRIPTIONS.get_campaign_insights,
    inputSchema: {
      type: "object",
      properties: {
        campaignId: {
          type: "string",
          description: "Campaign ID cần lấy insights",
        },
        datePreset: {
          type: "string",
          description: "Date preset (last_7d, last_30d, last_90d, this_month)",
        },
        timeRange: {
          type: "object",
          description: "Custom time range",
          properties: {
            since: {
              type: "string",
              description: "Start date (YYYY-MM-DD)",
            },
            until: {
              type: "string",
              description: "End date (YYYY-MM-DD)",
            },
          },
        },
      },
      required: ["campaignId"],
    },
  },
  {
    name: "create_campaign",
    description: TOOL_DESCRIPTIONS.create_campaign,
    inputSchema: {
      type: "object",
      properties: {
        name: {
          type: "string",
          description: "Tên campaign",
        },
        objective: {
          type: "string",
          description: "Campaign objective (CONVERSIONS, LINK_CLICKS, REACH, VIDEO_VIEWS, LEAD_GENERATION)",
        },
        status: {
          type: "string",
          description: "Initial status (ACTIVE, PAUSED)",
        },
        buyingType: {
          type: "string",
          description: "Buying type (AUCTION, RESERVED)",
        },
        dailyBudget: {
          type: "number",
          description: "Daily budget in VND",
        },
        lifetimeBudget: {
          type: "number",
          description: "Lifetime budget in VND",
        },
      },
      required: ["name", "objective"],
    },
  },
  {
    name: "create_adset",
    description: TOOL_DESCRIPTIONS.create_adset,
    inputSchema: {
      type: "object",
      properties: {
        campaignId: {
          type: "string",
          description: "Parent campaign ID",
        },
        name: {
          type: "string",
          description: "Tên ad set",
        },
        optimizationGoal: {
          type: "string",
          description: "Optimization goal (OFFSITE_CONVERSIONS, LINK_CLICKS, REACH, etc.)",
        },
        billingEvent: {
          type: "string",
          description: "Billing event (IMPRESSIONS, LINK_CLICKS, ONSITE_CONVERSIONS)",
        },
        bidAmount: {
          type: "number",
          description: "Bid amount in VND",
        },
        dailyBudget: {
          type: "number",
          description: "Daily budget in VND",
        },
        lifetimeBudget: {
          type: "number",
          description: "Lifetime budget in VND",
        },
        startTime: {
          type: "string",
          description: "Start time (ISO 8601)",
        },
        endTime: {
          type: "string",
          description: "End time (ISO 8601)",
        },
        targeting: {
          type: "object",
          description: "Targeting configuration",
        },
      },
      required: ["campaignId", "name", "optimizationGoal", "billingEvent"],
    },
  },
  {
    name: "get_adsets",
    description: TOOL_DESCRIPTIONS.get_adsets,
    inputSchema: {
      type: "object",
      properties: {
        campaignId: {
          type: "string",
          description: "Filter by campaign ID",
        },
        limit: {
          type: "number",
          description: "Số lượng ad sets trả về",
        },
      },
    },
  },
  {
    name: "create_ad",
    description: TOOL_DESCRIPTIONS.create_ad,
    inputSchema: {
      type: "object",
      properties: {
        adsetId: {
          type: "string",
          description: "Parent ad set ID",
        },
        creative: {
          type: "object",
          description: "Ad creative object",
        },
        trackingSpecs: {
          type: "object",
          description: "Tracking specifications",
        },
        status: {
          type: "string",
          description: "Ad status (ACTIVE, PAUSED)",
        },
      },
      required: ["adsetId", "creative"],
    },
  },
  {
    name: "get_ads",
    description: TOOL_DESCRIPTIONS.get_ads,
    inputSchema: {
      type: "object",
      properties: {
        adsetId: {
          type: "string",
          description: "Filter by ad set ID",
        },
        limit: {
          type: "number",
          description: "Số lượng ads trả về",
        },
      },
    },
  },
  {
    name: "get_ad_insights",
    description: TOOL_DESCRIPTIONS.get_ad_insights,
    inputSchema: {
      type: "object",
      properties: {
        adId: {
          type: "string",
          description: "Ad ID (optional, nếu không có sẽ lấy insights của toàn bộ ad account)",
        },
        datePreset: {
          type: "string",
          description: "Date preset",
        },
        timeRange: {
          type: "object",
          properties: {
            since: { type: "string" },
            until: { type: "string" },
          },
        },
        level: {
          type: "string",
          description: "Level breakdown (campaign, adset, ad)",
        },
      },
    },
  },
  {
    name: "get_ad_creative",
    description: TOOL_DESCRIPTIONS.get_ad_creative,
    inputSchema: {
      type: "object",
      properties: {
        adId: {
          type: "string",
          description: "Ad ID cần lấy creative",
        },
      },
      required: ["adId"],
    },
  },
  {
    name: "create_lead_form",
    description: TOOL_DESCRIPTIONS.create_lead_form,
    inputSchema: {
      type: "object",
      properties: {
        name: {
          type: "string",
          description: "Tên form",
        },
        headline: {
          type: "string",
          description: "Form headline",
        },
        description: {
          type: "string",
          description: "Form description",
        },
        ctaType: {
          type: "string",
          description: "Call to action type (SUBSCRIBE, SIGN_UP, GET_QUOTE, etc.)",
        },
        fields: {
          type: "array",
          items: { type: "string" },
          description: "Form fields (email, phone_number, full_name, etc.)",
        },
        privacyUrl: {
          type: "string",
          description: "Privacy policy URL",
        },
      },
      required: ["name", "headline", "description", "ctaType", "fields", "privacyUrl"],
    },
  },
  {
    name: "get_leads",
    description: TOOL_DESCRIPTIONS.get_leads,
    inputSchema: {
      type: "object",
      properties: {
        formId: {
          type: "string",
          description: "Lead form ID",
        },
        limit: {
          type: "number",
          description: "Số lượng leads trả về",
        },
      },
      required: ["formId"],
    },
  },
  {
    name: "get_custom_audiences",
    description: TOOL_DESCRIPTIONS.get_custom_audiences,
    inputSchema: {
      type: "object",
      properties: {
        limit: {
          type: "number",
          description: "Số lượng audiences trả về",
        },
      },
    },
  },
  {
    name: "create_custom_audience",
    description: TOOL_DESCRIPTIONS.create_custom_audience,
    inputSchema: {
      type: "object",
      properties: {
        name: {
          type: "string",
          description: "Audience name",
        },
        description: {
          type: "string",
          description: "Audience description",
        },
        subtype: {
          type: "string",
          description: "Audience type (CUSTOM, WEBSITE, APP, ENGAGEMENT)",
        },
        dataSource: {
          type: "object",
          description: "Data source configuration",
        },
        audienceSize: {
          type: "number",
          description: "Estimated audience size",
        },
      },
      required: ["name", "subtype"],
    },
  },
  {
    name: "get_pixel_events",
    description: TOOL_DESCRIPTIONS.get_pixel_events,
    inputSchema: {
      type: "object",
      properties: {
        pixelId: {
          type: "string",
          description: "Meta Pixel ID",
        },
        startTime: {
          type: "string",
          description: "Start time (ISO 8601)",
        },
        endTime: {
          type: "string",
          description: "End time (ISO 8601)",
        },
        limit: {
          type: "number",
          description: "Số lượng events trả về",
        },
      },
      required: ["pixelId"],
    },
  },
  {
    name: "get_ad_report",
    description: TOOL_DESCRIPTIONS.get_ad_report,
    inputSchema: {
      type: "object",
      properties: {
        datePreset: {
          type: "string",
          description: "Date preset",
        },
        timeRange: {
          type: "object",
          properties: {
            since: { type: "string" },
            until: { type: "string" },
          },
        },
        breakdowns: {
          type: "array",
          items: { type: "string" },
          description: "Breakdowns (age, gender, placement, device)",
        },
      },
    },
  },
  {
    name: "get_performance_benchmark",
    description: TOOL_DESCRIPTIONS.get_performance_benchmark,
    inputSchema: {
      type: "object",
      properties: {
        industry: {
          type: "string",
          description: "Industry vertical (ecommerce, finance, real_estate, etc.)",
        },
        objective: {
          type: "string",
          description: "Campaign objective",
        },
      },
    },
  },
  {
    name: "get_budget_optimization",
    description: TOOL_DESCRIPTIONS.get_budget_optimization,
    inputSchema: {
      type: "object",
      properties: {
        campaignId: {
          type: "string",
          description: "Campaign ID",
        },
        targetCpa: {
          type: "number",
          description: "Target CPA in VND",
        },
        dailyBudget: {
          type: "number",
          description: "Suggested daily budget in VND",
        },
      },
      required: ["campaignId"],
    },
  },
];
