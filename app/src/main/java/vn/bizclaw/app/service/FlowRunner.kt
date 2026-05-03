package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.delay
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/**
 * FlowRunner — Chạy automation macro KHÔNG cần LLM. Instant execution.
 *
 * Học từ DroidClaw "flows" — chuỗi hành động cố định, chạy tức thì.
 * Khác với Workflow (cần LLM suy nghĩ), Flow chỉ chạy tuần tự các bước.
 *
 * Use cases:
 * - Cross-post nội dung lên Facebook + Zalo + Instagram cùng lúc
 * - Gửi tin nhắn hàng loạt cho nhiều contact
 * - Morning routine: check weather → post → send report
 * - Macro bán hàng: đăng sản phẩm lên Shopee → share Zalo
 *
 * ```
 * FlowDefinition (JSON/code)
 *   ↓
 * FlowRunner.run()
 *   ↓
 * Step 1: facebook_post("Nội dung") → ✅ 2s
 *   ↓
 * Step 2: zalo_post("Nội dung") → ✅ 3s
 *   ↓
 * Step 3: messenger_send("Boss", "Done!") → ✅ 2s
 *   ↓
 * FlowResult: 3/3 steps completed in 7s
 * ```
 *
 * NO LLM. NO API KEYS. $0 COST. INSTANT.
 */
class FlowRunner(private val context: Context) {

    companion object {
        private const val TAG = "FlowRunner"
        private val json = Json { ignoreUnknownKeys = true; prettyPrint = true; encodeDefaults = true }

        // ─── Built-in Flow Templates ─────────────────────────────────

        /**
         * Cross-post nội dung lên nhiều nền tảng cùng lúc.
         */
        fun crossPostFlow(content: String): FlowDefinition {
            return FlowDefinition(
                name = "Cross-post",
                description = "Đăng bài lên Facebook + Zalo + Instagram",
                steps = listOf(
                    FlowStep(action = FlowAction.FACEBOOK_POST, params = mapOf("content" to content)),
                    FlowStep(action = FlowAction.ZALO_POST, params = mapOf("content" to content)),
                    FlowStep(action = FlowAction.INSTAGRAM_POST, params = mapOf("caption" to content)),
                )
            )
        }

        /**
         * Gửi tin nhắn cho nhiều người cùng lúc.
         */
        fun broadcastMessageFlow(contacts: List<Pair<String, String>>, message: String): FlowDefinition {
            val steps = contacts.map { (platform, name) ->
                when (platform.lowercase()) {
                    "zalo" -> FlowStep(
                        action = FlowAction.ZALO_SEND,
                        params = mapOf("contact_name" to name, "message" to message)
                    )
                    "messenger" -> FlowStep(
                        action = FlowAction.MESSENGER_SEND,
                        params = mapOf("contact_name" to name, "message" to message)
                    )
                    "telegram" -> FlowStep(
                        action = FlowAction.TELEGRAM_SEND,
                        params = mapOf("contact_name" to name, "message" to message)
                    )
                    else -> FlowStep(
                        action = FlowAction.ZALO_SEND,
                        params = mapOf("contact_name" to name, "message" to message)
                    )
                }
            }
            return FlowDefinition(
                name = "Broadcast",
                description = "Gửi tin nhắn cho ${ contacts.size} người",
                steps = steps
            )
        }

        /**
         * Đăng bài bán hàng: post lên Facebook + Zalo + gửi cho danh sách khách
         */
        fun salesPostFlow(content: String, customerNames: List<String>): FlowDefinition {
            val steps = mutableListOf<FlowStep>()
            // Post to social platforms
            steps.add(FlowStep(action = FlowAction.FACEBOOK_POST, params = mapOf("content" to content)))
            steps.add(FlowStep(action = FlowAction.ZALO_POST, params = mapOf("content" to content)))
            // Send to each customer
            for (name in customerNames) {
                steps.add(FlowStep(
                    action = FlowAction.ZALO_SEND,
                    params = mapOf("contact_name" to name, "message" to content),
                    delayAfterMs = 1500, // Shorter delay for messages
                ))
            }
            return FlowDefinition(
                name = "Sales Post",
                description = "Đăng bán hàng + gửi cho ${customerNames.size} khách",
                steps = steps
            )
        }
    }

    private val appController = AppController(context)

    /**
     * Run a flow — execute all steps sequentially. No LLM needed.
     *
     * @param flow The flow definition to execute
     * @param onStepComplete Callback after each step (for UI progress)
     * @return FlowResult with details of each step execution
     */
    suspend fun run(
        flow: FlowDefinition,
        onStepComplete: ((Int, Int, FlowStepResult) -> Unit)? = null,
    ): FlowResult {
        Log.i(TAG, "▶️ Starting flow: ${flow.name} (${flow.steps.size} steps)")
        val startTime = System.currentTimeMillis()
        val stepResults = mutableListOf<FlowStepResult>()

        for ((index, step) in flow.steps.withIndex()) {
            if (!step.enabled) {
                Log.d(TAG, "⏭️ Step ${index + 1} skipped (disabled)")
                stepResults.add(FlowStepResult(
                    step = step,
                    success = true,
                    message = "Skipped (disabled)",
                    durationMs = 0
                ))
                continue
            }

            Log.i(TAG, "🔧 Step ${index + 1}/${flow.steps.size}: ${step.action}")
            val stepStart = System.currentTimeMillis()

            val result = try {
                executeStep(step)
            } catch (e: Exception) {
                Log.e(TAG, "Step ${index + 1} crashed", e)
                FlowStepResult(
                    step = step,
                    success = false,
                    message = "Exception: ${e.message}",
                    durationMs = System.currentTimeMillis() - stepStart
                )
            }

            stepResults.add(result)
            onStepComplete?.invoke(index, flow.steps.size, result)

            // Delay between steps (for UI to render)
            val delayMs = step.delayAfterMs
            if (delayMs > 0 && index < flow.steps.size - 1) {
                delay(delayMs)
            }

            // Stop on failure if configured
            if (!result.success && flow.stopOnFailure) {
                Log.w(TAG, "⛔ Flow stopped due to failure at step ${index + 1}")
                break
            }
        }

        val totalMs = System.currentTimeMillis() - startTime
        val successCount = stepResults.count { it.success }
        Log.i(TAG, "✅ Flow '${flow.name}' complete: $successCount/${flow.steps.size} steps, ${totalMs}ms")

        return FlowResult(
            flowName = flow.name,
            totalSteps = flow.steps.size,
            successCount = successCount,
            failureCount = stepResults.count { !it.success },
            totalDurationMs = totalMs,
            stepResults = stepResults
        )
    }

    /**
     * Execute a single flow step.
     */
    private suspend fun executeStep(step: FlowStep): FlowStepResult {
        val startTime = System.currentTimeMillis()

        val automationResult = when (step.action) {
            // ── Social Media Posts ─────────────────────────
            FlowAction.FACEBOOK_POST -> {
                val content = step.params["content"] ?: return errorResult(step, startTime, "Missing 'content'")
                appController.facebookPost(content)
            }
            FlowAction.FACEBOOK_COMMENT -> {
                val comment = step.params["comment"] ?: return errorResult(step, startTime, "Missing 'comment'")
                appController.facebookComment(comment)
            }
            FlowAction.ZALO_POST -> {
                val content = step.params["content"] ?: return errorResult(step, startTime, "Missing 'content'")
                appController.zaloPost(content)
            }
            FlowAction.INSTAGRAM_POST -> {
                val caption = step.params["caption"] ?: return errorResult(step, startTime, "Missing 'caption'")
                appController.instagramCaption(caption)
            }
            FlowAction.THREADS_POST -> {
                val content = step.params["content"] ?: return errorResult(step, startTime, "Missing 'content'")
                appController.threadsPost(content)
            }

            // ── Messaging ─────────────────────────────────
            FlowAction.ZALO_SEND -> {
                val name = step.params["contact_name"] ?: return errorResult(step, startTime, "Missing 'contact_name'")
                val msg = step.params["message"] ?: return errorResult(step, startTime, "Missing 'message'")
                appController.zaloSendMessage(name, msg)
            }
            FlowAction.MESSENGER_SEND -> {
                val name = step.params["contact_name"] ?: return errorResult(step, startTime, "Missing 'contact_name'")
                val msg = step.params["message"] ?: return errorResult(step, startTime, "Missing 'message'")
                appController.messengerReply(name, msg)
            }
            FlowAction.TELEGRAM_SEND -> {
                val name = step.params["contact_name"] ?: return errorResult(step, startTime, "Missing 'contact_name'")
                val msg = step.params["message"] ?: return errorResult(step, startTime, "Missing 'message'")
                appController.telegramSendMessage(name, msg)
            }
            FlowAction.LARK_SEND -> {
                val name = step.params["contact_name"] ?: return errorResult(step, startTime, "Missing 'contact_name'")
                val msg = step.params["message"] ?: return errorResult(step, startTime, "Missing 'message'")
                appController.larkSendMessage(name, msg)
            }

            // ── Email ─────────────────────────────────────
            FlowAction.GMAIL_COMPOSE -> {
                val to = step.params["to"] ?: return errorResult(step, startTime, "Missing 'to'")
                val subject = step.params["subject"] ?: ""
                val body = step.params["body"] ?: ""
                appController.gmailCompose(to, subject, body)
            }
            FlowAction.LARK_MAIL -> {
                val to = step.params["to"] ?: return errorResult(step, startTime, "Missing 'to'")
                val subject = step.params["subject"] ?: ""
                val body = step.params["body"] ?: ""
                appController.larkComposeMail(to, subject, body)
            }

            // ── Navigation ────────────────────────────────
            FlowAction.OPEN_APP -> {
                val pkg = step.params["package"] ?: return errorResult(step, startTime, "Missing 'package'")
                appController.openApp(pkg)
                AutomationResult.success("Opened app: $pkg")
            }
            FlowAction.OPEN_URL -> {
                val url = step.params["url"] ?: return errorResult(step, startTime, "Missing 'url'")
                appController.openUrl(url)
                AutomationResult.success("Opened URL: $url")
            }

            // ── Screen ────────────────────────────────────
            FlowAction.CLICK -> {
                val text = step.params["text"] ?: return errorResult(step, startTime, "Missing 'text'")
                val result = appController.clickElement(text)
                result
            }
            FlowAction.TYPE -> {
                val text = step.params["text"] ?: return errorResult(step, startTime, "Missing 'text'")
                val typed = BizClawAccessibilityService.typeText(text)
                if (typed) AutomationResult.success("Typed: $text")
                else AutomationResult.error("No focused input field")
            }
            FlowAction.TYPE_INTO -> {
                val hint = step.params["hint"] ?: return errorResult(step, startTime, "Missing 'hint'")
                val text = step.params["text"] ?: return errorResult(step, startTime, "Missing 'text'")
                val typed = BizClawAccessibilityService.typeIntoField(hint, text)
                if (typed) AutomationResult.success("Typed '$text' into '$hint'")
                else AutomationResult.error("Field not found: $hint")
            }
            FlowAction.TAP -> {
                val x = step.params["x"]?.toFloatOrNull() ?: return errorResult(step, startTime, "Missing 'x'")
                val y = step.params["y"]?.toFloatOrNull() ?: return errorResult(step, startTime, "Missing 'y'")
                val ok = BizClawAccessibilityService.tapAt(x, y)
                if (ok) AutomationResult.success("Tapped ($x, $y)")
                else AutomationResult.error("Tap failed")
            }
            FlowAction.SCROLL_DOWN -> {
                val ok = BizClawAccessibilityService.scrollDown()
                if (ok) AutomationResult.success("Scrolled down")
                else AutomationResult.error("No scrollable element")
            }
            FlowAction.PRESS_BACK -> {
                val ok = BizClawAccessibilityService.pressBack()
                if (ok) AutomationResult.success("Back") else AutomationResult.error("Back failed")
            }
            FlowAction.PRESS_HOME -> {
                val ok = BizClawAccessibilityService.pressHome()
                if (ok) AutomationResult.success("Home") else AutomationResult.error("Home failed")
            }
            FlowAction.PRESS_ENTER -> {
                val ok = BizClawAccessibilityService.pressEnter()
                if (ok) AutomationResult.success("Enter") else AutomationResult.error("Enter failed")
            }

            // ── Utility ───────────────────────────────────
            FlowAction.WAIT -> {
                val seconds = step.params["seconds"]?.toLongOrNull() ?: 2
                delay(seconds * 1000)
                AutomationResult.success("Waited ${seconds}s")
            }
            FlowAction.LOG -> {
                val message = step.params["message"] ?: "checkpoint"
                Log.i(TAG, "📝 Flow log: $message")
                AutomationResult.success("Log: $message")
            }
        }

        return FlowStepResult(
            step = step,
            success = automationResult.success,
            message = automationResult.message,
            durationMs = System.currentTimeMillis() - startTime
        )
    }

    private fun errorResult(step: FlowStep, startTime: Long, msg: String): FlowStepResult {
        return FlowStepResult(step = step, success = false, message = msg,
            durationMs = System.currentTimeMillis() - startTime)
    }

    // ═══════════════════════════════════════════════════════════════
    // Flow Persistence (save/load flows as JSON)
    // ═══════════════════════════════════════════════════════════════

    private val flowsDir = context.filesDir.resolve("flows").also { it.mkdirs() }

    fun saveFlow(flow: FlowDefinition) {
        val file = flowsDir.resolve("${flow.id}.json")
        file.writeText(json.encodeToString(FlowDefinition.serializer(), flow))
        Log.i(TAG, "💾 Saved flow: ${flow.name} → ${file.name}")
    }

    fun loadFlow(id: String): FlowDefinition? {
        val file = flowsDir.resolve("$id.json")
        if (!file.exists()) return null
        return try {
            json.decodeFromString(FlowDefinition.serializer(), file.readText())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load flow: $id", e)
            null
        }
    }

    fun listFlows(): List<FlowDefinition> {
        return flowsDir.listFiles()
            ?.filter { it.extension == "json" }
            ?.mapNotNull { file ->
                try { json.decodeFromString(FlowDefinition.serializer(), file.readText()) }
                catch (_: Exception) { null }
            }
            ?.sortedByDescending { it.createdAt }
            ?: emptyList()
    }

    fun deleteFlow(id: String) {
        flowsDir.resolve("$id.json").delete()
    }
}

// ═══════════════════════════════════════════════════════════════
// Data Types
// ═══════════════════════════════════════════════════════════════

/**
 * A complete flow definition — a sequence of steps to execute without LLM.
 */
@Serializable
data class FlowDefinition(
    val id: String = java.util.UUID.randomUUID().toString().take(8),
    val name: String,
    val description: String = "",
    val steps: List<FlowStep>,
    val stopOnFailure: Boolean = false,
    val createdAt: Long = System.currentTimeMillis(),
)

/**
 * A single step in a flow.
 */
@Serializable
data class FlowStep(
    val action: FlowAction,
    val params: Map<String, String> = emptyMap(),
    val delayAfterMs: Long = 2500,  // Default 2.5s between steps for UI to render
    val enabled: Boolean = true,
)

/**
 * All 24 supported flow actions.
 */
@Serializable
enum class FlowAction {
    // Social media posts
    FACEBOOK_POST,
    FACEBOOK_COMMENT,
    ZALO_POST,
    INSTAGRAM_POST,
    THREADS_POST,

    // Messaging
    ZALO_SEND,
    MESSENGER_SEND,
    TELEGRAM_SEND,
    LARK_SEND,

    // Email
    GMAIL_COMPOSE,
    LARK_MAIL,

    // Navigation
    OPEN_APP,
    OPEN_URL,

    // Screen interaction
    CLICK,
    TYPE,
    TYPE_INTO,
    TAP,
    SCROLL_DOWN,
    PRESS_BACK,
    PRESS_HOME,
    PRESS_ENTER,

    // Utility
    WAIT,
    LOG,
}

/**
 * Result of a single step execution.
 */
data class FlowStepResult(
    val step: FlowStep,
    val success: Boolean,
    val message: String,
    val durationMs: Long,
)

/**
 * Result of an entire flow execution.
 */
data class FlowResult(
    val flowName: String,
    val totalSteps: Int,
    val successCount: Int,
    val failureCount: Int,
    val totalDurationMs: Long,
    val stepResults: List<FlowStepResult>,
) {
    val allSuccess: Boolean get() = failureCount == 0

    fun summary(): String = buildString {
        appendLine("📋 Flow: $flowName")
        appendLine("✅ Success: $successCount/$totalSteps")
        if (failureCount > 0) appendLine("❌ Failed: $failureCount")
        appendLine("⏱️ Duration: ${totalDurationMs / 1000}s")
        appendLine()
        for ((i, r) in stepResults.withIndex()) {
            val icon = if (r.success) "✅" else "❌"
            appendLine("  ${i + 1}. $icon ${r.step.action} — ${r.message} (${r.durationMs}ms)")
        }
    }
}
