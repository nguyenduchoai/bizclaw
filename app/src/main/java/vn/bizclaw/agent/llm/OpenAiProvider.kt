package vn.bizclaw.agent.llm

import android.util.Log
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlinx.serialization.json.putJsonObject

private const val TAG = "OpenAiProvider"
private const val ENDPOINT = "https://api.openai.com/v1/chat/completions"

/**
 * OpenAI via Chat Completions.
 *
 * No token cap and no sampling parameters are sent: the newer model families renamed
 * or removed both, and a rejected request costs the whole turn. Server defaults are
 * fine for two-sentence replies.
 */
class OpenAiProvider(private val keys: ApiKeyStore) : LlmProvider {

    override val kind = ProviderKind.OPENAI
    override val supportsTools = true

    override fun isReady() = keys.openAiKey.isNotBlank()

    override suspend fun chat(
        system: String,
        userMessage: String,
        tools: List<ToolSpec>,
        onToolCall: suspend (ToolCall) -> String,
    ): Result<LlmReply> = runCatching {
        val messages = mutableListOf(
            buildJsonObject {
                put("role", "system")
                put("content", system)
            },
            buildJsonObject {
                put("role", "user")
                put("content", userMessage)
            },
        )
        val toolsUsed = mutableListOf<String>()

        repeat(LlmProvider.MAX_TOOL_ROUNDS) {
            val response = send(messages, tools, responseFormat = null)
            val choice = response["choices"]?.jsonArray?.firstOrNull()?.jsonObject
                ?: error("OpenAI trả về rỗng")
            val message = choice["message"]?.jsonObject ?: error("Thiếu message")
            Log.d(TAG, "finish_reason=${choice["finish_reason"]?.jsonPrimitive?.content}")

            val text = message["content"]?.jsonPrimitive?.contentOrNullSafe().orEmpty().trim()
            val rawCalls = message["tool_calls"]?.jsonArray

            if (rawCalls == null || rawCalls.isEmpty()) {
                return@runCatching LlmReply(text, toolsUsed)
            }

            // The assistant turn carrying tool_calls must be replayed before the results.
            messages += message
            rawCalls.map { it.jsonObject }.forEach { raw ->
                val fn = raw["function"]?.jsonObject
                val call = ToolCall(
                    id = raw["id"]?.jsonPrimitive?.content.orEmpty(),
                    name = fn?.get("name")?.jsonPrimitive?.content.orEmpty(),
                    // Arguments arrive as a JSON *string*, not an object.
                    arguments = runCatching {
                        HttpJson.json
                            .parseToJsonElement(fn?.get("arguments")?.jsonPrimitive?.content ?: "{}")
                            .jsonObject
                    }.getOrDefault(JsonObject(emptyMap())),
                )
                toolsUsed += call.name
                messages += buildJsonObject {
                    put("role", "tool")
                    put("tool_call_id", call.id)
                    put("content", onToolCall(call))
                }
            }
        }

        error("Vượt quá ${LlmProvider.MAX_TOOL_ROUNDS} vòng gọi tool")
    }

    override suspend fun json(prompt: String, jsonSchema: String): Result<String> = runCatching {
        val schema = HttpJson.json.parseToJsonElement(jsonSchema).jsonObject
        val format = buildJsonObject {
            put("type", "json_schema")
            putJsonObject("json_schema") {
                put("name", "extraction")
                put("strict", true)
                put("schema", schema)
            }
        }
        val messages = listOf(
            buildJsonObject {
                put("role", "user")
                put("content", prompt)
            },
        )
        val response = send(messages, tools = emptyList(), responseFormat = format)
        response["choices"]?.jsonArray?.firstOrNull()?.jsonObject
            ?.get("message")?.jsonObject
            ?.get("content")?.jsonPrimitive?.contentOrNullSafe()
            ?.trim()
            .orEmpty()
    }

    private suspend fun send(
        messages: List<JsonObject>,
        tools: List<ToolSpec>,
        responseFormat: JsonObject?,
    ): JsonObject {
        val body = buildJsonObject {
            put("model", keys.openAiModel)
            putJsonArray("messages") { messages.forEach { add(it) } }
            if (tools.isNotEmpty()) {
                putJsonArray("tools") {
                    tools.forEach { spec ->
                        add(
                            buildJsonObject {
                                put("type", "function")
                                putJsonObject("function") {
                                    put("name", spec.name)
                                    put("description", spec.description)
                                    put("strict", true)
                                    put(
                                        "parameters",
                                        HttpJson.json
                                            .parseToJsonElement(spec.parameters).jsonObject,
                                    )
                                }
                            },
                        )
                    }
                }
            }
            responseFormat?.let { put("response_format", it) }
        }
        return HttpJson.post(ENDPOINT, headers(), body)
    }

    private fun headers() = okhttp3.Headers.Builder()
        .add("Authorization", "Bearer ${keys.openAiKey}")
        .add("content-type", "application/json")
        .build()
}

/** `content` is JSON null on a pure tool-call turn; treat that as absent, not "null". */
private fun kotlinx.serialization.json.JsonPrimitive.contentOrNullSafe(): String? =
    if (this is kotlinx.serialization.json.JsonNull) null else content
