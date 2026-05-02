import { z } from "zod";
declare const CampaignSchema: z.ZodObject<{
    id: z.ZodString;
    name: z.ZodString;
    objective: z.ZodString;
    status: z.ZodString;
    buying_type: z.ZodOptional<z.ZodString>;
    daily_budget: z.ZodOptional<z.ZodString>;
    lifetime_budget: z.ZodOptional<z.ZodString>;
    created_time: z.ZodString;
    updated_time: z.ZodString;
}, "strip", z.ZodTypeAny, {
    id: string;
    name: string;
    objective: string;
    status: string;
    created_time: string;
    updated_time: string;
    buying_type?: string | undefined;
    daily_budget?: string | undefined;
    lifetime_budget?: string | undefined;
}, {
    id: string;
    name: string;
    objective: string;
    status: string;
    created_time: string;
    updated_time: string;
    buying_type?: string | undefined;
    daily_budget?: string | undefined;
    lifetime_budget?: string | undefined;
}>;
declare const AdSetSchema: z.ZodObject<{
    id: z.ZodString;
    name: z.ZodString;
    campaign_id: z.ZodString;
    optimization_goal: z.ZodString;
    billing_event: z.ZodString;
    bid_amount: z.ZodOptional<z.ZodNumber>;
    daily_budget: z.ZodOptional<z.ZodString>;
    lifetime_budget: z.ZodOptional<z.ZodString>;
    status: z.ZodString;
    targeting: z.ZodOptional<z.ZodAny>;
    start_time: z.ZodOptional<z.ZodString>;
    end_time: z.ZodOptional<z.ZodString>;
}, "strip", z.ZodTypeAny, {
    id: string;
    name: string;
    status: string;
    campaign_id: string;
    optimization_goal: string;
    billing_event: string;
    daily_budget?: string | undefined;
    lifetime_budget?: string | undefined;
    bid_amount?: number | undefined;
    targeting?: any;
    start_time?: string | undefined;
    end_time?: string | undefined;
}, {
    id: string;
    name: string;
    status: string;
    campaign_id: string;
    optimization_goal: string;
    billing_event: string;
    daily_budget?: string | undefined;
    lifetime_budget?: string | undefined;
    bid_amount?: number | undefined;
    targeting?: any;
    start_time?: string | undefined;
    end_time?: string | undefined;
}>;
declare const AdSchema: z.ZodObject<{
    id: z.ZodString;
    name: z.ZodString;
    adset_id: z.ZodString;
    campaign_id: z.ZodString;
    status: z.ZodString;
    creative: z.ZodOptional<z.ZodAny>;
    tracking_specs: z.ZodOptional<z.ZodAny>;
    created_time: z.ZodString;
}, "strip", z.ZodTypeAny, {
    id: string;
    name: string;
    status: string;
    created_time: string;
    campaign_id: string;
    adset_id: string;
    creative?: any;
    tracking_specs?: any;
}, {
    id: string;
    name: string;
    status: string;
    created_time: string;
    campaign_id: string;
    adset_id: string;
    creative?: any;
    tracking_specs?: any;
}>;
declare const InsightsSchema: z.ZodObject<{
    spend: z.ZodString;
    impressions: z.ZodString;
    clicks: z.ZodString;
    ctr: z.ZodString;
    cpc: z.ZodString;
    cpm: z.ZodString;
    reach: z.ZodString;
    frequency: z.ZodString;
    actions: z.ZodOptional<z.ZodArray<z.ZodAny, "many">>;
    cost_per_action_type: z.ZodOptional<z.ZodArray<z.ZodAny, "many">>;
    date_start: z.ZodString;
    date_stop: z.ZodString;
}, "strip", z.ZodTypeAny, {
    spend: string;
    impressions: string;
    clicks: string;
    ctr: string;
    cpc: string;
    cpm: string;
    reach: string;
    frequency: string;
    date_start: string;
    date_stop: string;
    actions?: any[] | undefined;
    cost_per_action_type?: any[] | undefined;
}, {
    spend: string;
    impressions: string;
    clicks: string;
    ctr: string;
    cpc: string;
    cpm: string;
    reach: string;
    frequency: string;
    date_start: string;
    date_stop: string;
    actions?: any[] | undefined;
    cost_per_action_type?: any[] | undefined;
}>;
declare const LeadSchema: z.ZodObject<{
    id: z.ZodString;
    created_time: z.ZodString;
    field_data: z.ZodArray<z.ZodAny, "many">;
}, "strip", z.ZodTypeAny, {
    id: string;
    created_time: string;
    field_data: any[];
}, {
    id: string;
    created_time: string;
    field_data: any[];
}>;
declare const CustomAudienceSchema: z.ZodObject<{
    id: z.ZodString;
    name: z.ZodString;
    description: z.ZodOptional<z.ZodString>;
    subtype: z.ZodString;
    audience_size: z.ZodOptional<z.ZodNumber>;
    delivery_status: z.ZodOptional<z.ZodAny>;
}, "strip", z.ZodTypeAny, {
    id: string;
    name: string;
    subtype: string;
    description?: string | undefined;
    audience_size?: number | undefined;
    delivery_status?: any;
}, {
    id: string;
    name: string;
    subtype: string;
    description?: string | undefined;
    audience_size?: number | undefined;
    delivery_status?: any;
}>;
declare const PixelEventSchema: z.ZodObject<{
    id: z.ZodString;
    event_time: z.ZodString;
    event_name: z.ZodString;
    action_source: z.ZodString;
    user_data: z.ZodOptional<z.ZodAny>;
    custom_data: z.ZodOptional<z.ZodAny>;
}, "strip", z.ZodTypeAny, {
    id: string;
    event_time: string;
    event_name: string;
    action_source: string;
    user_data?: any;
    custom_data?: any;
}, {
    id: string;
    event_time: string;
    event_name: string;
    action_source: string;
    user_data?: any;
    custom_data?: any;
}>;
type Campaign = z.infer<typeof CampaignSchema>;
type AdSet = z.infer<typeof AdSetSchema>;
type Ad = z.infer<typeof AdSchema>;
type Insights = z.infer<typeof InsightsSchema>;
type Lead = z.infer<typeof LeadSchema>;
type CustomAudience = z.infer<typeof CustomAudienceSchema>;
type PixelEvent = z.infer<typeof PixelEventSchema>;
interface CampaignParams {
    status?: string;
    objective?: string;
    limit?: number;
}
interface AdSetParams {
    campaignId?: string;
    limit?: number;
}
interface AdParams {
    adsetId?: string;
    limit?: number;
}
interface InsightsParams {
    datePreset?: string;
    timeRange?: {
        since: string;
        until: string;
    };
    level?: string;
}
interface ReportParams {
    datePreset?: string;
    timeRange?: {
        since: string;
        until: string;
    };
    breakdowns?: string[];
}
interface LeadParams {
    limit?: number;
}
interface AudienceParams {
    limit?: number;
}
interface PixelParams {
    startTime?: string;
    endTime?: string;
    limit?: number;
}
interface BenchmarkParams {
    industry?: string;
    objective?: string;
}
export declare class MetaAdsClient {
    private accessToken;
    private api;
    private apiVersion;
    configure(accessToken: string): void;
    isConfigured(): boolean;
    private ensureApi;
    getCampaigns(adAccountId: string, params?: CampaignParams): Promise<Campaign[]>;
    createCampaign(adAccountId: string, data: {
        name: string;
        objective: string;
        status?: string;
        buyingType?: string;
        dailyBudget?: number;
        lifetimeBudget?: number;
    }): Promise<Campaign>;
    getCampaignInsights(campaignId: string, params?: InsightsParams): Promise<Insights[]>;
    getAdSets(adAccountId: string, params?: AdSetParams): Promise<AdSet[]>;
    createAdSet(adAccountId: string, data: {
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
    }): Promise<AdSet>;
    getAds(adAccountId: string, params?: AdParams): Promise<Ad[]>;
    createAd(adAccountId: string, data: {
        adsetId: string;
        creative: object;
        trackingSpecs?: object;
        status?: string;
    }): Promise<Ad>;
    getAdInsights(params: {
        adAccountId: string;
        adId?: string;
        datePreset?: string;
        timeRange?: {
            since: string;
            until: string;
        };
        level?: string;
    }): Promise<Insights[]>;
    getAdCreative(adId: string): Promise<object>;
    createLeadForm(adAccountId: string, data: {
        name: string;
        headline: string;
        description: string;
        ctaType: string;
        fields: string[];
        privacyUrl: string;
    }): Promise<object>;
    getLeads(formId: string, params?: LeadParams): Promise<Lead[]>;
    getCustomAudiences(adAccountId: string, params?: AudienceParams): Promise<CustomAudience[]>;
    createCustomAudience(adAccountId: string, data: {
        name: string;
        description?: string;
        subtype: string;
        dataSource?: object;
        audienceSize?: number;
    }): Promise<CustomAudience>;
    getPixelEvents(pixelId: string, params?: PixelParams): Promise<PixelEvent[]>;
    getAdReport(adAccountId: string, params?: ReportParams): Promise<object>;
    getPerformanceBenchmark(params?: BenchmarkParams): Promise<object>;
    getBudgetOptimization(campaignId: string, params: {
        targetCpa?: number;
        dailyBudget?: number;
    }): Promise<object>;
}
export {};
//# sourceMappingURL=client.d.ts.map