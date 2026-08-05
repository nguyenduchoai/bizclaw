package vn.bizclaw.agent.llm

import android.util.Log
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlinx.serialization.json.putJsonObject

private const val TAG = "AnthropicProvider"
private const val ENDPOINT = "https://api.anthropic.com/v1/messages"
private const val API_VERSION = "2023-06-01"
private const val MAX_TOKENS = 4096

/**
 * Claude via the Messages API.
 *
 * Two constraints drive the request shape. Sampling parameters (`temperature`,
 * `top_p`, `top_k`) are rejected outright on current models, so tone is steered by
 * prompt alone. And thinking is on by default — rather than disabling it (which makes
 * the model occasionally emit a tool call as plain text that then silently never
 * runs), depth is dialled down with a low effort level.
 */
class AnthropicProvider(private val keys: ApiKeyStore) : LlmProvider {

    override val kind = ProviderKind.CLAUDE
    override val supportsTools = true

    override fun isReady() = keys.claudeKey.isNotBlank()

    override suspend fun chat(
        system: String,
        userMessage: String,
        tools: List<ToolSpec>,
        onToolCall: suspend (ToolCall) -> String,
    ): Result<LlmReply> = runCatching {
        val messages = mutableListOf<JsonObject>(
            buildJsonObject {
                put("role", "user")
                put("content", userMessage)
            },
        )
        val toolsUsed = mutableListOf<String>()

        repeat(LlmProvider.MAX_TOOL_ROUNDS) {
            val response = send(system, messages, tools)

            when (val stop = response["stop_reason"]?.jsonPrimitive?.content) {
                "refusal" -> {
                    val category = response["stop_details"]?.jsonObject
                        ?.get("category")?.jsonPrimitive?.content
                    error("Claude từ chối trả lời${category?.let { " ($it)" }.orEmpty()}")
                }

                "max_tokens" -> Log.w(TAG, "Reply hit max_tokens; returning partial text")
                else -> Log.d(TAG, "stop_reason=$stop")
            }

            val content = response["content"]?.jsonArray ?: JsonArray(emptyList())
            val text = content
                .map { it.jsonObject }
                .filter { it["type"]?.jsonPrimitive?.content == "text" }
                .joinToString("") { it["text"]?.jsonPrimitive?.content.orEmpty() }
                .trim()

            val calls = content
                .map { it.jsonObject }
                .filter { it["type"]?.jsonPrimitive?.content == "tool_use" }
                .map { block ->
                    ToolCall(
                        id = block["id"]?.jsonPrimitive?.content.orEmpty(),
                        name = block["name"]?.jsonPrimitive?.content.orEmpty(),
                        arguments = block["input"]?.jsonObject ?: JsonObject(emptyMap()),
                    )
                }

            if (calls.isEmpty()) return@runCatching LlmReply(text, toolsUsed)

            // Run the tools first: the JSON builder below is not a suspend context.
            val results = calls.map { call ->
                toolsUsed += call.name
                call.id to onToolCall(call)
            }

            // Echo the assistant turn back verbatim: thinking blocks must survive
            // unmodified or the next request is rejected.
            messages += buildJsonObject {
                put("role", "assistant")
                put("content", content)
            }
            messages += buildJsonObject {
                put("role", "user")
                putJsonArray("content") {
                    results.forEach { (callId, result) ->
                        add(
                            buildJsonObject {
                                put("type", "tool_result")
                                put("tool_use_id", callId)
                                put("content", result)
                            },
                        )
                    }
                }
            }
        }

        error("Vượt quá ${LlmProvider.MAX_TOOL_ROUNDS} vòng gọi tool")
    }

    override suspend fun json(prompt: String, jsonSchema: String): Result<String> = runCatching {
        val schema = HttpJson.json.parseToJsonElement(jsonSchema).jsonObject
        val body = buildJsonObject {
            put("model", keys.claudeModel)
            put("max_tokens", MAX_TOKENS)
            putJsonObject("output_config") {
                put("effort", "low")
                putJsonObject("format") {
                    put("type", "json_schema")
                    put("schema", schema)
                }
            }
            putJsonArray("messages") {
                add(
                    buildJsonObject {
                        put("role", "user")
                        put("content", prompt)
                    },
                )
            }
        }
        val response = HttpJson.post(ENDPOINT, headers(), body)
        if (response["stop_reason"]?.jsonPrimitive?.content == "refusal") {
            error("Claude từ chối trả lời")
        }
        response["content"]?.jsonArray
            ?.map { it.jsonObject }
            ?.firstOrNull { it["type"]?.jsonPrimitive?.content == "text" }
            ?.get("text")?.jsonPrimitive?.content
            ?.trim()
            .orEmpty()
    }

    private suspend fun send(
        system: String,
        messages: List<JsonObject>,
        tools: List<ToolSpec>,
    ): JsonObject {
        val body = buildJsonObject {
            put("model", keys.claudeModel)
            put("max_tokens", MAX_TOKENS)
            put("system", system)
            // Customer-service replies are short and latency-sensitive; low effort keeps
            // thinking on (the reliable path for tool calls) without paying for depth.
            putJsonObject("output_config") { put("effort", "low") }
            putJsonArray("messages") { messages.forEach { add(it) } }
            if (tools.isNotEmpty()) {
                putJsonArray("tools") {
                    tools.forEach { spec ->
                        add(
                            buildJsonObject {
                                put("name", spec.name)
                                put("description", spec.description)
                                put(
                                    "input_schema",
                                    HttpJson.json.parseToJsonElement(spec.parameters).jsonObject,
                                )
                                // Guarantees the arguments object validates, so the
                                // dispatcher never sees a half-formed order.
                                put("strict", true)
                            },
                        )
                    }
                }
            }
        }
        return HttpJson.post(ENDPOINT, headers(), body)
    }

    private fun headers() = okhttp3.Headers.Builder()
        .add("x-api-key", keys.claudeKey)
        .add("anthropic-version", API_VERSION)
        .add("content-type", "application/json")
        .build()
}
