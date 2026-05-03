package vn.bizclaw.app.box

import android.content.Context
import kotlinx.coroutines.*
import java.io.File

/**
 * Box Engine - On-device AI cho Android
 * 
 * Features:
 * - Chat AI offline 100% với llama.cpp
 * - Voice talk + speech-to-text với whisper.cpp
 * - Vision AI qua camera
 * - Tạo ảnh ngay trên máy
 * - Biometric lock + mã hóa lịch sử chat
 * - Hard Offline Mode chặn hoàn toàn internet
 * - Hỗ trợ NPU/GPU trên Snapdragon, Tensor, MediaTek
 */
class BoxEngine(private val context: Context) {
    
    private val chatEngine = BoxChatEngine(context)
    private val voiceEngine = BoxVoiceEngine(context)
    private val visionEngine = BoxVisionEngine(context)
    private val imageEngine = BoxImageEngine(context)
    
    private var isOfflineMode = false
    private var biometricEnabled = false
    
    /**
     * Initialize Box với model path
     */
    suspend fun initialize(config: BoxConfig): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            chatEngine.loadModel(config.modelPath, config.quantization)
            
            config.whisperPath?.let { path ->
                voiceEngine.loadModel(path)
            }
            
            if (config.visionEnabled) {
                config.visionPath?.let { path ->
                    visionEngine.loadModel(path)
                }
            }
            
            if (config.imageEnabled) {
                config.imagePath?.let { path ->
                    imageEngine.loadModel(path)
                }
            }
            
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Chat với AI offline
     */
    suspend fun chat(
        message: String,
        context: String = ""
    ): Result<ChatResponse> = withContext(Dispatchers.Default) {
        chatEngine.generate(message, context)
    }
    
    /**
     * Voice input - speech to text
     */
    suspend fun transcribe(audioData: ByteArray): Result<String> = 
        voiceEngine.transcribe(audioData)
    
    /**
     * Vision - phân tích ảnh/camera
     */
    suspend fun analyzeVision(imagePath: String): Result<VisionResult> =
        visionEngine.analyze(imagePath)
    
    /**
     * Tạo ảnh từ text
     */
    suspend fun generateImage(
        prompt: String,
        negativePrompt: String = "",
        width: Int = 512,
        height: Int = 512,
        steps: Int = 20
    ): Result<String> = imageEngine.generate(prompt, negativePrompt, width, height, steps)
    
    /**
     * Bật Hard Offline Mode
     */
    fun enableHardOfflineMode(enable: Boolean) {
        isOfflineMode = enable
        chatEngine.setOfflineMode(enable)
        voiceEngine.setOfflineMode(enable)
        visionEngine.setOfflineMode(enable)
    }
    
    /**
     * Bật biometric lock
     */
    fun enableBiometric(enable: Boolean) {
        biometricEnabled = enable
    }
    
    /**
     * Release all resources
     */
    fun release() {
        chatEngine.release()
        voiceEngine.release()
        visionEngine.release()
        imageEngine.release()
    }
}

/**
 * Box configuration
 */
data class BoxConfig(
    val modelPath: String,
    val quantization: String = "Q4_K_M",
    val whisperPath: String? = null,
    val visionPath: String? = null,
    val imagePath: String? = null,
    val visionEnabled: Boolean = false,
    val imageEnabled: Boolean = false,
    val contextLength: Int = 4096,
    val threads: Int = 4
)

/**
 * Chat response
 */
data class ChatResponse(
    val content: String,
    val tokens: Int,
    val latency: Long
)

/**
 * Vision result
 */
data class VisionResult(
    val description: String,
    val objects: List<DetectedObject>,
    val text: String?
)

/**
 * Detected object
 */
data class DetectedObject(
    val label: String,
    val confidence: Float,
    val boundingBox: BoundingBox
)

/**
 * Bounding box
 */
data class BoundingBox(
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int
)
