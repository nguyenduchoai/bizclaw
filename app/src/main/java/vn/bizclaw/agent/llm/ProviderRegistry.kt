package vn.bizclaw.agent.llm

import android.content.Context

/**
 * Resolves the provider the agent should use for a turn.
 *
 * Falls back to on-device whenever the selected cloud provider has no key, so a
 * half-finished setup degrades to a working local agent instead of erroring on every
 * incoming message.
 */
class ProviderRegistry(context: Context, keys: ApiKeyStore) {

    private val onDevice = OnDeviceProvider(context)
    private val claude = AnthropicProvider(keys)
    private val openAi = OpenAiProvider(keys)

    val all: List<LlmProvider> = listOf(onDevice, claude, openAi)

    fun of(kind: ProviderKind): LlmProvider = when (kind) {
        ProviderKind.GEMMA -> onDevice
        ProviderKind.CLAUDE -> claude
        ProviderKind.OPENAI -> openAi
    }

    /** The provider to actually call: [preferred] if usable, else on-device. */
    fun resolve(preferred: ProviderKind): LlmProvider {
        val chosen = of(preferred)
        return if (chosen.isReady()) chosen else onDevice
    }
}
