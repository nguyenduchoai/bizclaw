package vn.bizclaw.agent.llm

import android.content.Context
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.google.ai.edge.litertlm.Backend
import com.google.ai.edge.litertlm.Content
import com.google.ai.edge.litertlm.ConversationConfig
import com.google.ai.edge.litertlm.Engine
import com.google.ai.edge.litertlm.EngineConfig
import com.google.ai.edge.litertlm.Message
import com.google.ai.edge.litertlm.ResponseFormat
import com.google.ai.edge.litertlm.SamplerConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

private const val TAG = "GemmaEngine"

enum class EngineState { UNLOADED, LOADING, READY, ERROR }

/**
 * Process-wide owner of the loaded Gemma model.
 *
 * Loading costs seconds and gigabytes of RAM, so exactly one [Engine] exists for the
 * whole app and every caller queues behind [lock]. The notification listener and the UI
 * both generate through here.
 */
object GemmaEngine {

    var state by mutableStateOf(EngineState.UNLOADED)
        private set

    var lastError by mutableStateOf<String?>(null)
        private set

    /** Which accelerator actually accepted the model — useful when replies feel slow. */
    var activeBackend by mutableStateOf("")
        private set

    private var engine: Engine? = null
    private val lock = Mutex()

    val isReady: Boolean get() = state == EngineState.READY && engine != null

    /**
     * Loads [variant] if it is not already resident. Returns true when the engine is
     * usable. Blocks for several seconds — never call from the main thread.
     */
    suspend fun ensureLoaded(context: Context, variant: ModelVariant): Boolean = lock.withLock {
        if (isReady) return true

        val model = variant.fileIn(context)
        if (!variant.isInstalled(context)) {
            fail("Chưa có model ${variant.label} trên máy.")
            return false
        }

        state = EngineState.LOADING
        lastError = null

        // GPU first for speed; a lot of mid-range SoCs reject the delegate, and CPU is
        // slower but always available.
        for (candidate in listOf("GPU", "CPU")) {
            val created = withContext(Dispatchers.Default) {
                runCatching {
                    val config = EngineConfig(
                        modelPath = model.absolutePath,
                        backend = if (candidate == "GPU") Backend.GPU() else Backend.CPU(),
                        cacheDir = context.cacheDir.absolutePath,
                    )
                    Engine(config).also { it.initialize() }
                }.onFailure { Log.w(TAG, "Backend $candidate failed to load", it) }.getOrNull()
            }
            if (created != null) {
                engine = created
                activeBackend = candidate
                state = EngineState.READY
                return true
            }
        }

        fail("Không nạp được model. Máy có thể thiếu RAM (cần ~${variant.peakRamMb} MB).")
        return false
    }

    /**
     * Runs [prompt] to completion in a throwaway conversation.
     *
     * Each reply is stateless on purpose: a shared conversation would let one customer's
     * chat leak into the next customer's answer.
     *
     * @param jsonSchema when set, decoding is constrained so the output parses as JSON
     *   matching the schema. Used for order extraction, where a malformed answer would
     *   silently drop a sale.
     */
    suspend fun generate(prompt: String, jsonSchema: String? = null): Result<String> =
        lock.withLock {
            val active =
                engine ?: return Result.failure(IllegalStateException("Engine chưa sẵn sàng"))
            withContext(Dispatchers.Default) {
                runCatching {
                    val config = ConversationConfig(
                        // Low temperature: this is customer service, not creative writing
                        // — the same question should get the same answer every time.
                        samplerConfig = SamplerConfig(topK = 20, topP = 0.9, temperature = 0.3),
                    )
                    active.createConversation(config).use { conversation ->
                        val message = if (jsonSchema == null) {
                            conversation.sendMessage(prompt)
                        } else {
                            conversation.sendMessage(
                                prompt,
                                responseFormat = ResponseFormat.json(jsonSchema),
                            )
                        }
                        message.textOrEmpty()
                    }
                }
            }
        }

    /** Flattens a reply into plain text; non-text parts are irrelevant for chat replies. */
    private fun Message.textOrEmpty(): String =
        contents.contents
            .filterIsInstance<Content.Text>()
            .joinToString(separator = "") { it.text }
            .trim()

    suspend fun unload() = lock.withLock {
        runCatching { engine?.close() }
        engine = null
        activeBackend = ""
        state = EngineState.UNLOADED
    }

    private fun fail(message: String) {
        lastError = message
        state = EngineState.ERROR
        Log.e(TAG, message)
    }
}
