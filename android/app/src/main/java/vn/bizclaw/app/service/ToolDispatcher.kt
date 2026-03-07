package vn.bizclaw.app.service

import android.content.Context
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * ToolDispatcher — maps LLM tool_call names to Kotlin handler functions.
 *
 * This is the bridge between:
 *   BizClawLLM (on-device inference) → ToolDispatcher → AppController / AccessibilityService / DeviceCapabilities
 *
 * All tools are LOCAL — no server needed. Everything runs on the phone.
 *
 * Tool format from LLM (Qwen3 / Llama3 function calling):
 *   {"name": "facebook_post", "arguments": {"content": "Hello world!"}}
 */
class ToolDispatcher(private val context: Context) {

    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    // ═══════════════════════════════════════════════════════════════
    // Tool definitions for system prompt
    // ═══════════════════════════════════════════════════════════════

    val toolDefinitions: String = """
You have access to these tools to control the Android device:

### Social Media Tools
- `facebook_post(content: string)` — Post content to Facebook feed
- `facebook_comment(comment: string)` — Comment on the current visible post
- `messenger_reply(contact_name: string, message: string)` — Reply in Messenger
- `messenger_read()` — Read last messages in current Messenger chat
- `zalo_send(contact_name: string, message: string)` — Send Zalo message

### Screen Control Tools
- `screen_read()` — Read all text/buttons visible on current screen
- `screen_click(text: string)` — Click element containing this text
- `screen_type(text: string)` — Type text into focused input field
- `screen_type_into(hint: string, text: string)` — Type into field with matching hint
- `screen_scroll_down()` — Scroll down
- `screen_scroll_up()` — Scroll up
- `screen_tap(x: number, y: number)` — Tap at screen coordinates
- `screen_swipe(start_x: number, start_y: number, end_x: number, end_y: number)` — Swipe gesture
- `press_back()` — Press Back button
- `press_home()` — Press Home button
- `press_enter()` — Press Enter/Send

### App & System Tools
- `open_app(package_name: string)` — Open an app (e.g. com.facebook.katana)
- `open_url(url: string)` — Open URL in browser
- `device_info()` — Get device info (battery, storage, RAM, CPU)
- `notifications()` — Open notification shade

To use a tool, respond with EXACTLY this format:
<tool_call>
{"name": "tool_name", "arguments": {"param1": "value1", "param2": "value2"}}
</tool_call>

Wait for the tool result before continuing. You can call multiple tools in sequence.
When you're done and have the final answer, respond normally without tool_call tags.
""".trimIndent()

    // ═══════════════════════════════════════════════════════════════
    // Dispatch: tool name + args → execute → result string
    // ═══════════════════════════════════════════════════════════════

    suspend fun dispatch(toolName: String, args: JsonObject): ToolResult {
        return try {
            val result = when (toolName) {
                // Social media
                "facebook_post" -> facebookPost(args)
                "facebook_comment" -> facebookComment(args)
                "messenger_reply" -> messengerReply(args)
                "messenger_read" -> messengerRead()
                "zalo_send" -> zaloSend(args)

                // Screen control
                "screen_read" -> screenRead()
                "screen_click" -> screenClick(args)
                "screen_type" -> screenType(args)
                "screen_type_into" -> screenTypeInto(args)
                "screen_scroll_down" -> screenScrollDown()
                "screen_scroll_up" -> screenScrollUp()
                "screen_tap" -> screenTap(args)
                "screen_swipe" -> screenSwipe(args)
                "press_back" -> pressBack()
                "press_home" -> pressHome()
                "press_enter" -> pressEnter()

                // App & system
                "open_app" -> openApp(args)
                "open_url" -> openUrl(args)
                "device_info" -> deviceInfo()
                "notifications" -> openNotifications()

                else -> ToolResult(false, "Unknown tool: $toolName")
            }
            result
        } catch (e: Exception) {
            ToolResult(false, "Tool error: ${e.message}")
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Tool Implementations
    // ═══════════════════════════════════════════════════════════════

    // ── Social Media ─────────────────────────────────────────────

    private suspend fun facebookPost(args: JsonObject): ToolResult {
        val content = args["content"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'content'")
        val ctrl = AppController(context)
        val result = ctrl.facebookPost(content)
        return ToolResult(result.success, result.message)
    }

    private suspend fun facebookComment(args: JsonObject): ToolResult {
        val comment = args["comment"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'comment'")
        val ctrl = AppController(context)
        val result = ctrl.facebookComment(comment)
        return ToolResult(result.success, result.message)
    }

    private suspend fun messengerReply(args: JsonObject): ToolResult {
        val name = args["contact_name"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'contact_name'")
        val msg = args["message"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'message'")
        val ctrl = AppController(context)
        val result = ctrl.messengerReply(name, msg)
        return ToolResult(result.success, result.message)
    }

    private fun messengerRead(): ToolResult {
        val ctrl = AppController(context)
        val result = ctrl.messengerReadMessages()
        return ToolResult(result.success, result.message)
    }

    private suspend fun zaloSend(args: JsonObject): ToolResult {
        val name = args["contact_name"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'contact_name'")
        val msg = args["message"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'message'")
        val ctrl = AppController(context)
        val result = ctrl.zaloSendMessage(name, msg)
        return ToolResult(result.success, result.message)
    }

    // ── Screen Control ───────────────────────────────────────────

    private fun screenRead(): ToolResult {
        val content = BizClawAccessibilityService.readScreen()
            ?: return ToolResult(false, "Accessibility Service not connected. Enable in Settings → Accessibility → BizClaw Agent")
        val summary = buildString {
            appendLine("App: ${content.packageName}")
            appendLine("Elements: ${content.elementCount}")
            appendLine("---")
            for (el in content.elements) {
                val tags = mutableListOf<String>()
                if (el.isClickable) tags.add("clickable")
                if (el.isEditable) tags.add("editable")
                if (el.isScrollable) tags.add("scrollable")
                val tagStr = if (tags.isNotEmpty()) " [${tags.joinToString(",")}]" else ""
                if (el.text.isNotEmpty()) {
                    appendLine("• ${el.className}: \"${el.text}\"$tagStr")
                } else if (el.contentDescription.isNotEmpty()) {
                    appendLine("• ${el.className}: (${el.contentDescription})$tagStr")
                } else if (el.hint.isNotEmpty()) {
                    appendLine("• ${el.className}: hint=\"${el.hint}\"$tagStr")
                }
            }
        }
        return ToolResult(true, summary)
    }

    private fun screenClick(args: JsonObject): ToolResult {
        val text = args["text"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'text'")
        val clicked = BizClawAccessibilityService.clickByText(text)
        return if (clicked) ToolResult(true, "Clicked element with text: \"$text\"")
        else ToolResult(false, "Could not find clickable element with text: \"$text\"")
    }

    private fun screenType(args: JsonObject): ToolResult {
        val text = args["text"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'text'")
        val typed = BizClawAccessibilityService.typeText(text)
        return if (typed) ToolResult(true, "Typed: \"$text\"")
        else ToolResult(false, "No focused input field to type into")
    }

    private fun screenTypeInto(args: JsonObject): ToolResult {
        val hint = args["hint"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'hint'")
        val text = args["text"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'text'")
        val typed = BizClawAccessibilityService.typeIntoField(hint, text)
        return if (typed) ToolResult(true, "Typed \"$text\" into field with hint \"$hint\"")
        else ToolResult(false, "Could not find input field with hint: \"$hint\"")
    }

    private fun screenScrollDown(): ToolResult {
        val ok = BizClawAccessibilityService.scrollDown()
        return ToolResult(ok, if (ok) "Scrolled down" else "No scrollable element found")
    }

    private fun screenScrollUp(): ToolResult {
        val ok = BizClawAccessibilityService.scrollUp()
        return ToolResult(ok, if (ok) "Scrolled up" else "No scrollable element found")
    }

    private fun screenTap(args: JsonObject): ToolResult {
        val x = args["x"]?.jsonPrimitive?.content?.toFloatOrNull() ?: return ToolResult(false, "Missing 'x'")
        val y = args["y"]?.jsonPrimitive?.content?.toFloatOrNull() ?: return ToolResult(false, "Missing 'y'")
        val ok = BizClawAccessibilityService.tapAt(x, y)
        return ToolResult(ok, if (ok) "Tapped at ($x, $y)" else "Tap failed")
    }

    private fun screenSwipe(args: JsonObject): ToolResult {
        val sx = args["start_x"]?.jsonPrimitive?.content?.toFloatOrNull() ?: return ToolResult(false, "Missing 'start_x'")
        val sy = args["start_y"]?.jsonPrimitive?.content?.toFloatOrNull() ?: return ToolResult(false, "Missing 'start_y'")
        val ex = args["end_x"]?.jsonPrimitive?.content?.toFloatOrNull() ?: return ToolResult(false, "Missing 'end_x'")
        val ey = args["end_y"]?.jsonPrimitive?.content?.toFloatOrNull() ?: return ToolResult(false, "Missing 'end_y'")
        val ok = BizClawAccessibilityService.swipe(sx, sy, ex, ey)
        return ToolResult(ok, if (ok) "Swiped from ($sx,$sy) to ($ex,$ey)" else "Swipe failed")
    }

    private fun pressBack(): ToolResult {
        val ok = BizClawAccessibilityService.pressBack()
        return ToolResult(ok, if (ok) "Pressed Back" else "Back press failed")
    }

    private fun pressHome(): ToolResult {
        val ok = BizClawAccessibilityService.pressHome()
        return ToolResult(ok, if (ok) "Pressed Home" else "Home press failed")
    }

    private fun pressEnter(): ToolResult {
        val ok = BizClawAccessibilityService.pressEnter()
        return ToolResult(ok, if (ok) "Pressed Enter/Send" else "Enter press failed")
    }

    // ── App & System ─────────────────────────────────────────────

    private fun openApp(args: JsonObject): ToolResult {
        val pkg = args["package_name"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'package_name'")
        val ctrl = AppController(context)
        ctrl.openApp(pkg)
        return ToolResult(true, "Opened app")
    }

    private fun openUrl(args: JsonObject): ToolResult {
        val url = args["url"]?.jsonPrimitive?.content ?: return ToolResult(false, "Missing 'url'")
        val ctrl = AppController(context)
        ctrl.openUrl(url)
        return ToolResult(true, "Opened url")
    }

    private fun deviceInfo(): ToolResult {
        val caps = DeviceCapabilities(context)
        return ToolResult(true, buildString {
            appendLine("📱 Device Info:")
            appendLine("  Battery: ${caps.getBatteryInfo().level}%")
            appendLine("  Storage Free: ${caps.getStorageInfo().freeGb} GB")
            appendLine("  RAM Free: ${caps.getDeviceInfo().freeRamMb} MB")
            appendLine("  CPU Cores: ${Runtime.getRuntime().availableProcessors()}")
            appendLine("  Android: ${android.os.Build.VERSION.RELEASE}")
            appendLine("  Device: ${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}")
        })
    }

    private fun openNotifications(): ToolResult {
        val ok = BizClawAccessibilityService.openNotifications()
        return ToolResult(ok, if (ok) "Opened notification shade" else "Failed to open notifications")
    }
}

// ─── Result Types ─────────────────────────────────────────────

@Serializable
data class ToolResult(
    val success: Boolean,
    val message: String,
)

/**
 * Common Android package names for tool use
 */
object AppPackages {
    const val FACEBOOK = "com.facebook.katana"
    const val MESSENGER = "com.facebook.orca"
    const val ZALO = "com.zing.zalo"
    const val TELEGRAM = "org.telegram.messenger"
    const val CHROME = "com.android.chrome"
    const val CAMERA = "com.android.camera"
    const val SETTINGS = "com.android.settings"
    const val YOUTUBE = "com.google.android.youtube"
    const val TIKTOK = "com.zhiliaoapp.musically"
    const val INSTAGRAM = "com.instagram.android"
    const val GMAIL = "com.google.android.gm"
    const val MAPS = "com.google.android.apps.maps"
}
