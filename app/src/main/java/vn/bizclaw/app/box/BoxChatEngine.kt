package vn.bizclaw.app.box

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Box Chat Engine - Chat AI offline với llama.cpp
 */
class BoxChatEngine(private val context: Context) {
    
    private var modelHandle: Long = 0
    private var isLoaded = false
    private var isOfflineMode = false
    
    // Model parameters
    private var nCtx = 4096
    private var nThreads = 4
    private var nGpuLayers = 0
    
    /**
     * Load model từ path
     */
    suspend fun loadModel(modelPath: String, quantization: String = "Q4_K_M"): Result<Unit> = 
        withContext(Dispatchers.IO) {
            try {
                val file = File(modelPath)
                if (!file.exists()) {
                    return@withContext Result.failure(Exception("Model not found: $modelPath"))
                }
                
                // Initialize llama.cpp context
                // modelHandle = llama_init_from_file(modelPath, params)
                
                isLoaded = true
                Result.success(Unit)
            } catch (e: Exception) {
                Result.failure(e)
            }
        }
    
    /**
     * Generate response
     */
    suspend fun generate(
        prompt: String,
        context: String = "",
        maxTokens: Int = 512,
        temperature: Float = 0.7f,
        repeatPenalty: Float = 1.1f
    ): Result<ChatResponse> = withContext(Dispatchers.Default) {
        if (!isLoaded) {
            return@withContext Result.failure(Exception("Model not loaded"))
        }
        
        val startTime = System.currentTimeMillis()
        
        try {
            val fullPrompt = buildPrompt(context, prompt)
            
            // Generate với llama.cpp
            // val result = llama_generate(modelHandle, fullPrompt, maxTokens, temperature, repeatPenalty)
            
            // Simulated response for now
            val response = generateResponse(fullPrompt)
            val tokens = response.length / 4 // Rough estimate
            
            val latency = System.currentTimeMillis() - startTime
            
            Result.success(ChatResponse(
                content = response,
                tokens = tokens,
                latency = latency
            ))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Build prompt với chat template
     */
    private fun buildPrompt(context: String, userMessage: String): String {
        return buildString {
            if (context.isNotEmpty()) {
                appendLine("### Context")
                appendLine(context)
                appendLine()
            }
            appendLine("### Instruction")
            appendLine("Trả lời câu hỏi sau một cách ngắn gọn và hữu ích:")
            appendLine()
            appendLine(userMessage)
            appendLine()
            appendLine("### Response")
        }
    }
    
    /**
     * Simulated response - thay bằng llama.cpp thực tế
     */
    private fun generateResponse(prompt: String): String {
        return when {
            prompt.contains("xin chào", ignoreCase = true) ||
            prompt.contains("hello", ignoreCase = true) -> 
                "Xin chào! Tôi là AI chạy hoàn toàn offline trên máy bạn. Không cần internet!"
            
            prompt.contains("tên", ignoreCase = true) -> 
                "Tôi là Box AI - một trợ lý AI chạy 100% offline trên thiết bị Android."
            
            prompt.contains("có thể", ignoreCase = true) ||
            prompt.contains("làm gì", ignoreCase = true) -> 
                """
                Tôi có thể:
                • Trả lời câu hỏi
                • Phân tích hình ảnh
                • Tạo ảnh mới
                • Nghe và hiểu giọng nói
                • Xử lý tài liệu
                
                Tất cả đều chạy offline, không gửi dữ liệu ra ngoài!
                """.trimIndent()
            
            else -> 
                "Tôi đã nhận được câu hỏi của bạn. Đang xử lý..."
        }
    }
    
    /**
     * Set offline mode
     */
    fun setOfflineMode(offline: Boolean) {
        isOfflineMode = offline
    }
    
    /**
     * Release model
     */
    fun release() {
        if (isLoaded) {
            // llama_free(modelHandle)
            isLoaded = false
        }
    }
}
