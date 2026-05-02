import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ListPromptsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { MetaAdsClient } from "./client.js";
import { tools, TOOL_DESCRIPTIONS } from "./tools.js";

class MetaAdsMCPServer {
  private server: Server;
  private client: MetaAdsClient;
  private adAccountId: string | null = null;

  constructor() {
    this.server = new Server(
      {
        name: "bizclaw-meta-ads",
        version: "0.1.0",
      },
      {
        capabilities: {
          tools: {},
          resources: {},
        },
      }
    );

    this.client = new MetaAdsClient();

    this.setupHandlers();
  }

  private setupHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools,
    }));

    this.server.setRequestHandler(ListResourcesRequestSchema, async () => ({
      resources: [],
    }));

    this.server.setRequestHandler(ListPromptsRequestSchema, async () => ({
      prompts: [],
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args = {} } = request.params;

      try {
        switch (name) {
          case "configure_meta_ads":
            return await this.handleConfigure(args);

          case "get_campaigns":
            return await this.handleGetCampaigns(args);

          case "get_campaign_insights":
            return await this.handleGetCampaignInsights(args);

          case "create_campaign":
            return await this.handleCreateCampaign(args);

          case "create_adset":
            return await this.handleCreateAdSet(args);

          case "get_adsets":
            return await this.handleGetAdSets(args);

          case "create_ad":
            return await this.handleCreateAd(args);

          case "get_ads":
            return await this.handleGetAds(args);

          case "get_ad_insights":
            return await this.handleGetAdInsights(args);

          case "get_ad_creative":
            return await this.handleGetAdCreative(args);

          case "create_lead_form":
            return await this.handleCreateLeadForm(args);

          case "get_leads":
            return await this.handleGetLeads(args);

          case "get_custom_audiences":
            return await this.handleGetCustomAudiences(args);

          case "create_custom_audience":
            return await this.handleCreateCustomAudience(args);

          case "get_pixel_events":
            return await this.handleGetPixelEvents(args);

          case "get_ad_report":
            return await this.handleGetAdReport(args);

          case "get_performance_benchmark":
            return await this.handleGetPerformanceBenchmark(args);

          case "get_budget_optimization":
            return await this.handleGetBudgetOptimization(args);

          default:
            throw new Error(`Unknown tool: ${name}`);
        }
      } catch (error) {
        return {
          content: [
            {
              type: "text",
              text: `Error: ${error instanceof Error ? error.message : String(error)}`,
            },
          ],
          isError: true,
        };
      }
    });
  }

  private async handleConfigure(args: Record<string, unknown>) {
    const { accessToken, adAccountId } = args as {
      accessToken: string;
      adAccountId: string;
    };

    this.client.configure(accessToken);
    this.adAccountId = adAccountId;

    return {
      content: [
        {
          type: "text",
          text: `Meta Ads MCP configured successfully!\n\n` +
            `- Access Token: ${accessToken.substring(0, 10)}...\n` +
            `- Ad Account ID: ${adAccountId}\n\n` +
            `Available tools:\n` +
            `- Campaign Management: get_campaigns, create_campaign, get_campaign_insights\n` +
            `- Ad Set Management: get_adsets, create_adset\n` +
            `- Ad Management: get_ads, create_ad, get_ad_insights\n` +
            `- Lead Generation: create_lead_form, get_leads\n` +
            `- Audiences: get_custom_audiences, create_custom_audience\n` +
            `- Analytics: get_ad_report, get_performance_benchmark, get_budget_optimization`,
        },
      ],
    };
  }

  private async handleGetCampaigns(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { status, objective, limit } = args as {
      status?: string;
      objective?: string;
      limit?: number;
    };

    const campaigns = await this.client.getCampaigns(
      this.adAccountId!,
      { status, objective, limit }
    );

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(campaigns, null, 2),
        },
      ],
    };
  }

  private async handleGetCampaignInsights(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { campaignId, datePreset, timeRange } = args as {
      campaignId: string;
      datePreset?: string;
      timeRange?: { since: string; until: string };
    };

    const insights = await this.client.getCampaignInsights(
      campaignId,
      { datePreset, timeRange }
    );

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(insights, null, 2),
        },
      ],
    };
  }

  private async handleCreateCampaign(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { name, objective, status, buyingType, dailyBudget, lifetimeBudget } = args as {
      name: string;
      objective: string;
      status?: string;
      buyingType?: string;
      dailyBudget?: number;
      lifetimeBudget?: number;
    };

    const campaign = await this.client.createCampaign(this.adAccountId!, {
      name,
      objective,
      status,
      buyingType,
      dailyBudget,
      lifetimeBudget,
    });

    return {
      content: [
        {
          type: "text",
          text: `Campaign created successfully!\n\nCampaign ID: ${campaign.id}\nName: ${campaign.name}\nObjective: ${campaign.objective}\nStatus: ${campaign.status}`,
        },
      ],
    };
  }

  private async handleCreateAdSet(args: Record<string, unknown>) {
    this.ensureConfigured();

    const {
      campaignId,
      name,
      optimizationGoal,
      billingEvent,
      bidAmount,
      dailyBudget,
      lifetimeBudget,
      startTime,
      endTime,
      targeting,
    } = args as {
      campaignId: string;
      name: string;
      optimizationGoal: string;
      billingEvent: string;
      bidAmount?: number;
      dailyBudget?: number;
      lifetimeBudget?: number;
      startTime?: string;
      endTime?: string;
      targeting?: object;
    };

    const adset = await this.client.createAdSet(this.adAccountId!, {
      campaignId,
      name,
      optimizationGoal,
      billingEvent,
      bidAmount,
      dailyBudget,
      lifetimeBudget,
      startTime,
      endTime,
      targeting,
    });

    return {
      content: [
        {
          type: "text",
          text: `Ad Set created successfully!\n\nAd Set ID: ${adset.id}\nName: ${adset.name}\nOptimization Goal: ${adset.optimization_goal}`,
        },
      ],
    };
  }

  private async handleGetAdSets(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { campaignId, limit } = args as {
      campaignId?: string;
      limit?: number;
    };

    const adsets = await this.client.getAdSets(this.adAccountId!, {
      campaignId,
      limit,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(adsets, null, 2),
        },
      ],
    };
  }

  private async handleCreateAd(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { adsetId, creative, trackingSpecs, status } = args as {
      adsetId: string;
      creative: object;
      trackingSpecs?: object;
      status?: string;
    };

    const ad = await this.client.createAd(this.adAccountId!, {
      adsetId,
      creative,
      trackingSpecs,
      status,
    });

    return {
      content: [
        {
          type: "text",
          text: `Ad created successfully!\n\nAd ID: ${ad.id}\nStatus: ${ad.status}`,
        },
      ],
    };
  }

  private async handleGetAds(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { adsetId, limit } = args as {
      adsetId?: string;
      limit?: number;
    };

    const ads = await this.client.getAds(this.adAccountId!, { adsetId, limit });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(ads, null, 2),
        },
      ],
    };
  }

  private async handleGetAdInsights(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { adId, datePreset, timeRange, level } = args as {
      adId?: string;
      adsetId?: string;
      campaignId?: string;
      datePreset?: string;
      timeRange?: { since: string; until: string };
      level?: string;
    };

    const insights = await this.client.getAdInsights({
      adAccountId: this.adAccountId!,
      adId,
      datePreset,
      timeRange,
      level,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(insights, null, 2),
        },
      ],
    };
  }

  private async handleGetAdCreative(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { adId } = args as { adId: string };

    const creative = await this.client.getAdCreative(adId);

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(creative, null, 2),
        },
      ],
    };
  }

  private async handleCreateLeadForm(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { name, headline, description, ctaType, fields, privacyUrl } = args as {
      name: string;
      headline: string;
      description: string;
      ctaType: string;
      fields: string[];
      privacyUrl: string;
    };

    const form = await this.client.createLeadForm(this.adAccountId!, {
      name,
      headline,
      description,
      ctaType,
      fields,
      privacyUrl,
    });

    const formData = form as { id: string; name: string };

    return {
      content: [
        {
          type: "text",
          text: `Lead Form created successfully!\n\nForm ID: ${formData.id}\nName: ${formData.name}`,
        },
      ],
    };
  }

  private async handleGetLeads(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { formId, limit } = args as {
      formId: string;
      limit?: number;
    };

    const leads = await this.client.getLeads(formId, { limit });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(leads, null, 2),
        },
      ],
    };
  }

  private async handleGetCustomAudiences(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { limit } = args as { limit?: number };

    const audiences = await this.client.getCustomAudiences(this.adAccountId!, {
      limit,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(audiences, null, 2),
        },
      ],
    };
  }

  private async handleCreateCustomAudience(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { name, description, subtype, dataSource, audienceSize } = args as {
      name: string;
      description?: string;
      subtype: string;
      dataSource?: object;
      audienceSize?: number;
    };

    const audience = await this.client.createCustomAudience(this.adAccountId!, {
      name,
      description,
      subtype,
      dataSource,
      audienceSize,
    });

    return {
      content: [
        {
          type: "text",
          text: `Custom Audience created successfully!\n\nAudience ID: ${audience.id}\nName: ${audience.name}`,
        },
      ],
    };
  }

  private async handleGetPixelEvents(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { pixelId, startTime, endTime, limit } = args as {
      pixelId: string;
      startTime?: string;
      endTime?: string;
      limit?: number;
    };

    const events = await this.client.getPixelEvents(pixelId, {
      startTime,
      endTime,
      limit,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(events, null, 2),
        },
      ],
    };
  }

  private async handleGetAdReport(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { datePreset, timeRange, breakdowns } = args as {
      datePreset?: string;
      timeRange?: { since: string; until: string };
      breakdowns?: string[];
    };

    const report = await this.client.getAdReport(this.adAccountId!, {
      datePreset,
      timeRange,
      breakdowns,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(report, null, 2),
        },
      ],
    };
  }

  private async handleGetPerformanceBenchmark(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { industry, objective } = args as {
      industry?: string;
      objective?: string;
    };

    const benchmark = await this.client.getPerformanceBenchmark({
      industry,
      objective,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(benchmark, null, 2),
        },
      ],
    };
  }

  private async handleGetBudgetOptimization(args: Record<string, unknown>) {
    this.ensureConfigured();

    const { campaignId, targetCpa, dailyBudget } = args as {
      campaignId: string;
      targetCpa?: number;
      dailyBudget?: number;
    };

    const optimization = await this.client.getBudgetOptimization(campaignId, {
      targetCpa,
      dailyBudget,
    });

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(optimization, null, 2),
        },
      ],
    };
  }

  private ensureConfigured() {
    if (!this.client.isConfigured()) {
      throw new Error(
        "Meta Ads not configured. Please run configure_meta_ads first."
      );
    }
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("Meta Ads MCP Server running on stdio");
  }
}

const server = new MetaAdsMCPServer();
server.start().catch(console.error);
