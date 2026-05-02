import axios from "axios";
import { z } from "zod";
const CampaignSchema = z.object({
    id: z.string(),
    name: z.string(),
    objective: z.string(),
    status: z.string(),
    buying_type: z.string().optional(),
    daily_budget: z.string().optional(),
    lifetime_budget: z.string().optional(),
    created_time: z.string(),
    updated_time: z.string(),
});
const AdSetSchema = z.object({
    id: z.string(),
    name: z.string(),
    campaign_id: z.string(),
    optimization_goal: z.string(),
    billing_event: z.string(),
    bid_amount: z.number().optional(),
    daily_budget: z.string().optional(),
    lifetime_budget: z.string().optional(),
    status: z.string(),
    targeting: z.any().optional(),
    start_time: z.string().optional(),
    end_time: z.string().optional(),
});
const AdSchema = z.object({
    id: z.string(),
    name: z.string(),
    adset_id: z.string(),
    campaign_id: z.string(),
    status: z.string(),
    creative: z.any().optional(),
    tracking_specs: z.any().optional(),
    created_time: z.string(),
});
const InsightsSchema = z.object({
    spend: z.string(),
    impressions: z.string(),
    clicks: z.string(),
    ctr: z.string(),
    cpc: z.string(),
    cpm: z.string(),
    reach: z.string(),
    frequency: z.string(),
    actions: z.array(z.any()).optional(),
    cost_per_action_type: z.array(z.any()).optional(),
    date_start: z.string(),
    date_stop: z.string(),
});
const LeadSchema = z.object({
    id: z.string(),
    created_time: z.string(),
    field_data: z.array(z.any()),
});
const CustomAudienceSchema = z.object({
    id: z.string(),
    name: z.string(),
    description: z.string().optional(),
    subtype: z.string(),
    audience_size: z.number().optional(),
    delivery_status: z.any().optional(),
});
const PixelEventSchema = z.object({
    id: z.string(),
    event_time: z.string(),
    event_name: z.string(),
    action_source: z.string(),
    user_data: z.any().optional(),
    custom_data: z.any().optional(),
});
export class MetaAdsClient {
    accessToken = null;
    api = null;
    apiVersion = "v19.0";
    configure(accessToken) {
        this.accessToken = accessToken;
        this.api = axios.create({
            baseURL: `https://graph.facebook.com/${this.apiVersion}`,
            params: {
                access_token: accessToken,
            },
        });
    }
    isConfigured() {
        return this.accessToken !== null && this.api !== null;
    }
    ensureApi() {
        if (!this.api) {
            throw new Error("Client not configured. Call configure() first.");
        }
        return this.api;
    }
    async getCampaigns(adAccountId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "id",
            "name",
            "objective",
            "status",
            "buying_type",
            "daily_budget",
            "lifetime_budget",
            "created_time",
            "updated_time",
        ].join(",");
        const response = await api.get(`/${adAccountId}/campaigns`, {
            params: {
                fields,
                limit: params.limit || 50,
                ...(params.status && { filtering: JSON.stringify([{ field: "campaign.status", operator: "IN", value: params.status.split(",") }]) }),
                ...(params.objective && { filtering: JSON.stringify([{ field: "campaign.objective", operator: "IN", value: params.objective.split(",") }]) }),
            },
        });
        return CampaignSchema.array().parse(response.data.data);
    }
    async createCampaign(adAccountId, data) {
        const api = this.ensureApi();
        const params = {
            name: data.name,
            objective: data.objective,
            status: data.status || "PAUSED",
        };
        if (data.buyingType)
            params.buying_type = data.buyingType;
        if (data.dailyBudget)
            params.daily_budget = data.dailyBudget;
        if (data.lifetimeBudget)
            params.lifetime_budget = data.lifetimeBudget;
        const response = await api.post(`/${adAccountId}/campaigns`, null, {
            params,
        });
        return CampaignSchema.parse(response.data);
    }
    async getCampaignInsights(campaignId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "spend",
            "impressions",
            "clicks",
            "ctr",
            "cpc",
            "cpm",
            "reach",
            "frequency",
            "actions",
            "cost_per_action_type",
            "date_start",
            "date_stop",
        ].join(",");
        const queryParams = {
            fields,
            date_preset: params.datePreset || "last_7d",
            level: params.level || "campaign",
        };
        if (params.timeRange) {
            queryParams.time_range = JSON.stringify(params.timeRange);
        }
        const response = await api.get(`/${campaignId}/insights`, {
            params: queryParams,
        });
        return InsightsSchema.array().parse(response.data.data);
    }
    async getAdSets(adAccountId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "id",
            "name",
            "campaign_id",
            "optimization_goal",
            "billing_event",
            "bid_amount",
            "daily_budget",
            "lifetime_budget",
            "status",
            "targeting",
            "start_time",
            "end_time",
        ].join(",");
        const response = await api.get(`/${adAccountId}/adsets`, {
            params: {
                fields,
                limit: params.limit || 50,
                ...(params.campaignId && { filtering: JSON.stringify([{ field: "adset.campaign_id", operator: "EQUAL", value: params.campaignId }]) }),
            },
        });
        return AdSetSchema.array().parse(response.data.data);
    }
    async createAdSet(adAccountId, data) {
        const api = this.ensureApi();
        const params = {
            campaign_id: data.campaignId,
            name: data.name,
            optimization_goal: data.optimizationGoal,
            billing_event: data.billingEvent,
            status: "PAUSED",
        };
        if (data.bidAmount)
            params.bid_amount = data.bidAmount;
        if (data.dailyBudget)
            params.daily_budget = data.dailyBudget;
        if (data.lifetimeBudget)
            params.lifetime_budget = data.lifetimeBudget;
        if (data.startTime)
            params.start_time = data.startTime;
        if (data.endTime)
            params.end_time = data.endTime;
        if (data.targeting)
            params.targeting = JSON.stringify(data.targeting);
        const response = await api.post(`/${adAccountId}/adsets`, null, {
            params,
        });
        return AdSetSchema.parse(response.data);
    }
    async getAds(adAccountId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "id",
            "name",
            "adset_id",
            "campaign_id",
            "status",
            "creative",
            "tracking_specs",
            "created_time",
        ].join(",");
        const response = await api.get(`/${adAccountId}/ads`, {
            params: {
                fields,
                limit: params.limit || 50,
                ...(params.adsetId && { filtering: JSON.stringify([{ field: "ad.adset_id", operator: "EQUAL", value: params.adsetId }]) }),
            },
        });
        return AdSchema.array().parse(response.data.data);
    }
    async createAd(adAccountId, data) {
        const api = this.ensureApi();
        const params = {
            adset_id: data.adsetId,
            creative: JSON.stringify(data.creative),
            status: data.status || "PAUSED",
        };
        if (data.trackingSpecs) {
            params.tracking_specs = JSON.stringify(data.trackingSpecs);
        }
        const response = await api.post(`/${adAccountId}/ads`, null, {
            params,
        });
        return AdSchema.parse(response.data);
    }
    async getAdInsights(params) {
        const api = this.ensureApi();
        const fields = [
            "spend",
            "impressions",
            "clicks",
            "ctr",
            "cpc",
            "cpm",
            "reach",
            "frequency",
            "actions",
            "cost_per_action_type",
            "date_start",
            "date_stop",
        ].join(",");
        let endpoint = `/${params.adAccountId}/insights`;
        if (params.adId) {
            endpoint = `/${params.adId}/insights`;
        }
        else if (params.level) {
            endpoint = `/${params.adAccountId}/insights`;
        }
        const queryParams = {
            fields,
            date_preset: params.datePreset || "last_7d",
            level: params.level || "ad",
        };
        if (params.timeRange) {
            queryParams.time_range = JSON.stringify(params.timeRange);
        }
        const response = await api.get(endpoint, {
            params: queryParams,
        });
        return InsightsSchema.array().parse(response.data.data);
    }
    async getAdCreative(adId) {
        const api = this.ensureApi();
        const fields = [
            "id",
            "name",
            "object_type",
            "object_url",
            "image_url",
            "image_hash",
            "title",
            "body",
            "call_to_action_type",
            "link_preview_url",
        ].join(",");
        const response = await api.get(`/${adId}/creatives`, {
            params: { fields },
        });
        return response.data.data[0] || {};
    }
    async createLeadForm(adAccountId, data) {
        const api = this.ensureApi();
        const params = {
            name: data.name,
            headline: data.headline,
            description: data.description,
            cta_type: data.ctaType,
            fields: JSON.stringify(data.fields.map((f) => ({ key: f }))),
            privacy_url: data.privacyUrl,
        };
        const response = await api.post(`/${adAccountId}/leadgen_forms`, null, {
            params,
        });
        return response.data;
    }
    async getLeads(formId, params = {}) {
        const api = this.ensureApi();
        const fields = ["id", "created_time", "field_data"].join(",");
        const response = await api.get(`/${formId}/leads`, {
            params: {
                fields,
                limit: params.limit || 100,
            },
        });
        return LeadSchema.array().parse(response.data.data);
    }
    async getCustomAudiences(adAccountId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "id",
            "name",
            "description",
            "subtype",
            "audience_size",
            "delivery_status",
        ].join(",");
        const response = await api.get(`/${adAccountId}/customaudiences`, {
            params: {
                fields,
                limit: params.limit || 50,
            },
        });
        return CustomAudienceSchema.array().parse(response.data.data);
    }
    async createCustomAudience(adAccountId, data) {
        const api = this.ensureApi();
        const params = {
            name: data.name,
            subtype: data.subtype,
        };
        if (data.description)
            params.description = data.description;
        if (data.dataSource)
            params.data_source = JSON.stringify(data.dataSource);
        if (data.audienceSize)
            params.audience_size = data.audienceSize;
        const response = await api.post(`/${adAccountId}/customaudiences`, null, {
            params,
        });
        return CustomAudienceSchema.parse(response.data);
    }
    async getPixelEvents(pixelId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "id",
            "event_time",
            "event_name",
            "action_source",
            "user_data",
            "custom_data",
        ].join(",");
        const queryParams = {
            fields,
            limit: params.limit || 100,
        };
        if (params.startTime)
            queryParams.start_time = params.startTime;
        if (params.endTime)
            queryParams.end_time = params.endTime;
        const response = await api.get(`/${pixelId}/events`, {
            params: queryParams,
        });
        return PixelEventSchema.array().parse(response.data.data);
    }
    async getAdReport(adAccountId, params = {}) {
        const api = this.ensureApi();
        const fields = [
            "campaign_name",
            "campaign_id",
            "adset_name",
            "adset_id",
            "ad_name",
            "ad_id",
            "impressions",
            "clicks",
            "spend",
            "ctr",
            "cpc",
            "cpm",
            "reach",
            "frequency",
            "actions",
            "cost_per_action_type",
            "date_start",
            "date_stop",
        ].join(",");
        const queryParams = {
            fields,
            date_preset: params.datePreset || "last_7d",
            level: "ad",
        };
        if (params.breakdowns && params.breakdowns.length > 0) {
            queryParams.breakdowns = params.breakdowns.join(",");
        }
        if (params.timeRange) {
            queryParams.time_range = JSON.stringify(params.timeRange);
        }
        const response = await api.get(`/${adAccountId}/insights`, {
            params: queryParams,
        });
        return response.data;
    }
    async getPerformanceBenchmark(params = {}) {
        const benchmarks = {
            industry: params.industry || "ecommerce",
            objective: params.objective || "CONVERSIONS",
            vietnam: {
                avg_ctr: 0.89,
                avg_cpc: 4500,
                avg_cpm: 40000,
                avg_roas: 3.5,
                avg_frequency: 2.1,
                avg_conversion_rate: 0.022,
            },
            recommendations: {
                ctr: "CTR bình quân 0.89%, nếu dưới 0.5% cần cải thiện creative",
                cpc: "CPC trung bình 4,500 VND, tối ưu bằng cách thu hẹp targeting",
                cpm: "CPM trung bình 40,000 VND, phù hợp với thị trường Việt Nam",
                roas: "ROAS mục tiêu 3x-5x cho e-commerce",
            },
        };
        return benchmarks;
    }
    async getBudgetOptimization(campaignId, params) {
        const api = this.ensureApi();
        const response = await api.get(`/${campaignId}`, {
            params: {
                fields: ["id", "name", "daily_budget", "lifetime_budget", "spend"].join(","),
            },
        });
        const campaign = response.data;
        const currentSpend = parseFloat(campaign.spend || "0");
        const dailyBudget = params.dailyBudget || parseInt(campaign.daily_budget || "0") / 100;
        return {
            campaign_id: campaignId,
            campaign_name: campaign.name,
            current_daily_budget: dailyBudget,
            current_spend: currentSpend,
            budget_utilization: dailyBudget > 0 ? (currentSpend / dailyBudget) * 100 : 0,
            recommendations: {
                increase: currentSpend >= dailyBudget * 0.95
                    ? "Ngân sách đang sử dụng gần hết, nên tăng budget nếu ROAS tốt"
                    : "Ngân sách sử dụng hợp lý",
                cpa_optimization: params.targetCpa
                    ? `CPA mục tiêu: ${params.targetCpa.toLocaleString("vi-VN")} VND`
                    : "Chưa đặt CPA mục tiêu",
            },
        };
    }
}
//# sourceMappingURL=client.js.map