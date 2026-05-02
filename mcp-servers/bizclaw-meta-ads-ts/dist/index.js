import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { CallToolRequestSchema, ListToolsRequestSchema, ListResourcesRequestSchema, ListPromptsRequestSchema, } from "@modelcontextprotocol/sdk/types.js";
import { MetaAdsClient } from "./client.js";
import { tools } from "./tools.js";
class MetaAdsMCPServer {
    server;
    client;
    adAccountId = null;
    constructor() {
        this.server = new Server({
            name: "bizclaw-meta-ads",
            version: "0.1.0",
        }, {
            capabilities: {
                tools: {},
                resources: {},
            },
        });
        this.client = new MetaAdsClient();
        this.setupHandlers();
    }
    setupHandlers() {
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
            }
            catch (error) {
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
    async handleConfigure(args) {
        const { accessToken, adAccountId } = args;
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
    async handleGetCampaigns(args) {
        this.ensureConfigured();
        const { status, objective, limit } = args;
        const campaigns = await this.client.getCampaigns(this.adAccountId, { status, objective, limit });
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(campaigns, null, 2),
                },
            ],
        };
    }
    async handleGetCampaignInsights(args) {
        this.ensureConfigured();
        const { campaignId, datePreset, timeRange } = args;
        const insights = await this.client.getCampaignInsights(campaignId, { datePreset, timeRange });
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(insights, null, 2),
                },
            ],
        };
    }
    async handleCreateCampaign(args) {
        this.ensureConfigured();
        const { name, objective, status, buyingType, dailyBudget, lifetimeBudget } = args;
        const campaign = await this.client.createCampaign(this.adAccountId, {
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
    async handleCreateAdSet(args) {
        this.ensureConfigured();
        const { campaignId, name, optimizationGoal, billingEvent, bidAmount, dailyBudget, lifetimeBudget, startTime, endTime, targeting, } = args;
        const adset = await this.client.createAdSet(this.adAccountId, {
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
    async handleGetAdSets(args) {
        this.ensureConfigured();
        const { campaignId, limit } = args;
        const adsets = await this.client.getAdSets(this.adAccountId, {
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
    async handleCreateAd(args) {
        this.ensureConfigured();
        const { adsetId, creative, trackingSpecs, status } = args;
        const ad = await this.client.createAd(this.adAccountId, {
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
    async handleGetAds(args) {
        this.ensureConfigured();
        const { adsetId, limit } = args;
        const ads = await this.client.getAds(this.adAccountId, { adsetId, limit });
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(ads, null, 2),
                },
            ],
        };
    }
    async handleGetAdInsights(args) {
        this.ensureConfigured();
        const { adId, datePreset, timeRange, level } = args;
        const insights = await this.client.getAdInsights({
            adAccountId: this.adAccountId,
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
    async handleGetAdCreative(args) {
        this.ensureConfigured();
        const { adId } = args;
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
    async handleCreateLeadForm(args) {
        this.ensureConfigured();
        const { name, headline, description, ctaType, fields, privacyUrl } = args;
        const form = await this.client.createLeadForm(this.adAccountId, {
            name,
            headline,
            description,
            ctaType,
            fields,
            privacyUrl,
        });
        const formData = form;
        return {
            content: [
                {
                    type: "text",
                    text: `Lead Form created successfully!\n\nForm ID: ${formData.id}\nName: ${formData.name}`,
                },
            ],
        };
    }
    async handleGetLeads(args) {
        this.ensureConfigured();
        const { formId, limit } = args;
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
    async handleGetCustomAudiences(args) {
        this.ensureConfigured();
        const { limit } = args;
        const audiences = await this.client.getCustomAudiences(this.adAccountId, {
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
    async handleCreateCustomAudience(args) {
        this.ensureConfigured();
        const { name, description, subtype, dataSource, audienceSize } = args;
        const audience = await this.client.createCustomAudience(this.adAccountId, {
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
    async handleGetPixelEvents(args) {
        this.ensureConfigured();
        const { pixelId, startTime, endTime, limit } = args;
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
    async handleGetAdReport(args) {
        this.ensureConfigured();
        const { datePreset, timeRange, breakdowns } = args;
        const report = await this.client.getAdReport(this.adAccountId, {
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
    async handleGetPerformanceBenchmark(args) {
        this.ensureConfigured();
        const { industry, objective } = args;
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
    async handleGetBudgetOptimization(args) {
        this.ensureConfigured();
        const { campaignId, targetCpa, dailyBudget } = args;
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
    ensureConfigured() {
        if (!this.client.isConfigured()) {
            throw new Error("Meta Ads not configured. Please run configure_meta_ads first.");
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
//# sourceMappingURL=index.js.map