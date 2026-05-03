package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import vn.bizclaw.app.engine.*
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * PlatformSync — Sync Android device ↔ BizClaw Server (apps.viagent.vn)
 *
 * Sync flow:
 * ┌──────────┐         ┌──────────────────┐
 * │ Android  │ ←-sync→ │ BizClaw Gateway  │
 * │  Device  │         │ apps.viagent.vn  │
 * └──────────┘         └──────────────────┘
 *
 * What syncs:
 * - Agent configs (both directions)
 * - Chat history (phone → server for backup)
 * - Knowledge base documents (server → phone)
 * - Task results (phone → server)
 * - Server-pushed tasks (server → phone via WSS)
 *
 * Security:
 * - JWT auth via /api/v1/auth/login
 * - All API calls over HTTPS
 * - API keys stay in EncryptedSharedPreferences
 */
class PlatformSync(private val context: Context) {

    companion object {
        private const val TAG = "PlatformSync"
        private const val PREFS_NAME = "platform_sync"
        private const val KEY_SERVER_URL = "server_url"
        private const val KEY_JWT = "jwt_token"
        private const val KEY_LAST_SYNC = "last_sync_at"
        private const val KEY_ENABLED = "sync_enabled"
        private const val KEY_DEVICE_NAME = "device_name"

        var instance: PlatformSync? = null
            private set

        var onSyncComplete: ((SyncResult) -> Unit)? = null
        var onTaskReceived: ((ServerTask) -> Unit)? = null
    }

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        prettyPrint = true
        encodeDefaults = true
    }

    private val client = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build()

    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var taskWebSocket: WebSocket? = null
    private var autoSyncJob: Job? = null

    // ═══════════════════════════════════════════════════════════════
    // Config
    // ═══════════════════════════════════════════════════════════════

    var serverUrl: String
        get() = prefs.getString(KEY_SERVER_URL, "https://apps.viagent.vn") ?: "https://apps.viagent.vn"
        set(value) = prefs.edit().putString(KEY_SERVER_URL, value.trimEnd('/')).apply()

    var jwt: String
        get() = prefs.getString(KEY_JWT, "") ?: ""
        set(value) = prefs.edit().putString(KEY_JWT, value).apply()

    var isEnabled: Boolean
        get() = prefs.getBoolean(KEY_ENABLED, false)
        set(value) = prefs.edit().putBoolean(KEY_ENABLED, value).apply()

    var lastSyncAt: Long
        get() = prefs.getLong(KEY_LAST_SYNC, 0)
        private set(value) = prefs.edit().putLong(KEY_LAST_SYNC, value).apply()

    val isAuthenticated: Boolean get() = jwt.isNotBlank()

    // ═══════════════════════════════════════════════════════════════
    // Initialize
    // ═══════════════════════════════════════════════════════════════

    fun start() {
        instance = this
        if (isEnabled && isAuthenticated) {
            startAutoSync()
            connectTaskWSS()
        }
        Log.i(TAG, "🔗 PlatformSync started (enabled=$isEnabled, auth=$isAuthenticated)")
    }

    fun stop() {
        autoSyncJob?.cancel()
        taskWebSocket?.close(1000, "PlatformSync stopped")
        taskWebSocket = null
        instance = null
    }

    // ═══════════════════════════════════════════════════════════════
    // Auth
    // ═══════════════════════════════════════════════════════════════

    suspend fun login(username: String, password: String): Result<String> = runCatching {
        val body = json.encodeToString(
            mapOf("username" to username, "password" to password)
        ).toRequestBody("application/json".toMediaType())

        val request = Request.Builder()
            .url("$serverUrl/api/v1/auth/login")
            .post(body)
            .build()

        val response = client.newCall(request).execute()
        val respBody = response.body?.string() ?: throw Exception("Empty response")

        if (!response.isSuccessful) {
            throw Exception("Login failed: ${response.code} — $respBody")
        }

        val respJson = json.parseToJsonElement(respBody).jsonObject
        val token = respJson["token"]?.jsonPrimitive?.content
            ?: throw Exception("No token in response")

        jwt = token
        isEnabled = true
        Log.i(TAG, "✅ Logged in successfully")
        token
    }

    // ═══════════════════════════════════════════════════════════════
    // Sync Agents (both directions)
    // ═══════════════════════════════════════════════════════════════

    /**
     * Pull agents from server → merge with local agents.
     * Push local-only agents → server.
     */
    suspend fun syncAgents(): SyncResult {
        val agentManager = LocalAgentManager(context)
        val localAgents = agentManager.loadAgents()
        var pulled = 0
        var pushed = 0

        // 1. Pull from server
        try {
            val request = Request.Builder()
                .url("$serverUrl/api/v1/agents")
                .get()
                .addHeader("Authorization", "Bearer $jwt")
                .build()

            val response = withContext(Dispatchers.IO) {
                client.newCall(request).execute()
            }

            if (response.isSuccessful) {
                val body = response.body?.string() ?: "[]"
                val serverAgents = json.parseToJsonElement(body).jsonArray

                for (agentJson in serverAgents) {
                    val obj = agentJson.jsonObject
                    val name = obj["name"]?.jsonPrimitive?.content ?: continue
                    val systemPrompt = obj["system_prompt"]?.jsonPrimitive?.content ?: ""

                    // Check if local agent exists
                    val existing = localAgents.find { it.name == name }
                    if (existing == null) {
                        // New agent from server → create locally
                        val agent = LocalAgent(
                            id = "server_${name.lowercase().replace(" ", "_")}",
                            name = name,
                            role = obj["role"]?.jsonPrimitive?.content ?: "",
                            systemPrompt = systemPrompt,
                            emoji = obj["emoji"]?.jsonPrimitive?.content ?: "🤖",
                            providerId = "local_gguf",
                        )
                        agentManager.addAgent(agent)
                        pulled++
                        Log.i(TAG, "📥 Pulled agent: $name")
                    }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to pull agents: ${e.message}")
        }

        // 2. Push local agents to server
        for (agent in localAgents) {
            if (agent.id.startsWith("flow_")) continue // Don't push flow agents
            try {
                val agentPayload = buildString {
                    append("{")
                    append("\"name\":\"${agent.name}\",")
                    append("\"system_prompt\":\"${agent.systemPrompt.replace("\"", "\\\"").replace("\n", "\\n")}\",")
                    append("\"role\":\"${agent.role}\",")
                    append("\"emoji\":\"${agent.emoji}\"")
                    append("}")
                }

                val request = Request.Builder()
                    .url("$serverUrl/api/v1/agents/${agent.name}/config")
                    .put(agentPayload.toRequestBody("application/json".toMediaType()))
                    .addHeader("Authorization", "Bearer $jwt")
                    .build()

                val response = withContext(Dispatchers.IO) {
                    client.newCall(request).execute()
                }
                if (response.isSuccessful) {
                    pushed++
                    Log.i(TAG, "📤 Pushed agent: ${agent.name}")
                }
            } catch (e: Exception) {
                Log.w(TAG, "Failed to push agent ${agent.name}: ${e.message}")
            }
        }

        return SyncResult(
            agentsPulled = pulled,
            agentsPushed = pushed,
        )
    }

    // ═══════════════════════════════════════════════════════════════
    // Sync Knowledge Base (server → phone)
    // ═══════════════════════════════════════════════════════════════

    suspend fun syncKnowledge(): Int {
        var synced = 0
        try {
            val request = Request.Builder()
                .url("$serverUrl/api/v1/knowledge/documents")
                .get()
                .addHeader("Authorization", "Bearer $jwt")
                .build()

            val response = withContext(Dispatchers.IO) {
                client.newCall(request).execute()
            }

            if (response.isSuccessful) {
                val body = response.body?.string() ?: "[]"
                val docs = json.parseToJsonElement(body).jsonArray

                val kbDir = File(context.filesDir, "knowledge_sync")
                kbDir.mkdirs()

                for (doc in docs) {
                    val obj = doc.jsonObject
                    val id = obj["id"]?.jsonPrimitive?.content ?: continue
                    val content = obj["content"]?.jsonPrimitive?.content ?: continue
                    val title = obj["title"]?.jsonPrimitive?.content ?: id

                    val file = File(kbDir, "${id}.md")
                    if (!file.exists() || file.lastModified() < (obj["updated_at"]?.jsonPrimitive?.content?.toLongOrNull() ?: 0)) {
                        file.writeText("# $title\n\n$content")
                        synced++
                        Log.i(TAG, "📥 Synced KB doc: $title")
                    }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "KB sync failed: ${e.message}")
        }
        return synced
    }

    // ═══════════════════════════════════════════════════════════════
    // Push Chat History (phone → server for backup)
    // ═══════════════════════════════════════════════════════════════

    suspend fun pushChatMessage(
        agentName: String,
        userMessage: String,
        aiResponse: String,
        source: String = "android",
    ): Boolean {
        return try {
            val payload = buildString {
                append("{")
                append("\"message\":\"${userMessage.replace("\"", "\\\"").replace("\n", "\\n")}\",")
                append("\"source\":\"$source\"")
                append("}")
            }

            val request = Request.Builder()
                .url("$serverUrl/api/v1/agents/$agentName/chat")
                .post(payload.toRequestBody("application/json".toMediaType()))
                .addHeader("Authorization", "Bearer $jwt")
                .build()

            val response = withContext(Dispatchers.IO) {
                client.newCall(request).execute()
            }
            response.isSuccessful
        } catch (e: Exception) {
            Log.w(TAG, "Failed to push chat: ${e.message}")
            false
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Server-Pushed Tasks (via WSS)
    // ═══════════════════════════════════════════════════════════════

    fun connectTaskWSS() {
        val wsUrl = serverUrl
            .replace("https://", "wss://")
            .replace("http://", "ws://") + "/ws/device"

        val request = Request.Builder()
            .url(wsUrl)
            .addHeader("Authorization", "Bearer $jwt")
            .build()

        taskWebSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.i(TAG, "🟢 Task WSS connected to $wsUrl")
                // Register device
                val reg = DeviceRegistration(
                    deviceId = getDeviceId(),
                    deviceName = "${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}",
                    androidVersion = android.os.Build.VERSION.SDK_INT,
                    appVersion = try {
                        context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "?"
                    } catch (_: Exception) { "?" },
                    modelLoaded = GlobalLLM.loadedModelName,
                    accessibilityEnabled = BizClawAccessibilityService.isRunning(),
                    notificationListenerEnabled = BizClawNotificationListener.instance != null,
                )
                val msg = WsMessage("register", commandJson.encodeToString(reg))
                webSocket.send(commandJson.encodeToString(msg))
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                handleServerMessage(text)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e(TAG, "🔴 Task WSS failed: ${t.message?.take(100)}")
                // Reconnect after 10s
                scope.launch {
                    delay(10_000)
                    if (isEnabled && isAuthenticated) connectTaskWSS()
                }
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "Task WSS closed: $code")
            }
        })
    }

    private fun handleServerMessage(text: String) {
        try {
            val msg = commandJson.decodeFromString<WsMessage>(text)
            when (msg.type) {
                "task" -> {
                    val task = commandJson.decodeFromString<ServerTask>(msg.payload)
                    Log.i(TAG, "📥 Server task: ${task.type} — ${task.description.take(80)}")
                    onTaskReceived?.invoke(task)

                    // Execute task
                    scope.launch {
                        executeServerTask(task)
                    }
                }
                "command" -> {
                    // Forward to existing CommandReceiver
                    val cmd = commandJson.decodeFromString<DeviceCommand>(msg.payload)
                    scope.launch {
                        val result = CommandExecutor.execute(context, cmd)
                        // Send result back via WSS
                        val resultMsg = WsMessage("result", commandJson.encodeToString(result))
                        taskWebSocket?.send(commandJson.encodeToString(resultMsg))
                    }
                }
                "ping" -> {
                    taskWebSocket?.send(commandJson.encodeToString(WsMessage("pong", "")))
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to handle server message: ${e.message}")
        }
    }

    private suspend fun executeServerTask(task: ServerTask) {
        val startTime = System.currentTimeMillis()
        try {
            val result = when (task.type) {
                "post_content" -> {
                    // Server says: "Post this content to Facebook/Zalo/etc."
                    val cmd = DeviceCommand(
                        id = task.id,
                        type = CommandType.automation,
                        action = task.action,
                        params = task.params,
                    )
                    CommandExecutor.execute(context, cmd)
                }
                "reply_customer" -> {
                    val cmd = DeviceCommand(
                        id = task.id,
                        type = CommandType.social_reply,
                        action = "reply",
                        params = task.params,
                    )
                    CommandExecutor.execute(context, cmd)
                }
                "agent_chat" -> {
                    val message = task.params["message"] ?: "Hello"
                    val agentId = task.params["agent_id"]
                    val cmd = DeviceCommand(
                        id = task.id,
                        type = CommandType.chat,
                        action = "chat",
                        params = mapOf(
                            "message" to message,
                            "agent_id" to (agentId ?: ""),
                        ),
                    )
                    CommandExecutor.execute(context, cmd)
                }
                else -> {
                    CommandResult(
                        id = task.id,
                        status = CommandStatus.unsupported,
                        error = "Unknown task type: ${task.type}",
                    )
                }
            }

            // Report result back to server
            reportTaskResult(task.id, result)

        } catch (e: Exception) {
            val errorResult = CommandResult(
                id = task.id,
                status = CommandStatus.failed,
                error = e.message?.take(200),
                durationMs = System.currentTimeMillis() - startTime,
            )
            reportTaskResult(task.id, errorResult)
        }
    }

    private fun reportTaskResult(taskId: String, result: CommandResult) {
        val msg = WsMessage("task_result", commandJson.encodeToString(result))
        taskWebSocket?.send(commandJson.encodeToString(msg))
        Log.i(TAG, "📤 Task result: $taskId → ${result.status}")
    }

    // ═══════════════════════════════════════════════════════════════
    // Auto Sync (periodic)
    // ═══════════════════════════════════════════════════════════════

    fun startAutoSync(intervalMs: Long = 15 * 60 * 1000) {
        autoSyncJob?.cancel()
        autoSyncJob = scope.launch {
            while (isActive) {
                try {
                    val result = fullSync()
                    onSyncComplete?.invoke(result)
                } catch (e: Exception) {
                    Log.e(TAG, "Auto-sync failed: ${e.message}")
                }
                delay(intervalMs)
            }
        }
    }

    suspend fun fullSync(): SyncResult {
        Log.i(TAG, "🔄 Full sync starting...")
        val agentResult = syncAgents()
        val kbDocs = syncKnowledge()
        lastSyncAt = System.currentTimeMillis()

        val result = agentResult.copy(knowledgeSynced = kbDocs)
        Log.i(TAG, "✅ Sync complete: $result")
        return result
    }

    // ═══════════════════════════════════════════════════════════════
    // Helpers
    // ═══════════════════════════════════════════════════════════════

    private fun getDeviceId(): String {
        val prefs = context.getSharedPreferences("bizclaw", Context.MODE_PRIVATE)
        var id = prefs.getString("device_id", null)
        if (id == null) {
            id = "device_${java.util.UUID.randomUUID().toString().take(8)}"
            prefs.edit().putString("device_id", id).apply()
        }
        return id
    }
}

// ═══════════════════════════════════════════════════════════════
// Data Types
// ═══════════════════════════════════════════════════════════════

@Serializable
data class SyncResult(
    val agentsPulled: Int = 0,
    val agentsPushed: Int = 0,
    val knowledgeSynced: Int = 0,
    val chatsPushed: Int = 0,
    val timestamp: Long = System.currentTimeMillis(),
) {
    override fun toString(): String =
        "📥 Pulled: $agentsPulled agents | 📤 Pushed: $agentsPushed agents | 📚 KB: $knowledgeSynced docs"
}

@Serializable
data class ServerTask(
    val id: String,
    val type: String,           // "post_content", "reply_customer", "agent_chat"
    val action: String = "",    // specific action name
    val description: String = "",
    val params: Map<String, String> = emptyMap(),
    val priority: Int = 0,      // 0=normal, 1=high, 2=urgent
    val createdAt: Long = System.currentTimeMillis(),
)
