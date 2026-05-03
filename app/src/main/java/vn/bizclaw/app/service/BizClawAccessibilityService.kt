package vn.bizclaw.app.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.accessibilityservice.GestureDescription
import android.content.Intent
import android.graphics.Path
import android.os.Bundle
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * BizClaw Accessibility Service — enables AI agents to control ANY app on the phone.
 *
 * Capabilities:
 * - Read screen content (text, buttons, input fields)
 * - Click any UI element by text, ID, or position
 * - Type text into input fields
 * - Scroll, swipe, navigate
 * - Read notifications
 *
 * Use cases:
 * - Post to Facebook (find "Bạn đang nghĩ gì?" → tap → type → post)
 * - Reply in Messenger (find conversation → tap → type → send)
 * - Comment on posts (find comment field → type → submit)
 * - Like/react to posts
 * - Read and respond to Zalo, Telegram, etc.
 *
 * ⚠️ IMPORTANT: User must manually enable this in:
 *    Settings → Accessibility → BizClaw Agent → Enable
 *
 * Flow:
 *    Agent tool call → BizClawDaemonService → AppController → AccessibilityService
 */
class BizClawAccessibilityService : AccessibilityService() {

    companion object {
        private var instance: BizClawAccessibilityService? = null
        private val json = Json { ignoreUnknownKeys = true; prettyPrint = true }

        fun isRunning(): Boolean = instance != null

        fun getInstance(): BizClawAccessibilityService? = instance

        // ─── Public API for Agents ────────────────────────────────

        /**
         * Get all text visible on the current screen.
         */
        fun readScreen(): ScreenContent? {
            val service = instance ?: return null
            val root = service.rootInActiveWindow ?: return null

            val elements = mutableListOf<ScreenElement>()
            collectElements(root, elements, depth = 0)

            return ScreenContent(
                packageName = root.packageName?.toString() ?: "unknown",
                elementCount = elements.size,
                elements = elements.take(50), // Limit to prevent OOM
            )
        }

        /**
         * Find a UI element by text content.
         */
        fun findByText(text: String): List<ScreenElement> {
            val service = instance ?: return emptyList()
            val root = service.rootInActiveWindow ?: return emptyList()

            val results = mutableListOf<ScreenElement>()
            val nodes = root.findAccessibilityNodeInfosByText(text)
            for (node in nodes) {
                results.add(nodeToElement(node))
            }
            return results
        }

        /**
         * Click on a UI element containing the given text.
         * Returns true if clicked successfully.
         */
        fun clickByText(text: String): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            val nodes = root.findAccessibilityNodeInfosByText(text)
            for (node in nodes) {
                if (node.isClickable) {
                    val result = node.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                    if (result) return true
                }
                // If node isn't clickable, try parent
                var parent = node.parent
                while (parent != null) {
                    if (parent.isClickable) {
                        val result = parent.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                        if (result) return true
                    }
                    parent = parent.parent
                }
            }
            return false
        }

        /**
         * Type text into the currently focused input field.
         */
        fun typeText(text: String): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            // Find focused editable field
            val editField = findFocusedEditText(root)
            if (editField != null) {
                val args = Bundle()
                args.putCharSequence(
                    AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                    text
                )
                return editField.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
            }

            return false
        }

        /**
         * Type text into an input field found by hint/placeholder text.
         */
        fun typeIntoField(hintText: String, text: String): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            val editFields = findEditableFields(root)
            for (field in editFields) {
                val hint = field.hintText?.toString() ?: ""
                val fieldText = field.text?.toString() ?: ""
                val desc = field.contentDescription?.toString() ?: ""

                if (hint.contains(hintText, ignoreCase = true) ||
                    fieldText.contains(hintText, ignoreCase = true) ||
                    desc.contains(hintText, ignoreCase = true)
                ) {
                    // Focus the field
                    field.performAction(AccessibilityNodeInfo.ACTION_FOCUS)
                    field.performAction(AccessibilityNodeInfo.ACTION_CLICK)

                    // Set text
                    val args = Bundle()
                    args.putCharSequence(
                        AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                        text
                    )
                    return field.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
                }
            }
            return false
        }

        /**
         * Type text into the first available editable field.
         */
        fun typeIntoAnyField(text: String): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            val editFields = findEditableFields(root)
            if (editFields.isNotEmpty()) {
                val field = editFields.first()
                field.performAction(AccessibilityNodeInfo.ACTION_FOCUS)
                field.performAction(AccessibilityNodeInfo.ACTION_CLICK)

                val args = Bundle()
                args.putCharSequence(
                    AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                    text
                )
                return field.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
            }
            return false
        }

        /**
         * Press Enter/IME action (send message, submit form).
         * Note: ACTION_IME_ENTER requires API 30+.
         * Fallback: click send button by common text.
         */
        fun pressEnter(): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            val editField = findFocusedEditText(root)
            if (editField != null) {
                // API 30+: use ACTION_IME_ENTER
                if (android.os.Build.VERSION.SDK_INT >= 30) {
                    val result = editField.performAction(
                        AccessibilityNodeInfo.AccessibilityAction.ACTION_IME_ENTER.id
                    )
                    if (result) return true
                }
                // Fallback: try to find and click a send button nearby
                return clickByText("Send")
                    || clickByText("Gửi")
                    || clickByText("➤")
                    || clickByText("►")
            }
            return false
        }

        /**
         * Scroll down on the current screen.
         */
        fun scrollDown(): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            return findScrollable(root)?.performAction(
                AccessibilityNodeInfo.ACTION_SCROLL_FORWARD
            ) ?: false
        }

        /**
         * Scroll up on the current screen.
         */
        fun scrollUp(): Boolean {
            val service = instance ?: return false
            val root = service.rootInActiveWindow ?: return false

            return findScrollable(root)?.performAction(
                AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD
            ) ?: false
        }

        /**
         * Press the global Back button.
         */
        fun pressBack(): Boolean {
            return instance?.performGlobalAction(GLOBAL_ACTION_BACK) ?: false
        }

        /**
         * Press the global Home button.
         */
        fun pressHome(): Boolean {
            return instance?.performGlobalAction(GLOBAL_ACTION_HOME) ?: false
        }

        /**
         * Open the recent apps view.
         */
        fun openRecents(): Boolean {
            return instance?.performGlobalAction(GLOBAL_ACTION_RECENTS) ?: false
        }

        /**
         * Open the notification shade.
         */
        fun openNotifications(): Boolean {
            return instance?.performGlobalAction(GLOBAL_ACTION_NOTIFICATIONS) ?: false
        }

        /**
         * Tap at specific screen coordinates.
         */
        fun tapAt(x: Float, y: Float): Boolean {
            val service = instance ?: return false

            val path = Path()
            path.moveTo(x, y)

            val gesture = GestureDescription.Builder()
                .addStroke(GestureDescription.StrokeDescription(path, 0, 100))
                .build()

            return service.dispatchGesture(gesture, null, null)
        }

        /**
         * Swipe gesture (e.g., scroll, pull-to-refresh).
         */
        fun swipe(startX: Float, startY: Float, endX: Float, endY: Float, durationMs: Long = 300): Boolean {
            val service = instance ?: return false

            val path = Path()
            path.moveTo(startX, startY)
            path.lineTo(endX, endY)

            val gesture = GestureDescription.Builder()
                .addStroke(GestureDescription.StrokeDescription(path, 0, durationMs))
                .build()

            return service.dispatchGesture(gesture, null, null)
        }

        // ─── Helper Functions ──────────────────────────────────────

        private fun collectElements(
            node: AccessibilityNodeInfo?,
            result: MutableList<ScreenElement>,
            depth: Int,
        ) {
            if (node == null || depth > 15) return

            val element = nodeToElement(node)
            if (element.text.isNotEmpty() || element.isClickable || element.isEditable) {
                result.add(element)
            }

            for (i in 0 until node.childCount) {
                collectElements(node.getChild(i), result, depth + 1)
            }
        }

        private fun nodeToElement(node: AccessibilityNodeInfo): ScreenElement {
            val rect = android.graphics.Rect()
            node.getBoundsInScreen(rect)

            return ScreenElement(
                text = node.text?.toString() ?: "",
                contentDescription = node.contentDescription?.toString() ?: "",
                className = node.className?.toString()?.substringAfterLast('.') ?: "",
                isClickable = node.isClickable,
                isEditable = node.isEditable,
                isScrollable = node.isScrollable,
                hint = node.hintText?.toString() ?: "",
                bounds = ElementBounds(rect.left, rect.top, rect.right, rect.bottom),
            )
        }

        private fun findFocusedEditText(root: AccessibilityNodeInfo): AccessibilityNodeInfo? {
            val focused = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            if (focused?.isEditable == true) return focused
            return findEditableFields(root).firstOrNull()
        }

        private fun findEditableFields(node: AccessibilityNodeInfo): List<AccessibilityNodeInfo> {
            val result = mutableListOf<AccessibilityNodeInfo>()
            collectEditableFields(node, result)
            return result
        }

        private fun collectEditableFields(
            node: AccessibilityNodeInfo?,
            result: MutableList<AccessibilityNodeInfo>,
        ) {
            if (node == null) return
            if (node.isEditable) result.add(node)
            for (i in 0 until node.childCount) {
                collectEditableFields(node.getChild(i), result)
            }
        }

        private fun findScrollable(node: AccessibilityNodeInfo?): AccessibilityNodeInfo? {
            if (node == null) return null
            if (node.isScrollable) return node
            for (i in 0 until node.childCount) {
                val found = findScrollable(node.getChild(i))
                if (found != null) return found
            }
            return null
        }
    }

    // ─── Service Lifecycle ────────────────────────────────────────

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this

        serviceInfo = AccessibilityServiceInfo().apply {
            eventTypes = AccessibilityEvent.TYPES_ALL_MASK
            feedbackType = AccessibilityServiceInfo.FEEDBACK_GENERIC
            flags = AccessibilityServiceInfo.FLAG_INCLUDE_NOT_IMPORTANT_VIEWS or
                    AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS or
                    AccessibilityServiceInfo.DEFAULT
            notificationTimeout = 100
        }

        android.util.Log.i("BizClaw", "♿ Accessibility service connected — agent can control apps")
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Events are received but we primarily use on-demand screen reading
        // Could be used for: notification monitoring, app change detection
        when (event?.eventType) {
            AccessibilityEvent.TYPE_NOTIFICATION_STATE_CHANGED -> {
                val text = event.text.joinToString(" ")
                val pkg = event.packageName?.toString() ?: ""
                android.util.Log.d("BizClaw", "📬 Notification: [$pkg] $text")
                // Forward to notification listener for auto-reply decisions
                // The NotificationListener already handles this via onNotificationPosted,
                // but AccessibilityService catches in-app notifications that don't
                // appear in the notification shade
                BizClawNotificationListener.onNotificationReceived?.invoke(
                    BizClawNotificationListener.SocialNotification(
                        app = BizClawNotificationListener.MONITORED_APPS[pkg] ?: pkg,
                        packageName = pkg,
                        sender = "",
                        message = text,
                    )
                )
            }
            else -> {} // Ignore other events for now
        }
    }

    override fun onInterrupt() {
        android.util.Log.w("BizClaw", "♿ Accessibility service interrupted")
    }

    override fun onDestroy() {
        super.onDestroy()
        instance = null
        android.util.Log.i("BizClaw", "♿ Accessibility service destroyed")
    }
}

// ─── Data Classes ─────────────────────────────────────────────────────

@Serializable
data class ScreenContent(
    val packageName: String,
    val elementCount: Int,
    val elements: List<ScreenElement>,
)

@Serializable
data class ScreenElement(
    val text: String = "",
    val contentDescription: String = "",
    val className: String = "",
    val isClickable: Boolean = false,
    val isEditable: Boolean = false,
    val isScrollable: Boolean = false,
    val hint: String = "",
    val bounds: ElementBounds = ElementBounds(),
)

@Serializable
data class ElementBounds(
    val left: Int = 0,
    val top: Int = 0,
    val right: Int = 0,
    val bottom: Int = 0,
)
