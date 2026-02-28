package vn.bizclaw.app.ui.chat

import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import vn.bizclaw.app.data.api.BizClawClient
import vn.bizclaw.app.data.model.AgentInfo
import vn.bizclaw.app.data.model.ChatMessage

data class UiMessage(
    val role: String,
    val content: String,
    val isStreaming: Boolean = false,
    val agentName: String = "",
    val tokensUsed: Int = 0,
)

class ChatViewModel : ViewModel() {
    private val client = BizClawClient()

    val messages = mutableStateListOf<UiMessage>()
    val isLoading = mutableStateOf(false)
    val currentAgent = mutableStateOf("default")
    val agents = mutableStateListOf<AgentInfo>()
    val isConnected = mutableStateOf(false)
    val error = mutableStateOf<String?>(null)

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
            }
        }
    }

    fun sendMessage(text: String) {
        if (text.isBlank() || isLoading.value) return

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
            .takeLast(20) // Keep last 20 messages for context
            .map { ChatMessage(role = it.role, content = it.content) }

        // Stream response
        viewModelScope.launch(Dispatchers.IO) {
            val streamContent = StringBuilder()

            client.chatStream(
                messages = apiMessages,
                agentName = currentAgent.value,
                onToken = { token ->
                    streamContent.append(token)
                    // Update the streaming message
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
                    // Fallback to non-streaming
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
                },
            )
        }
    }

    fun clearChat() {
        messages.clear()
    }

    fun selectAgent(name: String) {
        currentAgent.value = name
    }
}
