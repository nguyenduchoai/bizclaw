package vn.bizclaw.agent.messaging

import android.accessibilityservice.AccessibilityService
import android.content.ComponentName
import android.content.Context
import android.os.Bundle
import android.provider.Settings
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import kotlinx.coroutines.delay

/**
 * Drives the chat apps' own UI, for the one thing notifications cannot do: start a
 * conversation nobody messaged us first.
 *
 * This is the fragile half of the app by construction — it depends on Zalo's and
 * Messenger's layouts, which change without warning. Every step reports its own failure
 * so the owner is told "couldn't find the search box" rather than silently getting no
 * message sent. Nothing here runs unless the owner explicitly triggers it.
 */
class ChatAutomationService : AccessibilityService() {

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
    }

    override fun onDestroy() {
        if (instance === this) instance = null
        super.onDestroy()
    }

    // The service is driven imperatively by ProactiveSender; events are only what keeps
    // the binding alive.
    override fun onAccessibilityEvent(event: AccessibilityEvent?) = Unit

    override fun onInterrupt() = Unit

    /** Current screen root, or null if no window is readable. */
    fun root(): AccessibilityNodeInfo? = rootInActiveWindow

    /**
     * Waits for a node whose text or content-description contains [needle].
     *
     * Polls rather than listening for events: a chat app repaints many times while a
     * conversation loads, and a single event snapshot usually catches the wrong frame.
     */
    suspend fun awaitNode(
        needle: String,
        timeoutMs: Long = 6_000,
        predicate: (AccessibilityNodeInfo) -> Boolean = { true },
    ): AccessibilityNodeInfo? {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            root()?.let { root ->
                findAll(root).firstOrNull { node ->
                    node.matches(needle) && predicate(node)
                }?.let { return it }
            }
            delay(POLL_MS)
        }
        return null
    }

    /** Waits for the first editable field on screen — the message box, in practice. */
    suspend fun awaitEditable(timeoutMs: Long = 6_000): AccessibilityNodeInfo? {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            root()?.let { root ->
                findAll(root).firstOrNull { it.isEditable && it.isVisibleToUser }
                    ?.let { return it }
            }
            delay(POLL_MS)
        }
        return null
    }

    /**
     * Clicks [node], walking up to the nearest clickable ancestor if needed.
     *
     * Chat apps render list rows as a non-clickable text node inside a clickable
     * container, so clicking the node that matched the search text usually does nothing.
     */
    fun click(node: AccessibilityNodeInfo): Boolean {
        var current: AccessibilityNodeInfo? = node
        var hops = 0
        while (current != null && hops < MAX_ANCESTOR_HOPS) {
            if (current.isClickable && current.performAction(AccessibilityNodeInfo.ACTION_CLICK)) {
                return true
            }
            current = current.parent
            hops++
        }
        return false
    }

    fun setText(node: AccessibilityNodeInfo, text: String): Boolean {
        val args = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text)
        }
        return node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
    }

    fun back(): Boolean = performGlobalAction(GLOBAL_ACTION_BACK)

    private fun findAll(root: AccessibilityNodeInfo): List<AccessibilityNodeInfo> {
        val out = mutableListOf<AccessibilityNodeInfo>()
        val queue = ArrayDeque(listOf(root))
        var visited = 0
        while (queue.isNotEmpty() && visited < MAX_NODES) {
            val node = queue.removeFirst()
            visited++
            out += node
            for (i in 0 until node.childCount) {
                node.getChild(i)?.let(queue::addLast)
            }
        }
        return out
    }

    private fun AccessibilityNodeInfo.matches(needle: String): Boolean {
        val folded = needle.lowercase()
        return text?.toString()?.lowercase()?.contains(folded) == true ||
            contentDescription?.toString()?.lowercase()?.contains(folded) == true
    }

    companion object {
        private const val POLL_MS = 250L
        private const val MAX_ANCESTOR_HOPS = 6

        /** Guard against pathological trees; a chat screen is a few hundred nodes. */
        private const val MAX_NODES = 3_000

        @Volatile
        private var instance: ChatAutomationService? = null

        /** The bound service, or null when the owner has not enabled it. */
        fun current(): ChatAutomationService? = instance

        fun isEnabled(context: Context): Boolean {
            val enabled = Settings.Secure.getString(
                context.contentResolver,
                Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES,
            ).orEmpty()
            val component = ComponentName(context, ChatAutomationService::class.java)
            return enabled.split(":").any {
                ComponentName.unflattenFromString(it) == component
            }
        }
    }
}
