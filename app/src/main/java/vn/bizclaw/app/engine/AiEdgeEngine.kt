package vn.bizclaw.app.engine

import android.content.Context
import android.util.Log
import com.google.ai.edge.litertlm.Backend
import com.google.ai.edge.litertlm.Contents
import com.google.ai.edge.litertlm.Conversation
import com.google.ai.edge.litertlm.ConversationConfig
import com.google.ai.edge.litertlm.Engine
import com.google.ai.edge.litertlm.EngineConfig
import com.google.ai.edge.litertlm.ExperimentalApi
import com.google.ai.edge.litertlm.Message
import com.google.ai.edge.litertlm.MessageCallback
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.suspendCancellableCoroutine
import java.util.concurrent.CancellationException
import kotlin.coroutines.resume

/**
 * AI Edge (LiteRT-LM) Engine cho Gemma 4 và AI Agents.
 * Được ánh xạ từ google-ai-edge/gallery nhằm giải quyết các lỗi crash từ llama.cpp 
 * khi xử lý context window lớn hoặc các tính năng của Gemma 4.
 */
class AiEdgeEngine(private val context: Context) {
    private var engine: Engine? = null
    private var conversation: Conversation? = null
    private val TAG = "AiEdgeEngine"

    val isLoaded: Boolean
        get() = engine != null && conversation != null

    /**
     * Tải mô hình Gemma (.bin/.task) sử dụng LiteRT Engine.
     * Hỗ trợ GPU backend mặc định cho các mô hình lớn.
     */
    @OptIn(ExperimentalApi::class)
    fun loadModel(modelPath: String, useGpu: Boolean = true) {
        try {
            Log.i(TAG, "Loading LiteRT model from: \$modelPath")
            val engineConfig = EngineConfig(
                modelPath = modelPath,
                backend = if (useGpu) Backend.GPU() else Backend.CPU(),
                // Nếu sử dụng Gemma-4 Multimodal, thêm visionBackend
                visionBackend = if (useGpu) Backend.GPU() else null,
                maxNumTokens = 4096 // Phù hợp với Android
            )
            engine = Engine(engineConfig).apply { initialize() }
            
            // Khởi tạo Agent Tools (ToolSet từ AgentTools)
            val tools = listOf(com.google.ai.edge.litertlm.tool(BizClawAgentTools(context)))

            val conversationConfig = ConversationConfig(
                tools = tools
            )
            conversation = engine?.createConversation(conversationConfig)
            Log.i(TAG, "AiEdgeEngine (LiteRT) Loaded Successfully.")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load AiEdgeEngine: \${e.message}", e)
            throw e
        }
    }

    /**
     * Sinh phản hồi text-only sử dụng cấu hình Agent.
     */
    suspend fun getResponse(prompt: String): String = suspendCancellableCoroutine { continuation ->
        val conv = conversation
        if (conv == null) {
            continuation.resume("Lỗi: Mô hình chưa được khởi tạo.")
            return@suspendCancellableCoroutine
        }

        conv.sendMessageAsync(
            Contents.of(prompt),
            object : MessageCallback {
                private val sb = java.lang.StringBuilder()

                override fun onMessage(message: Message) {
                    sb.append(message.toString())
                }

                override fun onDone() {
                    if (continuation.isActive) {
                        continuation.resume(sb.toString())
                    }
                }

                override fun onError(throwable: Throwable) {
                    Log.e(TAG, "Lỗi từ mô hình trong quá trình fetch nội dung", throwable)
                    if (continuation.isActive) {
                        if (throwable is CancellationException) {
                            continuation.resume(sb.toString())
                        } else {
                            continuation.resume("Lỗi nội suy Gemma 4: \${throwable.message}")
                        }
                    }
                }
            }
        )
    }

    fun close() {
        try {
            conversation?.close()
            engine?.close()
            conversation = null
            engine = null
        } catch (e: Exception) {
            Log.e(TAG, "Lỗi khi giải phóng memory engine: \${e.message}")
        }
    }
}
