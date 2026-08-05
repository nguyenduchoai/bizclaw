package vn.bizclaw.agent.llm

import android.content.Context
import vn.bizclaw.agent.BizClawApp

/**
 * Gemma 4 running locally, behind the same interface as the cloud providers.
 *
 * [supportsTools] is false on purpose. A 4B model asked to emit structured tool calls
 * mid-conversation gets it wrong often enough that the agent falls back to the
 * constrained-JSON extraction path instead, which cannot produce malformed output.
 */
class OnDeviceProvider(private val context: Context) : LlmProvider {

    override val kind = ProviderKind.GEMMA
    override val supportsTools = false

    override fun isReady(): Boolean {
        val variant = BizClawApp.from(context).settings.modelVariant
        return variant.isInstalled(context)
    }

    override suspend fun chat(
        system: String,
        userMessage: String,
        tools: List<ToolSpec>,
        onToolCall: suspend (ToolCall) -> String,
    ): Result<LlmReply> {
        val variant = BizClawApp.from(context).settings.modelVariant
        if (!GemmaEngine.ensureLoaded(context, variant)) {
            return Result.failure(IllegalStateException(GemmaEngine.lastError ?: "Chưa nạp model"))
        }
        // No chat roles on the local path — system and user are concatenated into the
        // single prompt the engine takes.
        return GemmaEngine.generate("$system\n\n$userMessage").map { LlmReply(it) }
    }

    override suspend fun json(prompt: String, jsonSchema: String): Result<String> {
        val variant = BizClawApp.from(context).settings.modelVariant
        if (!GemmaEngine.ensureLoaded(context, variant)) {
            return Result.failure(IllegalStateException(GemmaEngine.lastError ?: "Chưa nạp model"))
        }
        return GemmaEngine.generate(prompt, jsonSchema = jsonSchema)
    }
}
