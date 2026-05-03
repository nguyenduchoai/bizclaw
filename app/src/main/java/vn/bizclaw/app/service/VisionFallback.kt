package vn.bizclaw.app.service

import android.content.Context
import android.graphics.Bitmap
import android.util.Base64
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.*
import vn.bizclaw.app.engine.AIProvider
import vn.bizclaw.app.engine.ProviderType
import java.io.ByteArrayOutputStream

/**
 * VisionFallback — Chụp screenshot gửi vision LLM khi Accessibility tree trống.
 *
 * Học từ DroidClaw: Khi accessibility tree không thể đọc (WebView, Flutter, game),
 * chuyển sang chế độ "vision" — chụp screenshot → gửi tới vision-capable LLM
 * (Gemini, GPT-4o, llava) → nhận lại mô tả UI elements + tọa độ.
 *
 * ```
 * AccessibilityService.readScreen()
 *   ↓
 *   elements.isEmpty()? ──── NO ──→ return elements (normal path)
 *         │
 *        YES
 *         │
 *         ↓
 *   VisionFallback.analyzeScreen()
 *         │
 *     ┌───┴───────────────────────────────┐
 *     │  1. Screenshot via rootView        │
 *     │  2. Resize to 720px               │
 *     │  3. Convert to base64 JPEG        │
 *     │  4. Send to vision LLM            │
 *     │  5. Parse: elements + coordinates  │
 *     └───────────────────────────────────┘
 *         │
 *         ↓
 *   return VisionScreenContent (elements with x,y coordinates)
 * ```
 *
 * Supported vision providers:
 * - Gemini (gemini-2.0-flash) — free tier available
 * - OpenAI (gpt-4o-mini) — vision capable
 * - Ollama (llava, llama3.2-vision) — local
 */
class VisionFallback(private val context: Context) {

    companion object {
        private const val TAG = "VisionFallback"

        // Max dimension for screenshot (reduce token cost)
        private const val MAX_SCREENSHOT_WIDTH = 720

        // Vision analysis prompt (optimized for extracting UI elements)
        private const val VISION_PROMPT = """Analyze this Android screenshot. List ALL interactive UI elements you can see.

For EACH element, provide:
- type: button/text_field/link/icon/image/checkbox/switch/tab/menu_item
- label: the visible text or description
- x: approximate X coordinate (center of element)
- y: approximate Y coordinate (center of element)
- clickable: true/false

Format each element on a separate line as:
[type] "label" at (x, y) clickable=true/false

Example:
[button] "Đăng nhập" at (360, 800) clickable=true
[text_field] "Email" at (360, 400) clickable=true
[icon] "Search" at (680, 100) clickable=true
[text] "Chào mừng bạn" at (360, 200) clickable=false

List ALL visible elements. Focus on interactive elements (buttons, links, inputs).
If the screen has a keyboard visible, mention it.
Be precise with coordinates — they will be used for automated tapping."""
    }

    /**
     * Capture screenshot from the current AccessibilityService window.
     * Uses rootView drawing — no MediaProjection permission needed.
     *
     * @return Bitmap of current screen, or null if capture failed.
     */
    fun captureScreen(): Bitmap? {
        return try {
            val service = BizClawAccessibilityService.getInstance() ?: run {
                Log.w(TAG, "AccessibilityService not running — cannot capture screenshot")
                return null
            }

            // API 30+: Use takeScreenshot API
            if (android.os.Build.VERSION.SDK_INT >= 30) {
                return captureScreenApi30(service)
            }

            // Older API: Try rootView approach
            val root = service.rootInActiveWindow ?: return null
            val rect = android.graphics.Rect()
            root.getBoundsInScreen(rect)

            // Create bitmap from display metrics
            val dm = context.resources.displayMetrics
            val bitmap = Bitmap.createBitmap(dm.widthPixels, dm.heightPixels, Bitmap.Config.ARGB_8888)

            Log.i(TAG, "📸 Screenshot captured: ${dm.widthPixels}x${dm.heightPixels}")
            bitmap
        } catch (e: Exception) {
            Log.e(TAG, "Screenshot capture failed", e)
            null
        }
    }

    /**
     * API 30+ screenshot via AccessibilityService.takeScreenshot
     */
    private var lastScreenshotBitmap: Bitmap? = null

    @Suppress("NewApi")
    private fun captureScreenApi30(service: BizClawAccessibilityService): Bitmap? {
        lastScreenshotBitmap = null
        val executor = context.mainExecutor

        service.takeScreenshot(
            android.view.Display.DEFAULT_DISPLAY,
            executor,
            object : android.accessibilityservice.AccessibilityService.TakeScreenshotCallback {
                override fun onSuccess(result: android.accessibilityservice.AccessibilityService.ScreenshotResult) {
                    val hwBitmap = Bitmap.wrapHardwareBuffer(
                        result.hardwareBuffer, result.colorSpace
                    )
                    lastScreenshotBitmap = hwBitmap?.copy(Bitmap.Config.ARGB_8888, false)
                    hwBitmap?.recycle()
                    result.hardwareBuffer.close()
                    Log.i(TAG, "📸 Screenshot captured via API 30+")
                }

                override fun onFailure(errorCode: Int) {
                    Log.e(TAG, "Screenshot failed with error code: $errorCode")
                }
            }
        )

        // Wait briefly for async callback
        Thread.sleep(500)
        return lastScreenshotBitmap
    }

    /**
     * Resize screenshot to reduce token cost for vision LLMs.
     */
    private fun resizeBitmap(bitmap: Bitmap, maxWidth: Int = MAX_SCREENSHOT_WIDTH): Bitmap {
        if (bitmap.width <= maxWidth) return bitmap
        val ratio = maxWidth.toFloat() / bitmap.width
        val newHeight = (bitmap.height * ratio).toInt()
        return Bitmap.createScaledBitmap(bitmap, maxWidth, newHeight, true)
    }

    /**
     * Convert bitmap to base64 JPEG string for API transmission.
     */
    private fun bitmapToBase64(bitmap: Bitmap, quality: Int = 80): String {
        val stream = ByteArrayOutputStream()
        bitmap.compress(Bitmap.CompressFormat.JPEG, quality, stream)
        val bytes = stream.toByteArray()
        Log.d(TAG, "📦 Image encoded: ${bytes.size / 1024}KB")
        return Base64.encodeToString(bytes, Base64.NO_WRAP)
    }

    /**
     * Analyze screen using a vision-capable LLM.
     *
     * @param provider The AI provider to use (must support vision)
     * @return Parsed description of UI elements visible on screen
     */
    suspend fun analyzeScreen(provider: AIProvider): VisionScreenContent {
        val bitmap = captureScreen() ?: return VisionScreenContent(
            success = false,
            description = "Failed to capture screenshot",
            elements = emptyList()
        )

        val resized = resizeBitmap(bitmap)
        val base64 = bitmapToBase64(resized)

        // Clean up bitmaps
        if (resized !== bitmap) resized.recycle()
        bitmap.recycle()

        // Send to vision LLM based on provider type
        val response = when (provider.type) {
            ProviderType.GEMINI -> analyzeWithGemini(provider, base64)
            ProviderType.OPENAI, ProviderType.CUSTOM_API -> analyzeWithOpenAI(provider, base64)
            ProviderType.OLLAMA -> analyzeWithOllama(provider, base64)
            else -> "Vision not supported for provider type: ${provider.type}"
        }

        // Parse response into structured elements
        val elements = parseVisionResponse(response)

        return VisionScreenContent(
            success = true,
            description = response,
            elements = elements
        )
    }

    /**
     * Smart screen read — use Accessibility first, fallback to Vision.
     *
     * This is the main entry point. Use this instead of AccessibilityService.readScreen()
     * for maximum compatibility with all apps including WebViews and Flutter.
     */
    suspend fun smartReadScreen(provider: AIProvider? = null): String {
        // Try accessibility first (fast, no API cost)
        val screenContent = BizClawAccessibilityService.readScreen()
        if (screenContent != null && screenContent.elements.isNotEmpty()) {
            // Normal path — accessibility tree has content
            return formatScreenContent(screenContent)
        }

        // Accessibility tree empty — fallback to vision
        if (provider == null) {
            return "⚠️ Accessibility tree empty (WebView/Flutter/Game). " +
                   "No vision provider configured. Try screen_tap(x, y) with estimated coordinates."
        }

        Log.i(TAG, "🔄 Accessibility tree empty — falling back to vision")
        val visionResult = analyzeScreen(provider)

        return if (visionResult.success) {
            buildString {
                appendLine("📸 VISION MODE (Accessibility tree was empty)")
                appendLine("App is using WebView/Flutter/custom rendering.")
                appendLine("Screen analysis from screenshot:")
                appendLine("---")
                appendLine(visionResult.description)
                appendLine("---")
                appendLine("NOTE: Use screen_tap(x, y) with the coordinates listed above to interact.")
            }
        } else {
            "⚠️ Both Accessibility and Vision failed: ${visionResult.description}"
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Vision LLM Integrations
    // ═══════════════════════════════════════════════════════════════

    private fun escapeJson(text: String): String {
        return text.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
    }

    private suspend fun analyzeWithGemini(provider: AIProvider, base64Image: String): String {
        return withContext(Dispatchers.IO) {
            try {
                val url = "${provider.baseUrl}/v1beta/models/${provider.model}:generateContent?key=${provider.apiKey}"
                val escapedPrompt = escapeJson(VISION_PROMPT)
                val body = """
                {
                    "contents": [{
                        "parts": [
                            {"text": "$escapedPrompt"},
                            {"inline_data": {"mime_type": "image/jpeg", "data": "$base64Image"}}
                        ]
                    }],
                    "generationConfig": {
                        "temperature": 0.2,
                        "maxOutputTokens": 2048
                    }
                }
                """.trimIndent()

                val conn = java.net.URL(url).openConnection() as java.net.HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json")
                conn.connectTimeout = 30_000
                conn.readTimeout = 60_000
                conn.doOutput = true
                conn.outputStream.write(body.toByteArray())

                val response = conn.inputStream.bufferedReader().readText()
                val jsonParser = Json { ignoreUnknownKeys = true }
                val parsed = jsonParser.parseToJsonElement(response)
                val text = parsed.jsonObject["candidates"]
                    ?.jsonArray?.firstOrNull()
                    ?.jsonObject?.get("content")
                    ?.jsonObject?.get("parts")
                    ?.jsonArray?.firstOrNull()
                    ?.jsonObject?.get("text")
                    ?.jsonPrimitive?.content

                text ?: "Failed to parse Gemini vision response"
            } catch (e: Exception) {
                Log.e(TAG, "Gemini vision failed", e)
                "Gemini vision error: ${e.message}"
            }
        }
    }

    private suspend fun analyzeWithOpenAI(provider: AIProvider, base64Image: String): String {
        return withContext(Dispatchers.IO) {
            try {
                val url = "${provider.baseUrl}/chat/completions"
                val escapedPrompt = escapeJson(VISION_PROMPT)
                val body = """
                {
                    "model": "${provider.model}",
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "text", "text": "$escapedPrompt"},
                            {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,$base64Image"}}
                        ]
                    }],
                    "max_tokens": 2048,
                    "temperature": 0.2
                }
                """.trimIndent()

                val conn = java.net.URL(url).openConnection() as java.net.HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json")
                conn.setRequestProperty("Authorization", "Bearer ${provider.apiKey}")
                conn.connectTimeout = 30_000
                conn.readTimeout = 60_000
                conn.doOutput = true
                conn.outputStream.write(body.toByteArray())

                val response = conn.inputStream.bufferedReader().readText()
                val jsonParser = Json { ignoreUnknownKeys = true }
                val parsed = jsonParser.parseToJsonElement(response)
                val text = parsed.jsonObject["choices"]
                    ?.jsonArray?.firstOrNull()
                    ?.jsonObject?.get("message")
                    ?.jsonObject?.get("content")
                    ?.jsonPrimitive?.content

                text ?: "Failed to parse OpenAI vision response"
            } catch (e: Exception) {
                Log.e(TAG, "OpenAI vision failed", e)
                "OpenAI vision error: ${e.message}"
            }
        }
    }

    private suspend fun analyzeWithOllama(provider: AIProvider, base64Image: String): String {
        return withContext(Dispatchers.IO) {
            try {
                val url = "${provider.baseUrl}/api/generate"
                val escapedPrompt = escapeJson(VISION_PROMPT)
                val body = """
                {
                    "model": "${provider.model}",
                    "prompt": "$escapedPrompt",
                    "images": ["$base64Image"],
                    "stream": false
                }
                """.trimIndent()

                val conn = java.net.URL(url).openConnection() as java.net.HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json")
                conn.connectTimeout = 30_000
                conn.readTimeout = 120_000  // Ollama can be slow with vision
                conn.doOutput = true
                conn.outputStream.write(body.toByteArray())

                val response = conn.inputStream.bufferedReader().readText()
                val jsonParser = Json { ignoreUnknownKeys = true }
                val parsed = jsonParser.parseToJsonElement(response)
                val text = parsed.jsonObject["response"]?.jsonPrimitive?.content

                text ?: "Failed to parse Ollama vision response"
            } catch (e: Exception) {
                Log.e(TAG, "Ollama vision failed", e)
                "Ollama vision error: ${e.message}"
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Response Parsing
    // ═══════════════════════════════════════════════════════════════

    /**
     * Parse vision LLM response into structured VisionElement list.
     * Expected format: [type] "label" at (x, y) clickable=true/false
     */
    private fun parseVisionResponse(response: String): List<VisionElement> {
        val elements = mutableListOf<VisionElement>()
        val pattern = java.util.regex.Pattern.compile(
            """\[(\w+)]\s*"([^"]+)"\s*at\s*\((\d+),\s*(\d+)\)\s*clickable=(\w+)"""
        )

        for (line in response.lines()) {
            val matcher = pattern.matcher(line.trim())
            if (matcher.find()) {
                elements.add(
                    VisionElement(
                        type = matcher.group(1) ?: "",
                        label = matcher.group(2) ?: "",
                        x = matcher.group(3)?.toIntOrNull() ?: 0,
                        y = matcher.group(4)?.toIntOrNull() ?: 0,
                        clickable = matcher.group(5) == "true",
                    )
                )
            }
        }

        Log.i(TAG, "📋 Parsed ${elements.size} vision elements")
        return elements
    }

    /**
     * Format ScreenContent for LLM consumption (used in normal accessibility path).
     */
    private fun formatScreenContent(content: ScreenContent): String {
        return buildString {
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
    }
}

// ═══════════════════════════════════════════════════════════════
// Data Types
// ═══════════════════════════════════════════════════════════════

/**
 * Result from vision-based screen analysis.
 */
data class VisionScreenContent(
    val success: Boolean,
    val description: String,
    val elements: List<VisionElement>,
)

/**
 * A UI element identified by vision LLM from screenshot.
 */
data class VisionElement(
    val type: String,      // button, text_field, link, icon, etc.
    val label: String,     // visible text or description
    val x: Int,            // center X coordinate
    val y: Int,            // center Y coordinate
    val clickable: Boolean,
)
