package vn.bizclaw.app.data.api

import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.sse.EventSource
import okhttp3.sse.EventSourceListener
import okhttp3.sse.EventSources
import vn.bizclaw.app.data.model.*
import java.util.concurrent.TimeUnit

/**
 * BizClaw API Client — connects to bizclaw-gateway.
 *
 * Uses OkHttp for HTTP + SSE streaming.
 * Lightweight: no Retrofit overhead, direct JSON serialization.
 */
class BizClawClient(
    private var serverUrl: String = "http://localhost:3001",
    private var apiKey: String = "",
) {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }

    private val client = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    private val jsonMediaType = "application/json".toMediaType()

    fun updateConfig(url: String, key: String) {
        serverUrl = url.trimEnd('/')
        apiKey = key
    }

    // ─── Auth Header ─────────────────────────────────────────────────

    private fun Request.Builder.addAuth(): Request.Builder {
        if (apiKey.isNotEmpty()) {
            addHeader("Authorization", "Bearer $apiKey")
        }
        return this
    }

    // ─── Chat Completion (non-streaming) ─────────────────────────────

    suspend fun chat(
        messages: List<ChatMessage>,
        model: String = "default",
        agentName: String? = null,
    ): Result<ChatResponse> = runCatching {
        val request = ChatRequest(
            model = agentName ?: model,
            messages = messages,
            stream = false,
        )

        val body = json.encodeToString(ChatRequest.serializer(), request)
            .toRequestBody(jsonMediaType)

        val httpRequest = Request.Builder()
            .url("$serverUrl/v1/chat/completions")
            .post(body)
            .addAuth()
            .build()

        val response = client.newCall(httpRequest).execute()
        val responseBody = response.body?.string() ?: throw Exception("Empty response")

        if (!response.isSuccessful) {
            throw Exception("API error ${response.code}: $responseBody")
        }

        json.decodeFromString(ChatResponse.serializer(), responseBody)
    }

    // ─── Chat Streaming (SSE) ────────────────────────────────────────

    fun chatStream(
        messages: List<ChatMessage>,
        model: String = "default",
        agentName: String? = null,
        onToken: (String) -> Unit,
        onComplete: () -> Unit,
        onError: (Throwable) -> Unit,
    ): EventSource {
        val request = ChatRequest(
            model = agentName ?: model,
            messages = messages,
            stream = true,
        )

        val body = json.encodeToString(ChatRequest.serializer(), request)
            .toRequestBody(jsonMediaType)

        val httpRequest = Request.Builder()
            .url("$serverUrl/v1/chat/completions")
            .post(body)
            .addAuth()
            .build()

        val listener = object : EventSourceListener() {
            override fun onEvent(
                eventSource: EventSource,
                id: String?,
                type: String?,
                data: String,
            ) {
                if (data == "[DONE]") {
                    onComplete()
                    return
                }
                try {
                    val chunk = json.decodeFromString(ChatResponse.serializer(), data)
                    val content = chunk.choices.firstOrNull()?.delta?.content
                    if (!content.isNullOrEmpty()) {
                        onToken(content)
                    }
                } catch (e: Exception) {
                    // Ignore parse errors on partial chunks
                }
            }

            override fun onFailure(
                eventSource: EventSource,
                t: Throwable?,
                response: Response?,
            ) {
                onError(t ?: Exception("SSE connection lost"))
            }
        }

        return EventSources.createFactory(client)
            .newEventSource(httpRequest, listener)
    }

    // ─── List Agents ─────────────────────────────────────────────────

    suspend fun listAgents(): Result<List<AgentInfo>> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/agents")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "[]"
        json.decodeFromString<List<AgentInfo>>(body)
    }

    // ─── List Models ─────────────────────────────────────────────────

    suspend fun listModels(): Result<ModelListResponse> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/v1/models")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "{}"
        json.decodeFromString(ModelListResponse.serializer(), body)
    }

    // ─── Dashboard Stats ─────────────────────────────────────────────

    suspend fun dashboardStats(): Result<DashboardStats> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/stats")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "{}"
        json.decodeFromString(DashboardStats.serializer(), body)
    }

    // ─── LLM Traces ──────────────────────────────────────────────────

    suspend fun listTraces(limit: Int = 20): Result<List<LlmTrace>> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/traces?limit=$limit")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "[]"
        json.decodeFromString<List<LlmTrace>>(body)
    }

    // ─── Health Check ────────────────────────────────────────────────

    suspend fun healthCheck(): Result<Boolean> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/health")
            .get()
            .build()

        val response = client.newCall(request).execute()
        response.isSuccessful
    }

    // ─── Session Stats (v0.6.0) ──────────────────────────────────────

    suspend fun getSessionStats(sessionId: String): Result<SessionStats> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/sessions/$sessionId/stats")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "{}"
        json.decodeFromString<SessionStats>(body)
    }

    suspend fun compactSession(sessionId: String): Result<SessionStats> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/sessions/$sessionId/compact")
            .post("".toRequestBody(jsonMediaType))
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "{}"
        json.decodeFromString<SessionStats>(body)
    }

    // ─── Feedback Collection (v0.6.0) ────────────────────────────────

    suspend fun submitFeedback(feedback: FeedbackEntry): Result<Boolean> = runCatching {
        val body = json.encodeToString(FeedbackEntry.serializer(), feedback)
            .toRequestBody(jsonMediaType)

        val request = Request.Builder()
            .url("$serverUrl/api/v1/feedback")
            .post(body)
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        response.isSuccessful
    }

    suspend fun getAgentMetrics(agentId: String): Result<AgentMetrics> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/agents/$agentId/metrics")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: "{}"
        json.decodeFromString<AgentMetrics>(body)
    }

    suspend fun getOptimizationPrompt(agentId: String): Result<String> = runCatching {
        val request = Request.Builder()
            .url("$serverUrl/api/v1/agents/$agentId/optimize")
            .get()
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        response.body?.string() ?: ""
    }

    // ─── Pairing Codes (v0.6.0) ─────────────────────────────────────

    suspend fun generatePairingCode(userId: String, channel: String = "telegram"): Result<PairingResponse> = runCatching {
        val pairingReq = PairingRequest(userId = userId, channel = channel)
        val body = json.encodeToString(PairingRequest.serializer(), pairingReq)
            .toRequestBody(jsonMediaType)

        val request = Request.Builder()
            .url("$serverUrl/api/v1/pairing/generate")
            .post(body)
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val respBody = response.body?.string() ?: "{}"
        json.decodeFromString<PairingResponse>(respBody)
    }

    suspend fun verifyPairingCode(code: String, userId: String): Result<PairingVerifyResponse> = runCatching {
        val verifyReq = PairingVerifyRequest(code = code, userId = userId)
        val body = json.encodeToString(PairingVerifyRequest.serializer(), verifyReq)
            .toRequestBody(jsonMediaType)

        val request = Request.Builder()
            .url("$serverUrl/api/v1/pairing/verify")
            .post(body)
            .addAuth()
            .build()

        val response = client.newCall(request).execute()
        val respBody = response.body?.string() ?: "{}"
        json.decodeFromString<PairingVerifyResponse>(respBody)
    }
}
