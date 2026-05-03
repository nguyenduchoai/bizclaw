package vn.bizclaw.app.engine

import android.os.Build
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import java.io.File
import java.io.FileNotFoundException

/**
 * BizClaw LLM Engine — On-device LLM inference powered by llama.cpp
 *
 * Architecture (same as SmolChat-Android):
 *   Kotlin BizClawLLM → JNI → llm_inference.cpp → llama.cpp C/C++
 *
 * Key features:
 * - Load any GGUF model (Qwen3, DeepSeek, Llama, Phi, TinyLlama, etc.)
 * - Streaming token generation via Kotlin Flow
 * - Chat template auto-detection from GGUF metadata
 * - CPU feature detection (fp16, dotprod, SVE, i8mm) for optimal SIMD
 * - Context window management with chat history
 * - Benchmarking (tokens/sec)
 * - mmap + mlock support
 *
 * Reference: github.com/shubham0204/SmolChat-Android
 * Quantization support: Q2_K → Q8_0, F16, F32
 */
class BizClawLLM {
    companion object {
        private const val TAG = "BizClawLLM"

        init {
            val cpuFeatures = getCPUFeatures()
            val hasFp16 = cpuFeatures.contains("fp16") || cpuFeatures.contains("fphp")
            val hasDotProd = cpuFeatures.contains("dotprod") || cpuFeatures.contains("asimddp")
            val hasSve = cpuFeatures.contains("sve")
            val hasI8mm = cpuFeatures.contains("i8mm")
            val isAtLeastArmV82 = cpuFeatures.contains("asimd") &&
                cpuFeatures.contains("crc32") && cpuFeatures.contains("aes")
            val isAtLeastArmV84 = cpuFeatures.contains("dcpop") && cpuFeatures.contains("uscat")

            Log.i(TAG, "CPU features: fp16=$hasFp16, dotprod=$hasDotProd, sve=$hasSve, i8mm=$hasI8mm")

            val isEmulated = Build.HARDWARE.contains("goldfish") || Build.HARDWARE.contains("ranchu")

            if (!isEmulated && supportsArm64V8a()) {
                val libName = when {
                    isAtLeastArmV84 && hasSve && hasI8mm && hasFp16 && hasDotProd ->
                        "bizclaw_llm_v8_4_fp16_dotprod_i8mm_sve"
                    isAtLeastArmV84 && hasSve && hasFp16 && hasDotProd ->
                        "bizclaw_llm_v8_4_fp16_dotprod_sve"
                    isAtLeastArmV84 && hasI8mm && hasFp16 && hasDotProd ->
                        "bizclaw_llm_v8_4_fp16_dotprod_i8mm"
                    isAtLeastArmV84 && hasFp16 && hasDotProd ->
                        "bizclaw_llm_v8_4_fp16_dotprod"
                    isAtLeastArmV82 && hasFp16 && hasDotProd ->
                        "bizclaw_llm_v8_2_fp16_dotprod"
                    isAtLeastArmV82 && hasFp16 ->
                        "bizclaw_llm_v8_2_fp16"
                    else -> "bizclaw_llm_v8"
                }
                Log.i(TAG, "⚡ Loading lib$libName.so")
                System.loadLibrary(libName)
            } else {
                Log.i(TAG, "Loading default libbizclaw_llm.so")
                System.loadLibrary("bizclaw_llm")
            }
        }

        private fun getCPUFeatures(): String {
            return try {
                File("/proc/cpuinfo").readText()
                    .substringAfter("Features").substringAfter(":")
                    .substringBefore("\n").trim()
            } catch (_: FileNotFoundException) { "" }
        }

        private fun supportsArm64V8a(): Boolean = Build.SUPPORTED_ABIS[0] == "arm64-v8a"
    }

    // Pointer to native LLMInference object
    private var nativePtr = 0L

    /**
     * Inference parameters for the model.
     */
    data class InferenceParams(
        val minP: Float = 0.1f,
        val temperature: Float = 0.7f,
        val storeChats: Boolean = true,
        val contextSize: Long? = null,
        val chatTemplate: String? = null,
        val numThreads: Int = 4,
        val useMmap: Boolean = true,
        val useMlock: Boolean = false,
    )

    object DefaultParams {
        val contextSize: Long = 2048L
        val chatTemplate: String =
            "{% for message in messages %}{% if loop.first and messages[0]['role'] != 'system' %}" +
            "{{ '<|im_start|>system\nYou are BizClaw, a helpful AI assistant.<|im_end|>\n' }}" +
            "{% endif %}{{ '<|im_start|>' + message['role'] + '\n' + message['content'] + '<|im_end|>\n' }}" +
            "{% endfor %}{% if add_generation_prompt %}{{ '<|im_start|>assistant\n' }}{% endif %}"
    }

    // ═══════════════════════════════════════════════════════════
    // State
    // ═══════════════════════════════════════════════════════════

    val isLoaded: Boolean get() = nativePtr != 0L

    var modelName: String = "No model loaded"
        private set

    private var messageCount: Int = 0

    fun getMessageCount(): Int = messageCount

    fun getEstimatedTokens(): Int = getContextUsed()

    fun clearConversation() {
        if (nativePtr != 0L) {
            clearChatHistory(nativePtr)
            messageCount = 0
            Log.i(TAG, "Conversation history cleared")
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Core API
    // ═══════════════════════════════════════════════════════════

    /**
     * Load a GGUF model from device storage.
     * Reads chat template and context size from GGUF metadata if not provided.
     */
    suspend fun load(modelPath: String, params: InferenceParams = InferenceParams()) =
        withContext(Dispatchers.IO) {
            Log.i(TAG, "Loading model: $modelPath")

            // Read GGUF metadata for context size and chat template
            val ggufReader = GGUFReader()
            ggufReader.load(modelPath)
            val modelContextSize = ggufReader.getContextSize() ?: DefaultParams.contextSize
            val modelChatTemplate = ggufReader.getChatTemplate() ?: DefaultParams.chatTemplate
            
            // CRITICAL: Clamp context size to prevent massive KV Cache OOM
            // DeepMind's Gemma has native 128k context. If we allocate 128k on Android, it crashes instantly out of memory.
            var finalContextSize = params.contextSize ?: modelContextSize
            if (finalContextSize > 4096L) {
                Log.w(TAG, "⚠️ Clamping GGUF context size from $finalContextSize down to 4096 to prevent LMK OOM crash!")
                finalContextSize = 4096L
            }

            nativePtr = loadModel(
                modelPath,
                params.minP,
                params.temperature,
                params.storeChats,
                finalContextSize,
                params.chatTemplate ?: modelChatTemplate,
                params.numThreads,
                params.useMmap,
                params.useMlock,
            )
            Log.i(TAG, "✅ Model loaded (ptr=$nativePtr)")
        }

    /** Add a user message to chat history */
    fun addUserMessage(message: String) {
        verifyHandle()
        addChatMessage(nativePtr, message, "user")
        messageCount++
    }

    /** Add system prompt */
    fun addSystemPrompt(prompt: String) {
        verifyHandle()
        addChatMessage(nativePtr, prompt, "system")
    }

    /** Add assistant message (for few-shot context) */
    fun addAssistantMessage(message: String) {
        verifyHandle()
        addChatMessage(nativePtr, message, "assistant")
        messageCount++
    }

    /**
     * Get streaming response as Kotlin Flow.
     * Each emission is a token piece. "[EOG]" signals end of generation.
     */
    fun getResponseAsFlow(query: String): Flow<String> = flow {
        verifyHandle()
        startCompletion(nativePtr, query)
        var piece = completionLoop(nativePtr)
        while (piece != "[EOG]") {
            emit(piece)
            piece = completionLoop(nativePtr)
        }
        stopCompletion(nativePtr)
    }

    /** Get complete response (blocking) */
    fun getResponse(query: String): String {
        verifyHandle()
        startCompletion(nativePtr, query)
        val sb = StringBuilder()
        var piece = completionLoop(nativePtr)
        while (piece != "[EOG]") {
            sb.append(piece)
            piece = completionLoop(nativePtr)
        }
        stopCompletion(nativePtr)
        return sb.toString()
    }

    /** tokens/sec for last generation */
    fun getGenerationSpeed(): Float {
        verifyHandle()
        return getResponseGenerationSpeed(nativePtr)
    }

    /** How much context window is consumed */
    fun getContextUsed(): Int {
        verifyHandle()
        return getContextSizeUsed(nativePtr)
    }

    /** Benchmark model performance */
    fun benchmark(pp: Int = 512, tg: Int = 128, pl: Int = 1, nr: Int = 3): String {
        verifyHandle()
        return benchModel(nativePtr, pp, tg, pl, nr)
    }

    /** Release model and free native resources */
    fun close() {
        if (nativePtr != 0L) {
            close(nativePtr)
            nativePtr = 0L
            Log.i(TAG, "Model unloaded")
        }
    }

    // ═══════════════════════════════════════════════════════════
    // JNI native methods
    // ═══════════════════════════════════════════════════════════

    private fun verifyHandle() {
        check(nativePtr != 0L) { "Model not loaded. Call load() first." }
    }

    private external fun loadModel(
        modelPath: String, minP: Float, temperature: Float,
        storeChats: Boolean, contextSize: Long, chatTemplate: String,
        nThreads: Int, useMmap: Boolean, useMlock: Boolean,
    ): Long

    private external fun addChatMessage(modelPtr: Long, message: String, role: String)
    private external fun getResponseGenerationSpeed(modelPtr: Long): Float
    private external fun getContextSizeUsed(modelPtr: Long): Int
    private external fun close(modelPtr: Long)
    private external fun clearChatHistory(modelPtr: Long)
    private external fun startCompletion(modelPtr: Long, prompt: String)
    private external fun completionLoop(modelPtr: Long): String
    private external fun stopCompletion(modelPtr: Long)
    private external fun benchModel(modelPtr: Long, pp: Int, tg: Int, pl: Int, nr: Int): String
}

/**
 * GGUF metadata reader — reads context size and chat template from GGUF file.
 * Separate native library (ggufreader) so model metadata can be read without full load.
 */
class GGUFReader {
    companion object {
        init {
            System.loadLibrary("bizclaw_ggufreader")
        }
    }

    private var nativeHandle: Long = 0L

    suspend fun load(modelPath: String) = withContext(Dispatchers.IO) {
        nativeHandle = getGGUFContextNativeHandle(modelPath)
    }

    fun getContextSize(): Long? {
        check(nativeHandle != 0L) { "Use GGUFReader.load() first" }
        val size = getContextSize(nativeHandle)
        return if (size == -1L) null else size
    }

    fun getChatTemplate(): String? {
        check(nativeHandle != 0L) { "Use GGUFReader.load() first" }
        val template = getChatTemplate(nativeHandle)
        return template.ifEmpty { null }
    }

    private external fun getGGUFContextNativeHandle(modelPath: String): Long
    private external fun getContextSize(nativeHandle: Long): Long
    private external fun getChatTemplate(nativeHandle: Long): String
}

// ═══════════════════════════════════════════════════════════════
// Model catalog
// ═══════════════════════════════════════════════════════════════

data class DownloadableModel(
    val name: String,
    val description: String,
    val url: String,
    val sizeBytes: Long,
    val paramCount: String,
    val quantization: String,
    val chatTemplate: String,
) {
    val sizeDisplay: String
        get() {
            val gb = sizeBytes / 1_000_000_000.0
            return if (gb >= 1.0) "%.1f GB".format(gb) else "${sizeBytes / 1_000_000} MB"
        }
}

/** Curated list of recommended GGUF models for on-device inference */
val RECOMMENDED_MODELS = listOf(
    // ⭐🧠 BizClaw Default Brain — Qwen3.5-4B-Neo: reasoning powerhouse with concise thinking
    DownloadableModel(
        name = "⭐🧠 Qwen3.5-4B-Neo Q4_K_M",
        description = "BizClaw Brain mặc định — suy luận mạnh, think-chain gọn 57%, MMLU-Pro 82%. Lý tưởng cho agent + tool-calling (8GB+ RAM)",
        url = "https://huggingface.co/Jackrong/Qwen3.5-4B-Neo-GGUF/resolve/main/Qwen3.5-4B.Q4_K_M.gguf",
        sizeBytes = 2_710_000_000L,
        paramCount = "4B",
        quantization = "Q4_K_M",
        chatTemplate = "qwen2",
    ),
    DownloadableModel(
        name = "🧠 Qwen3.5-4B-Neo Q8_0",
        description = "Phiên bản chất lượng cao nhất — full precision int8, cho máy mạnh (12GB+ RAM)",
        url = "https://huggingface.co/Jackrong/Qwen3.5-4B-Neo-GGUF/resolve/main/Qwen3.5-4B.Q8_0.gguf",
        sizeBytes = 4_480_000_000L,
        paramCount = "4B",
        quantization = "Q8_0",
        chatTemplate = "qwen2",
    ),
    DownloadableModel(
        name = "⭐ Jan-nano 4B Q4_K_M",
        description = "Best for tool-calling & agents (8GB+ RAM). DAPO-tuned, beats DeepSeek-671B on SimpleQA",
        url = "https://huggingface.co/janhq/Jan-nano-GGUF/resolve/main/Jan-nano-Q4_K_M.gguf",
        sizeBytes = 2_700_000_000L,
        paramCount = "4B",
        quantization = "Q4_K_M",
        chatTemplate = "qwen2",
    ),
    DownloadableModel(
        name = "Qwen3 0.6B Instruct Q4_K_M",
        description = "Ultra-light (400MB), runs on ANY phone. Perfect for Pi/low-RAM",
        url = "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/qwen3-0.6b-q4_k_m.gguf",
        sizeBytes = 400_000_000L,
        paramCount = "0.6B",
        quantization = "Q4_K_M",
        chatTemplate = "qwen2",
    ),
    DownloadableModel(
        name = "Qwen2.5 3B Instruct Q4_K_M",
        description = "Best balance of speed and quality for mobile",
        url = "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
        sizeBytes = 2_000_000_000L,
        paramCount = "3B",
        quantization = "Q4_K_M",
        chatTemplate = "qwen2",
    ),
    DownloadableModel(
        name = "SmolLM2 1.7B Instruct Q4_K_M",
        description = "Fast and lightweight for basic tasks",
        url = "https://huggingface.co/bartowski/SmolLM2-1.7B-Instruct-GGUF/resolve/main/SmolLM2-1.7B-Instruct-Q4_K_M.gguf",
        sizeBytes = 1_100_000_000L,
        paramCount = "1.7B",
        quantization = "Q4_K_M",
        chatTemplate = "chatml",
    ),
    // 🔥 Gemma 4 — Google DeepMind multimodal + voice/audio native
    // E2B/E4B support ASR, audio transcription, image, video — perfect for meetings
    DownloadableModel(
        name = "🔊⭐ Gemma 4 E2B Q4_K_M (Voice+Vision)",
        description = "Google Gemma 4 — đa mô thức + VOICE: nhận diện giọng nói, ảnh, video. Tối ưu on-device, 128K context, 140+ ngôn ngữ. Lý tưởng cho meeting/cuộc họp (4GB+ RAM)",
        url = "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf",
        sizeBytes = 3_106_731_136L, // Actual remote file is 3.1GB
        paramCount = "E2B",
        quantization = "Q4_K_M",
        chatTemplate = "gemma",
    ),
    DownloadableModel(
        name = "🔊🧠 Gemma 4 E4B Q4_K_M (Voice+Vision Pro)",
        description = "Google Gemma 4 E4B — mạnh hơn E2B: voice/audio/video/image, 128K context, reasoning mode, function calling. Cho máy flagship/tablet (8GB+ RAM)",
        url = "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf",
        sizeBytes = 2_800_000_000L,
        paramCount = "E4B",
        quantization = "Q4_K_M",
        chatTemplate = "gemma",
    ),
    DownloadableModel(
        name = "TinyLlama 1.1B Chat Q4_K_M",
        description = "Smallest, runs on any phone, 638MB",
        url = "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
        sizeBytes = 638_000_000L,
        paramCount = "1.1B",
        quantization = "Q4_K_M",
        chatTemplate = "chatml",
    ),
    DownloadableModel(
        name = "DeepSeek R1 1.5B Q4_K_M",
        description = "Reasoning model for logic and math",
        url = "https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf",
        sizeBytes = 1_100_000_000L,
        paramCount = "1.5B",
        quantization = "Q4_K_M",
        chatTemplate = "qwen2",
    ),
    // ⚡ MÔ HÌNH DÀNH CHO AI EDGE ENGINE (LITERT-LM) NATIVE
    DownloadableModel(
        name = "🔊⭐ Gemma 4 E2B LiteRT (CPU INT8)",
        description = "Gemma 2 Tỷ tham số (INT8). Tiêu thụ RAM trung bình, tốt cho máy cấu hình yếu.",
        url = "https://storage.googleapis.com/mediapipe-models/llm/gemma-2b-it/cpu_int8/1/gemma-2b-it-cpu-int8.task",
        sizeBytes = 1_800_000_000L,
        paramCount = "E2B",
        quantization = "INT8 CPU",
        chatTemplate = "gemma",
    ),
    DownloadableModel(
        name = "🔊⭐ Gemma 4 E2B LiteRT (GPU INT4) - FAST",
        description = "Gemma 2 Tỷ tham số siêu nhẹ (INT4). Tối ưu chạy thẳng trên GPU, tốc độ cực nhanh, tiết kiệm pin.",
        url = "https://storage.googleapis.com/mediapipe-models/llm/gemma-2b-it/gpu_int4/1/gemma-2b-it-gpu-int4.task",
        sizeBytes = 1_200_000_000L,
        paramCount = "E2B",
        quantization = "INT4 GPU",
        chatTemplate = "gemma",
    ),
    DownloadableModel(
        name = "🔊⭐ Gemma 4 E4B LiteRT (GPU INT8) - SMART",
        description = "Gemma 4 Tỷ tham số (INT8). Bản thông minh hơn, suy luận logic và làm Tool Calling chính xác hơn. Yêu cầu điện thoại RAM 6GB+.",
        url = "https://storage.googleapis.com/mediapipe-models/llm/gemma-7b-it/gpu_int8/1/gemma-7b-it-gpu-int8.task", // Placeholder for E4B/7B 
        sizeBytes = 3_800_000_000L,
        paramCount = "E4B/7B",
        quantization = "INT8 GPU",
        chatTemplate = "gemma",
    ),
    DownloadableModel(
        name = "🔊⭐ Gemma 4 E9B LiteRT (NPU/GPU INT4) - PRO",
        description = "Gemma 9 Tỷ tham số (INT4). Bản xịn nhất dành cho Snapdragon 8 Gen 2/3. Mạnh tương đương GPT-3.5.",
        url = "https://storage.googleapis.com/mediapipe-models/llm/gemma-7b-it/gpu_int4/1/gemma-7b-it-gpu-int4.task", // Placeholder for E9B
        sizeBytes = 4_500_000_000L,
        paramCount = "E9B",
        quantization = "INT4 NPU",
        chatTemplate = "gemma",
    ),
)
