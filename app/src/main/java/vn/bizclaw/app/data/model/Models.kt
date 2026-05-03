package vn.bizclaw.app.data.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

// ─── OpenAI-Compatible API Types ─────────────────────────────────────

@Serializable
data class ChatRequest(
    val model: String = "default",
    val messages: List<ChatMessage>,
    val temperature: Double = 0.7,
    @SerialName("max_tokens") val maxTokens: Int = 2048,
    val stream: Boolean = false,
)

@Serializable
data class ChatMessage(
    val role: String,
    val content: String,
)

@Serializable
data class ChatResponse(
    val id: String = "",
    val model: String = "",
    val choices: List<Choice> = emptyList(),
    val usage: UsageInfo? = null,
)

@Serializable
data class Choice(
    val index: Int = 0,
    val message: ChatMessage? = null,
    val delta: Delta? = null,
    @SerialName("finish_reason") val finishReason: String? = null,
)

@Serializable
data class Delta(
    val role: String? = null,
    val content: String? = null,
)

@Serializable
data class UsageInfo(
    @SerialName("prompt_tokens") val promptTokens: Int = 0,
    @SerialName("completion_tokens") val completionTokens: Int = 0,
    @SerialName("total_tokens") val totalTokens: Int = 0,
)

// ─── Agent Types ─────────────────────────────────────────────────────

@Serializable
data class AgentInfo(
    val name: String,
    val role: String = "",
    val description: String = "",
    val model: String = "",
    val status: String = "active",
    @SerialName("message_count") val messageCount: Int = 0,
)

// ─── Model Types ─────────────────────────────────────────────────────

@Serializable
data class ModelInfo(
    val id: String,
    @SerialName("object") val objectType: String = "model",
    val owned_by: String = "",
)

@Serializable
data class ModelListResponse(
    val data: List<ModelInfo> = emptyList(),
)

// ─── Trace Types ─────────────────────────────────────────────────────

@Serializable
data class LlmTrace(
    val id: String = "",
    val agent: String = "",
    val model: String = "",
    val provider: String = "",
    @SerialName("prompt_tokens") val promptTokens: Int = 0,
    @SerialName("completion_tokens") val completionTokens: Int = 0,
    @SerialName("cost_usd") val costUsd: Double = 0.0,
    @SerialName("latency_ms") val latencyMs: Long = 0,
    val timestamp: String = "",
)

// ─── Dashboard Types ─────────────────────────────────────────────────

@Serializable
data class DashboardStats(
    @SerialName("agent_count") val agentCount: Int = 0,
    @SerialName("total_requests") val totalRequests: Long = 0,
    @SerialName("total_tokens") val totalTokens: Long = 0,
    @SerialName("total_cost_usd") val totalCostUsd: Double = 0.0,
    val uptime: String = "",
)

// ─── Activity Events ─────────────────────────────────────────────────

@Serializable
data class ActivityEvent(
    val id: String = "",
    @SerialName("event_type") val eventType: String = "",
    val agent: String = "",
    val summary: String = "",
    val timestamp: String = "",
)

// ─── Session Stats (v0.6.0) ────────────────────────────────────────

@Serializable
data class SessionStats(
    @SerialName("session_id") val sessionId: String = "",
    @SerialName("message_count") val messageCount: Int = 0,
    @SerialName("estimated_tokens") val estimatedTokens: Int = 0,
    @SerialName("utilization_pct") val utilizationPct: Float = 0f,
    @SerialName("max_context") val maxContext: Int = 128000,
    @SerialName("last_tool_rounds") val lastToolRounds: Int = 0,
    val compacted: Boolean = false,
    val provider: String = "",
    val model: String = "",
)

// ─── Feedback (v0.6.0) ──────────────────────────────────────────────

@Serializable
data class FeedbackEntry(
    @SerialName("session_id") val sessionId: String = "",
    @SerialName("agent_id") val agentId: String = "",
    val helpful: Boolean = true,
    @SerialName("interaction_type") val interactionType: String = "chat",
    @SerialName("message_preview") val messagePreview: String = "",
    val timestamp: Long = System.currentTimeMillis(),
)

@Serializable
data class AgentMetrics(
    @SerialName("agent_id") val agentId: String = "",
    @SerialName("total_interactions") val totalInteractions: Int = 0,
    @SerialName("helpful_count") val helpfulCount: Int = 0,
    @SerialName("helpful_score") val helpfulScore: Float = 0f,
)

// ─── Pairing Code (v0.6.0) ──────────────────────────────────────────

@Serializable
data class PairingRequest(
    val userId: String,
    val channel: String = "telegram",
)

@Serializable
data class PairingResponse(
    val code: String = "",
    @SerialName("expires_at") val expiresAt: Long = 0,
    @SerialName("channel") val channel: String = "",
)

@Serializable
data class PairingVerifyRequest(
    val code: String,
    val userId: String,
)

@Serializable
data class PairingVerifyResponse(
    val success: Boolean = false,
    @SerialName("paired_user_id") val pairedUserId: String = "",
    val channel: String = "",
)
