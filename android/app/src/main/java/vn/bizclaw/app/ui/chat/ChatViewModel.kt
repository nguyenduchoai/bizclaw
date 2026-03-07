package vn.bizclaw.app.ui.chat

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.launch
import vn.bizclaw.app.data.api.BizClawClient
import vn.bizclaw.app.data.model.AgentInfo
import vn.bizclaw.app.data.model.ChatMessage
import vn.bizclaw.app.engine.BizClawLLM
import vn.bizclaw.app.engine.ModelDownloadManager
import vn.bizclaw.app.service.AgentToken
import vn.bizclaw.app.service.LocalAgentLoop

data class UiMessage(
    val role: String,
    val content: String,
    val isStreaming: Boolean = false,
    val agentName: String = "",
    val tokensUsed: Int = 0,
    val isLocal: Boolean = false,       // On-device inference
    val tokPerSec: Float = 0f,          // Local LLM speed
    val toolActions: String = "",        // Tool execution log for this message
)

/**
 * ChatViewModel — Unified chat controller for both Cloud and Local LLM.
 *
 * Flow:
 * 1. User selects agent (cloud agent OR "local")
 * 2. If "local" → use BizClawLLM (llama.cpp on-device)
 * 3. If cloud → use BizClawClient (server API)
 * 4. Fallback: if server unreachable AND local model loaded → auto-switch to local
 */
class ChatViewModel : ViewModel() {
    private val client = BizClawClient()

    val messages = mutableStateListOf<UiMessage>()
    val isLoading = mutableStateOf(false)
    val currentAgent = mutableStateOf("default")
    val agents = mutableStateListOf<AgentInfo>()
    val isConnected = mutableStateOf(false)
    val error = mutableStateOf<String?>(null)

    // ═══════════════════════════════════════════════════════════
    // LOCAL LLM STATE
    // ═══════════════════════════════════════════════════════════
    val localLLM = BizClawLLM()
    val isLocalMode = mutableStateOf(false)
    val localModelName = mutableStateOf<String?>(null)
    val localModelLoading = mutableStateOf(false)
    val localGenSpeed = mutableStateOf(0f)
    val localContextUsed = mutableStateOf(0)
    val agentMode = mutableStateOf(true) // true = full agent (tools), false = chat only

    // Agent loop (created when context is available)
    private var agentLoop: LocalAgentLoop? = null

    // List of downloaded GGUF models on device
    val localModels = mutableStateListOf<Pair<String, String>>() // name, path

    fun updateServer(url: String, key: String) {
        client.updateConfig(url, key)
        checkConnection()
    }

    fun checkConnection() {
        viewModelScope.launch(Dispatchers.IO) {
            val result = client.healthCheck()
            isConnected.value = result.getOrDefault(false)
            if (isConnected.value) {
                loadAgents()
            }
        }
    }

    fun loadAgents() {
        viewModelScope.launch(Dispatchers.IO) {
            client.listAgents().onSuccess { list ->
                agents.clear()
                agents.addAll(list)
                // Add "Local LLM" as a virtual agent if any local models exist
                if (localModels.isNotEmpty() || localLLM.isLoaded) {
                    agents.add(0, AgentInfo(
                        name = "local",
                        role = "On-Device AI",
                        description = "Chạy trực tiếp trên điện thoại — 100% offline",
                        model = localModelName.value ?: "No model loaded",
                        status = if (localLLM.isLoaded) "active" else "inactive",
                    ))
                }
            }
        }
    }

    /** Refresh local model list from storage */
    fun refreshLocalModels(context: Context) {
        val manager = ModelDownloadManager(context)
        val models = manager.downloadedModels.value.map { it.name to it.path }
        localModels.clear()
        localModels.addAll(models)

        // Create/update agent loop
        agentLoop = LocalAgentLoop(localLLM, context)

        // If no cloud agents loaded yet, add "local" agent
        if (localModels.isNotEmpty() && agents.none { it.name == "local" }) {
            agents.add(0, AgentInfo(
                name = "local",
                role = "On-Device AI Agent",
                description = "Chạy trực tiếp trên điện thoại — 100% offline, điều khiển apps",
                model = localModelName.value ?: "local-gguf",
                status = if (localLLM.isLoaded) "active" else "inactive",
            ))
        }
    }

    /** Load a local GGUF model for on-device inference */
    fun loadLocalModel(name: String, path: String) {
        viewModelScope.launch(Dispatchers.IO) {
            localModelLoading.value = true
            error.value = null
            try {
                localLLM.close()
                localLLM.load(
                    modelPath = path,
                    params = BizClawLLM.InferenceParams(
                        numThreads = Runtime.getRuntime().availableProcessors().coerceAtMost(8),
                    ),
                )
                // Use agent system prompt with full tool definitions
                val systemPrompt = agentLoop?.agentSystemPrompt
                    ?: ("You are BizClaw, a helpful AI assistant for business operations. " +
                        "Respond concisely and helpfully in the user's language.")
                localLLM.addSystemPrompt(systemPrompt)
                localModelName.value = name
                isLocalMode.value = true
                currentAgent.value = "local"

                // Update the "local" agent entry
                val idx = agents.indexOfFirst { it.name == "local" }
                if (idx >= 0) {
                    agents[idx] = agents[idx].copy(
                        model = name,
                        status = "active",
                    )
                }
            } catch (e: Exception) {
                error.value = "Failed to load local model: ${e.message}"
            }
            localModelLoading.value = false
        }
    }

    /** Unload local model */
    fun unloadLocalModel() {
        localLLM.close()
        localModelName.value = null
        if (isLocalMode.value) {
            isLocalMode.value = false
            currentAgent.value = "default"
        }
    }

    // ═══════════════════════════════════════════════════════════
    // UNIFIED SEND MESSAGE
    // ═══════════════════════════════════════════════════════════

    fun sendMessage(text: String) {
        if (text.isBlank() || isLoading.value) return

        // Determine: use local or cloud?
        val useLocal = isLocalMode.value || currentAgent.value == "local"

        if (useLocal && localLLM.isLoaded) {
            sendLocalMessage(text)
        } else if (useLocal && !localLLM.isLoaded) {
            error.value = "⚠️ Local model not loaded. Tap 🧠 to load a model."
        } else {
            sendCloudMessage(text)
        }
    }

    // ─── Local (On-Device Agent) ────────────────────────────────

    private fun sendLocalMessage(text: String) {
        // Add user message
        messages.add(UiMessage(
            role = "user",
            content = text,
            isLocal = true,
        ))

        // Add streaming placeholder
        val assistantIdx = messages.size
        messages.add(UiMessage(
            role = "assistant",
            content = "",
            isStreaming = true,
            agentName = "BizClaw Agent (Local)",
            isLocal = true,
        ))

        isLoading.value = true
        error.value = null

        val loop = agentLoop
        if (loop != null && agentMode.value) {
            // ═══ AGENT MODE: full Think-Act-Observe loop with tools ═══
            sendLocalAgentMessage(text, assistantIdx, loop)
        } else {
            // ═══ CHAT-ONLY MODE: simple LLM response ═══
            sendLocalChatMessage(text, assistantIdx)
        }
    }

    /** Agent mode: LocalAgentLoop with tool execution */
    private fun sendLocalAgentMessage(text: String, assistantIdx: Int, loop: LocalAgentLoop) {
        viewModelScope.launch {
            try {
                val responseBuilder = StringBuilder()
                val toolLog = StringBuilder()

                loop.run(text).collect { token ->
                    when (token) {
                        is AgentToken.Text -> {
                            responseBuilder.append(token.content)
                            if (assistantIdx < messages.size) {
                                messages[assistantIdx] = messages[assistantIdx].copy(
                                    content = responseBuilder.toString(),
                                    toolActions = toolLog.toString(),
                                )
                            }
                        }
                        is AgentToken.ToolStart -> {
                            toolLog.appendLine("🔧 Executing: ${token.toolName}...")
                            if (assistantIdx < messages.size) {
                                messages[assistantIdx] = messages[assistantIdx].copy(
                                    toolActions = toolLog.toString(),
                                )
                            }
                        }
                        is AgentToken.ToolEnd -> {
                            val icon = if (token.result.success) "✅" else "❌"
                            toolLog.appendLine("$icon ${token.toolName}: ${token.result.message.take(100)}")
                            if (assistantIdx < messages.size) {
                                messages[assistantIdx] = messages[assistantIdx].copy(
                                    toolActions = toolLog.toString(),
                                )
                            }
                        }
                        is AgentToken.Round -> {
                            if (token.number > 1) {
                                responseBuilder.clear()
                                toolLog.appendLine("\n--- Round ${token.number} ---")
                            }
                        }
                        is AgentToken.Done -> {
                            // Final update
                            val speed = localLLM.getGenerationSpeed()
                            val ctxUsed = localLLM.getContextUsed()
                            localGenSpeed.value = speed
                            localContextUsed.value = ctxUsed

                            if (assistantIdx < messages.size) {
                                messages[assistantIdx] = messages[assistantIdx].copy(
                                    isStreaming = false,
                                    tokPerSec = speed,
                                    toolActions = toolLog.toString(),
                                )
                            }
                        }
                    }
                }
            } catch (e: Exception) {
                if (assistantIdx < messages.size) {
                    messages[assistantIdx] = messages[assistantIdx].copy(
                        content = "⚠️ ${e.message}",
                        isStreaming = false,
                    )
                }
                error.value = "Agent error: ${e.message}"
            }
            isLoading.value = false
        }
    }

    /** Chat-only mode: simple LLM response without tools */
    private fun sendLocalChatMessage(text: String, assistantIdx: Int) {
        viewModelScope.launch {
            try {
                val responseBuilder = StringBuilder()
                localLLM.getResponseAsFlow(text)
                    .flowOn(Dispatchers.IO)
                    .collect { token ->
                        responseBuilder.append(token)
                        if (assistantIdx < messages.size) {
                            messages[assistantIdx] = messages[assistantIdx].copy(
                                content = responseBuilder.toString(),
                            )
                        }
                    }

                val speed = localLLM.getGenerationSpeed()
                val ctxUsed = localLLM.getContextUsed()
                localGenSpeed.value = speed
                localContextUsed.value = ctxUsed

                if (assistantIdx < messages.size) {
                    messages[assistantIdx] = messages[assistantIdx].copy(
                        isStreaming = false,
                        tokPerSec = speed,
                    )
                }
            } catch (e: Exception) {
                if (assistantIdx < messages.size) {
                    messages[assistantIdx] = messages[assistantIdx].copy(
                        content = "⚠️ ${e.message}",
                        isStreaming = false,
                    )
                }
                error.value = "Local inference error: ${e.message}"
            }
            isLoading.value = false
        }
    }

    // ─── Cloud (Server API) ───────────────────────────────────

    private fun sendCloudMessage(text: String) {
        // Add user message
        messages.add(UiMessage(role = "user", content = text))

        // Add placeholder for streaming response
        val assistantIdx = messages.size
        messages.add(
            UiMessage(
                role = "assistant",
                content = "",
                isStreaming = true,
                agentName = currentAgent.value,
            )
        )

        isLoading.value = true
        error.value = null

        // Build API messages
        val apiMessages = messages
            .filter { !it.isStreaming }
            .takeLast(20)
            .map { ChatMessage(role = it.role, content = it.content) }

        // Stream response
        viewModelScope.launch(Dispatchers.IO) {
            val streamContent = StringBuilder()

            client.chatStream(
                messages = apiMessages,
                agentName = currentAgent.value,
                onToken = { token ->
                    streamContent.append(token)
                    if (assistantIdx < messages.size) {
                        messages[assistantIdx] = messages[assistantIdx].copy(
                            content = streamContent.toString(),
                        )
                    }
                },
                onComplete = {
                    if (assistantIdx < messages.size) {
                        messages[assistantIdx] = messages[assistantIdx].copy(
                            isStreaming = false,
                        )
                    }
                    isLoading.value = false
                },
                onError = { e ->
                    // ═══════════════════════════════════════════════
                    // AUTO-FALLBACK: If server unreachable and local model loaded,
                    // automatically switch to local inference
                    // ═══════════════════════════════════════════════
                    if (localLLM.isLoaded) {
                        // Remove the cloud placeholder
                        if (assistantIdx < messages.size) {
                            messages.removeAt(assistantIdx)
                        }
                        // Remove the user message we just added (sendLocalMessage will re-add)
                        if (messages.isNotEmpty()) {
                            messages.removeAt(messages.size - 1)
                        }
                        isLoading.value = false
                        isLocalMode.value = true
                        currentAgent.value = "local"
                        error.value = "☁️ Server unreachable → auto-switched to Local LLM"
                        sendLocalMessage(text)
                    } else {
                        // No local model fallback — try non-streaming
                        viewModelScope.launch(Dispatchers.IO) {
                            client.chat(
                                messages = apiMessages,
                                agentName = currentAgent.value,
                            ).onSuccess { response ->
                                val content = response.choices.firstOrNull()?.message?.content ?: ""
                                if (assistantIdx < messages.size) {
                                    messages[assistantIdx] = UiMessage(
                                        role = "assistant",
                                        content = content,
                                        agentName = currentAgent.value,
                                        tokensUsed = response.usage?.totalTokens ?: 0,
                                    )
                                }
                            }.onFailure { err ->
                                error.value = err.message
                                if (assistantIdx < messages.size) {
                                    messages[assistantIdx] = UiMessage(
                                        role = "assistant",
                                        content = "❌ ${err.message}",
                                        agentName = currentAgent.value,
                                    )
                                }
                            }
                            isLoading.value = false
                        }
                    }
                },
            )
        }
    }

    fun clearChat() {
        messages.clear()
    }

    fun selectAgent(name: String) {
        currentAgent.value = name
        isLocalMode.value = (name == "local")
    }

    override fun onCleared() {
        super.onCleared()
        localLLM.close()
    }
}
