package vn.bizclaw.app.engine

/**
 * Global singleton for BizClawLLM — survives screen navigation.
 * Model stays loaded when user switches between screens.
 * Only closes when app is fully destroyed.
 */
import android.content.Context
import kotlinx.coroutines.sync.Mutex

object GlobalLLM {
    val instance: BizClawLLM = BizClawLLM()
    var aiEdgeEngine: AiEdgeEngine? = null // AI Edge Gemma 4 Engine
    
    /** Mutex to prevent multi-threaded crashes in C++ ggml backend */
    val generateMutex = Mutex()

    /** Name of the currently loaded model (null = no model loaded) */
    var loadedModelName: String? = null
        private set

    /** App context for loading providers */
    var appContext: Context? = null

    fun setModelName(name: String?) {
        loadedModelName = name
    }

    /**
     * Get the first enabled vision-capable AI provider.
     * Vision-capable = Gemini, OpenAI (GPT-4o), Ollama (llava/llama3.2-vision)
     * 
     * Used by VisionFallback when accessibility tree is empty.
     * Returns null if no vision provider is configured/enabled.
     */
    fun getVisionProvider(): AIProvider? {
        val ctx = appContext ?: return null
        val mgr = ProviderManager(ctx)
        val providers = mgr.loadProviders()

        // Vision-capable provider types (ordered by preference)
        val visionTypes = listOf(
            ProviderType.GEMINI,
            ProviderType.OPENAI,
            ProviderType.CUSTOM_API,
            ProviderType.OLLAMA,
        )

        return providers.firstOrNull { p ->
            p.enabled && p.type in visionTypes && p.apiKey.isNotBlank()
        }
    }
}
