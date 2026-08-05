package vn.bizclaw.agent.llm

import kotlinx.serialization.json.JsonObject

/** A function the model may call, described with a JSON Schema. */
data class ToolSpec(
    val name: String,
    val description: String,
    /** JSON Schema for the arguments object. Must set `additionalProperties: false`. */
    val parameters: String,
)

/** A request from the model to run one tool. */
data class ToolCall(
    /** Provider-assigned id; echoed back with the result. */
    val id: String,
    val name: String,
    val arguments: JsonObject,
)

/** Final result of one agent turn. */
data class LlmReply(
    val text: String,
    /** Names of the tools that actually ran, in order. */
    val toolsUsed: List<String> = emptyList(),
)

/** Where a provider runs — which decides what the owner is trusting with customer data. */
enum class ProviderKind(val label: String, val onDevice: Boolean) {
    GEMMA("Gemma 4 (trên máy)", true),
    CLAUDE("Claude", false),
    OPENAI("OpenAI", false),
}

/**
 * A chat model the agent can talk to.
 *
 * The tool loop lives *inside* each implementation rather than in the agent, because
 * the two cloud APIs disagree about how a tool result is threaded back into history
 * and leaking that difference through this interface would put provider branching in
 * the agent. Callers supply [onToolCall] and get back a finished reply.
 */
interface LlmProvider {
    val kind: ProviderKind

    /** True when the provider is configured enough to be called. */
    fun isReady(): Boolean

    /** True when the provider can run [ToolSpec]s. On-device Gemma cannot, reliably. */
    val supportsTools: Boolean

    /**
     * Runs one turn to completion, executing any tools the model asks for.
     *
     * @param onToolCall executes one tool and returns its result as text. Should return
     *   an error string rather than throwing — the model recovers from a stated failure
     *   but not from a dropped turn.
     */
    suspend fun chat(
        system: String,
        userMessage: String,
        tools: List<ToolSpec> = emptyList(),
        onToolCall: suspend (ToolCall) -> String = { "" },
    ): Result<LlmReply>

    /** Runs [prompt] constrained to [jsonSchema]. Returns raw JSON text. */
    suspend fun json(prompt: String, jsonSchema: String): Result<String>

    companion object {
        /** Ceiling on tool round-trips per turn — a runaway loop bills real money. */
        const val MAX_TOOL_ROUNDS = 6
    }
}
