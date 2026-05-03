package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.*
import vn.bizclaw.app.engine.BizClawLLM
import vn.bizclaw.app.engine.GlobalLLM
import java.util.regex.Pattern

/**
 * LocalAgentLoop — Think-Act-Observe loop powered by on-device LLM.
 *
 * This is the CORE of BizClaw Android running 100% offline:
 *
 * ```
 *  User query
 *    ↓
 *  LLM thinks → generates response with optional tool_call tags
 *    ↓
 *  Parse tool_call → ToolDispatcher executes
 *    ↓
 *  StuckDetector checks: screen frozen? action loop? drift?
 *    ↓ (if stuck → inject recovery hint)
 *  Feed result back to LLM as "tool" message
 *    ↓
 *  Repeat until LLM responds without tool_call (max 8 rounds)
 *    ↓
 *  Return final text to user
 * ```
 *
 * Features (v0.6.0):
 * - StuckDetector: 4-mode stuck detection with recovery hints
 * - VisionFallback: Screenshot → vision LLM when accessibility tree is empty
 * - Pre-parsed commands: /status, /compact, /help, /clear, /model
 *
 * Architecture:
 *   LocalAgentLoop → BizClawLLM (llama.cpp) → ToolDispatcher → AppController/AccessibilityService
 *                  → StuckDetector (monitors stuck conditions)
 *                  → VisionFallback (screenshot when accessibility fails)
 *                  → PreParsedCommands (local shortcuts, no LLM)
 *
 * Everything runs ON THE PHONE. No server. No API keys. $0 cost.
 */
class LocalAgentLoop(
    private val llm: BizClawLLM,
    private val context: Context,
    private val maxRounds: Int = 8,
) {
    private val tag = "LocalAgentLoop"
    private val dispatcher = ToolDispatcher(context)
    private val stuckDetector = StuckDetector()
    private val visionFallback = VisionFallback(context)
    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    // ─── Pre-parsed Commands (v0.6.0) ──────────────────────────────
    private val preParsedCommands = mapOf(
        "/status" to PreParsedCommand("Status", "Show session stats", false),
        "/compact" to PreParsedCommand("Compact", "Compact context window", false),
        "/help" to PreParsedCommand("Help", "Show available commands", false),
        "/clear" to PreParsedCommand("Clear", "Clear conversation history", false),
        "/model" to PreParsedCommand("Model", "Show current model info", false),
        "/tools" to PreParsedCommand("Tools", "List available tools", false),
        "/health" to PreParsedCommand("Health", "Check system health", false),
    )

    data class PreParsedCommand(
        val name: String,
        val description: String,
        val bypassLLM: Boolean = true,
    )

    fun isPreParsedCommand(input: String): Boolean {
        val trimmed = input.trim()
        return trimmed.startsWith("/") && preParsedCommands.containsKey(
            trimmed.split(" ").first()
        )
    }

    fun executePreParsedCommand(input: String): String {
        val trimmed = input.trim()
        val parts = trimmed.split(" ", limit = 2)
        val cmd = parts[0]

        return when (cmd) {
            "/status" -> buildStatusResponse()
            "/compact" -> compactContext()
            "/help" -> buildHelpResponse()
            "/clear" -> clearConversation()
            "/model" -> buildModelResponse()
            "/tools" -> buildToolsResponse()
            "/health" -> "✅ System healthy\n\n• LLM: ${llm.modelName}\n• Memory: ${Runtime.getRuntime().freeMemory() / 1024 / 1024}MB free"
            else -> "Unknown command: $cmd"
        }
    }

    private fun buildStatusResponse(): String {
        return buildString {
            appendLine("📊 Session Status")
            appendLine()
            appendLine("• LLM: ${llm.modelName}")
            appendLine("• Messages: ${llm.getMessageCount()}")
            appendLine("• Tokens: ~${llm.getEstimatedTokens()}")
            appendLine("• Compacting at: 80% context")
            appendLine()
            appendLine("💡 Type /compact to manually trigger compaction")
        }
    }

    private fun compactContext(): String {
        llm.clearConversation()
        return "✅ Context compacted!\n\nMemory cleared. Starting fresh conversation."
    }

    private fun buildHelpResponse(): String {
        return buildString {
            appendLine("🔧 Available Commands")
            appendLine()
            preParsedCommands.forEach { (cmd, info) ->
                appendLine("• $cmd — ${info.description}")
            }
            appendLine()
            appendLine("💡 Commands above run locally (no LLM)")
        }
    }

    private fun clearConversation(): String {
        llm.clearConversation()
        return "🗑️ Conversation cleared."
    }

    private fun buildModelResponse(): String {
        return buildString {
            appendLine("🤖 Model Info")
            appendLine()
            appendLine("• Name: ${llm.modelName}")
            appendLine("• Type: Local GGUF")
            appendLine("• Memory: ${Runtime.getRuntime().maxMemory() / 1024 / 1024}MB max")
            appendLine()
            appendLine("💡 Download new models from Settings → Local LLM")
        }
    }

    private fun buildToolsResponse(): String {
        val tools = dispatcher.toolDefinitions.lines()
            .filter { it.contains("\"name\"") }
            .mapNotNull {
                Regex("\"name\"\\s*:\\s*\"([^\"]+)\"").find(it)?.groupValues?.get(1)
            }
        return buildString {
            appendLine("🔧 Available Tools (${tools.size})")
            appendLine()
            tools.take(20).forEach { tool ->
                appendLine("• $tool")
            }
            if (tools.size > 20) {
                appendLine("• ... and ${tools.size - 20} more")
            }
        }
    }

    // Tool call parsing: <tool_call>...</tool_call>
    // Use java.util.regex.Pattern directly to avoid Android ICU engine angle-bracket issues
    private val toolCallPattern = Pattern.compile(
        "\\x3Ctool_call\\x3E\\s*\\{.*?\\}\\s*\\x3C/tool_call\\x3E",
        Pattern.DOTALL
    )

    // Alternative format: {"name": "...", "arguments": {...}} (without tags)
    // Also use Pattern.compile with hex-escaped braces for Android ICU compatibility
    private val jsonToolCallPattern = Pattern.compile(
        "\\x7B\"name\"\\s*:\\s*\"([^\"]+)\"\\s*,\\s*\"arguments\"\\s*:\\s*(\\x7B.*?\\x7D)\\s*\\x7D",
        Pattern.DOTALL
    )

    /**
     * System prompt that teaches the LLM about available tools.
     * Injected when loading the model.
     */
    val agentSystemPrompt: String = buildString {
        appendLine("You are BizClaw, an AI assistant running directly on an Android phone.")
        appendLine("You can control the phone, post on social media, send messages, and more.")
        appendLine("You respond in the user's language (Vietnamese or English).")
        appendLine()
        appendLine("IMPORTANT RULES:")
        appendLine("1. When you need to perform an action, use a tool call.")
        appendLine("2. After each tool call, wait for the result before proceeding.")
        appendLine("3. Always confirm what you did after completing an action.")
        appendLine("4. If a tool fails, try an alternative approach or inform the user.")
        appendLine("5. Be concise but helpful.")
        appendLine("6. If you receive a STUCK DETECTED warning, follow the recovery suggestions.")
        appendLine("7. If screen_read returns empty, the app may be using WebView/Flutter.")
        appendLine("   In that case, try screen_capture() for vision-based analysis.")
        appendLine()
        append(dispatcher.toolDefinitions)
    }

    /**
     * Run the agent loop for a user query.
     *
     * Returns a Flow that emits:
     * - Partial text tokens (for streaming display)
     * - Special markers: [TOOL_START], [TOOL_END], [ROUND_N]
     *
     * @param query User's message
     * @return Flow of streaming tokens + final response
     */
    fun run(query: String): Flow<AgentToken> = flow {
        var round = 0
        var currentQuery = query
        var pendingToolCalls = true

        // Reset stuck detector for new conversation
        stuckDetector.reset()

        while (pendingToolCalls && round < maxRounds) {
            round++
            Log.i(tag, "🔄 Round $round — processing: ${currentQuery.take(80)}")
            emit(AgentToken.Round(round))

            // === THINK: Get LLM response ===
            val responseBuilder = StringBuilder()
            llm.getResponseAsFlow(currentQuery)
                .flowOn(Dispatchers.IO)
                .collect { token ->
                    responseBuilder.append(token)
                    emit(AgentToken.Text(token))
                }

            val fullResponse = responseBuilder.toString()
            Log.d(tag, "📝 LLM response ($round): ${fullResponse.take(200)}")

            // === ACT: Check for tool calls ===
            val toolCalls = parseToolCalls(fullResponse)

            if (toolCalls.isEmpty()) {
                // No tool calls → LLM is done, this is the final answer
                pendingToolCalls = false
                Log.i(tag, "✅ Round $round — final answer (no tool calls)")
            } else {
                // Execute tool calls
                val toolResults = StringBuilder()
                for (call in toolCalls) {
                    Log.i(tag, "🔧 Executing tool: ${call.name}")
                    emit(AgentToken.ToolStart(call.name))

                    val result = withContext(Dispatchers.Main) {
                        dispatcher.dispatch(call.name, call.arguments)
                    }

                    Log.i(tag, "📋 Tool result: ${result.message.take(100)}")
                    emit(AgentToken.ToolEnd(call.name, result))

                    toolResults.appendLine("Tool '${call.name}' result:")
                    toolResults.appendLine(if (result.success) "✅ ${result.message}" else "❌ ${result.message}")
                    toolResults.appendLine()

                    // ── StuckDetector: Record this action ──
                    val targetText = extractTargetText(call)
                    val screenContent = BizClawAccessibilityService.readScreen()
                    val fingerprint = StuckDetector.fingerprint(screenContent)

                    val stuckHint = stuckDetector.onRoundComplete(
                        screenFingerprint = fingerprint,
                        action = StuckDetector.ActionRecord(
                            toolName = call.name,
                            targetText = targetText,
                            success = result.success,
                        )
                    )

                    // If stuck detected, inject recovery hint
                    if (stuckHint != null) {
                        val hint = stuckHint.recoveryHint()
                        toolResults.appendLine(hint)
                        toolResults.appendLine()
                        emit(AgentToken.Text("\n\n🔴 Stuck detected: ${stuckHint.name}\n"))
                        Log.w(tag, "🔴 Stuck hint injected: ${stuckHint.name}")
                    }

                    // ── VisionFallback: If screen_read returned empty ──
                    if (call.name == "screen_read" && result.success &&
                        screenContent != null && screenContent.elements.isEmpty()
                    ) {
                        val visionProvider = GlobalLLM.getVisionProvider()
                        if (visionProvider != null) {
                            Log.i(tag, "📸 Accessibility empty → Vision fallback")
                            emit(AgentToken.Text("\n📸 Vision mode: analyzing screenshot...\n"))
                            val visionResult = visionFallback.analyzeScreen(visionProvider)
                            if (visionResult.success) {
                                toolResults.appendLine("📸 VISION FALLBACK (accessibility tree was empty):")
                                toolResults.appendLine(visionResult.description)
                                toolResults.appendLine("Use screen_tap(x, y) with coordinates above to interact.")
                                toolResults.appendLine()
                            }
                        } else {
                            toolResults.appendLine("⚠️ Accessibility tree empty. No vision provider available.")
                            toolResults.appendLine("Try screen_tap(x, y) with estimated coordinates.")
                            toolResults.appendLine()
                        }
                    }
                }

                // === OBSERVE: Feed results back to LLM ===
                currentQuery = toolResults.toString().trimEnd()
                // Add tool result as a message (BizClawLLM.addChatMessage handles this)
                llm.addSystemPrompt("") // Clear for next round
            }
        }

        if (round >= maxRounds && pendingToolCalls) {
            emit(AgentToken.Text("\n\n⚠️ Đã đạt giới hạn $maxRounds rounds. Hãy thử lại với yêu cầu đơn giản hơn."))
        }

        emit(AgentToken.Done(round))
    }

    /**
     * Run the agent loop to completion (non-streaming, for automation).
     * Returns the final response text.
     */
    suspend fun runToCompletion(query: String): String {
        val result = StringBuilder()
        run(query).collect { token ->
            when (token) {
                is AgentToken.Text -> result.append(token.content)
                is AgentToken.Done -> {} // Complete
                else -> {} // Skip markers
            }
        }
        return result.toString()
    }

    // ═══════════════════════════════════════════════════════════════
    // Tool Call Parsing
    // ═══════════════════════════════════════════════════════════════

    private fun parseToolCalls(response: String): List<ParsedToolCall> {
        val calls = mutableListOf<ParsedToolCall>()

        // Method 1: <tool_call>{...}</tool_call>
        val matcher = toolCallPattern.matcher(response)
        while (matcher.find()) {
            val matched = matcher.group()
            // Strip tags using plain string replace (no regex needed)
            val jsonStr = matched
                .replace("<tool_call>", "")
                .replace("</tool_call>", "")
                .trim()
            parseJsonToolCall(jsonStr)?.let { calls.add(it) }
        }

        // Method 2: {"name": "...", "arguments": {...}}  (without tags)
        if (calls.isEmpty()) {
            val jsonMatcher = jsonToolCallPattern.matcher(response)
            while (jsonMatcher.find()) {
                val name = jsonMatcher.group(1) ?: continue
                val argsStr = jsonMatcher.group(2) ?: continue
                try {
                    val args = json.parseToJsonElement(argsStr).jsonObject
                    calls.add(ParsedToolCall(name, args))
                } catch (e: Exception) {
                    Log.w(tag, "Failed to parse tool args: $argsStr", e)
                }
            }
        }

        return calls
    }

    private fun parseJsonToolCall(jsonStr: String): ParsedToolCall? {
        return try {
            val obj = json.parseToJsonElement(jsonStr).jsonObject
            val name = obj["name"]?.jsonPrimitive?.content ?: return null
            val args = obj["arguments"]?.jsonObject ?: JsonObject(emptyMap())
            ParsedToolCall(name, args)
        } catch (e: Exception) {
            Log.w(tag, "Failed to parse tool call JSON: $jsonStr", e)
            null
        }
    }

    /**
     * Extract the primary target text from a tool call (for stuck detection).
     */
    private fun extractTargetText(call: ParsedToolCall): String {
        return call.arguments["text"]?.jsonPrimitive?.content
            ?: call.arguments["content"]?.jsonPrimitive?.content
            ?: call.arguments["contact_name"]?.jsonPrimitive?.content
            ?: call.arguments["hint"]?.jsonPrimitive?.content
            ?: call.arguments["package_name"]?.jsonPrimitive?.content
            ?: ""
    }
}

// ═══════════════════════════════════════════════════════════════
// Data Types
// ═══════════════════════════════════════════════════════════════

data class ParsedToolCall(
    val name: String,
    val arguments: JsonObject,
)

/**
 * Tokens emitted by the agent loop for UI consumption.
 */
sealed class AgentToken {
    /** Regular text token from LLM */
    data class Text(val content: String) : AgentToken()

    /** Tool execution started */
    data class ToolStart(val toolName: String) : AgentToken()

    /** Tool execution completed */
    data class ToolEnd(val toolName: String, val result: ToolResult) : AgentToken()

    /** New thinking round started */
    data class Round(val number: Int) : AgentToken()

    /** Agent loop completed */
    data class Done(val totalRounds: Int) : AgentToken()
}

