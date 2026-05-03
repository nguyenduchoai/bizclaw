package vn.bizclaw.app.service

import android.util.Log

/**
 * StuckDetector — Phát hiện agent bị kẹt và đề xuất recovery.
 *
 * Học từ DroidClaw: 3 loại stuck detection:
 * 1. Screen Frozen — không thay đổi sau N rounds
 * 2. Action Loop — lặp cùng 1 action liên tục
 * 3. Navigation Drift — spam back/home/scroll mà không interact
 *
 * Integration:
 *   LocalAgentLoop calls `onRoundComplete()` after each tool execution.
 *   If stuck is detected, a recovery hint is injected into the next LLM prompt.
 *
 * ```
 *  Round 1: screen_read → "5 elements" ✅
 *  Round 2: screen_click("Login") → failed → screen_read → "5 elements"
 *  Round 3: screen_click("Login") → failed → screen_read → "5 elements"
 *  ──► STUCK: same screen + same action for 3 rounds
 *  ──► Inject: "Screen hasn't changed. Try a different approach."
 * ```
 */
class StuckDetector(
    private val screenFrozenThreshold: Int = 3,
    private val actionLoopThreshold: Int = 3,
    private val driftThreshold: Int = 4,
) {
    private val tag = "StuckDetector"

    // Sliding windows
    private val screenSnapshots = mutableListOf<String>()
    private val actionHistory = mutableListOf<ActionRecord>()

    data class ActionRecord(
        val toolName: String,
        val targetText: String = "",   // e.g. text clicked, field typed into
        val success: Boolean = true,
    )

    /**
     * Record a completed round — screen state + action taken.
     *
     * @param screenFingerprint A hash or summary of the current screen state.
     *        Use element count + package name + first N element texts.
     * @param action The tool action that was executed.
     * @return A [StuckHint] if stuck is detected, null otherwise.
     */
    fun onRoundComplete(screenFingerprint: String, action: ActionRecord): StuckHint? {
        screenSnapshots.add(screenFingerprint)
        actionHistory.add(action)

        // Keep sliding windows bounded
        if (screenSnapshots.size > 10) screenSnapshots.removeFirst()
        if (actionHistory.size > 10) actionHistory.removeFirst()

        // Check 1: Screen Frozen
        if (screenSnapshots.size >= screenFrozenThreshold) {
            val recent = screenSnapshots.takeLast(screenFrozenThreshold)
            if (recent.distinct().size == 1) {
                Log.w(tag, "🔴 STUCK: Screen frozen for $screenFrozenThreshold rounds")
                return StuckHint.SCREEN_FROZEN
            }
        }

        // Check 2: Action Loop (same tool + same target repeated)
        if (actionHistory.size >= actionLoopThreshold) {
            val recent = actionHistory.takeLast(actionLoopThreshold)
            val uniqueActions = recent.map { "${it.toolName}:${it.targetText}" }.distinct()
            if (uniqueActions.size == 1) {
                Log.w(tag, "🔴 STUCK: Action loop detected — ${recent.first().toolName}")
                return StuckHint.ACTION_LOOP
            }
        }

        // Check 3: Navigation Drift (only nav actions, no interaction)
        if (actionHistory.size >= driftThreshold) {
            val navActions = setOf(
                "press_back", "press_home", "screen_scroll_down",
                "screen_scroll_up", "screen_swipe", "notifications"
            )
            val recent = actionHistory.takeLast(driftThreshold)
            if (recent.all { it.toolName in navActions }) {
                Log.w(tag, "🔴 STUCK: Navigation drift — ${recent.map { it.toolName }}")
                return StuckHint.NAVIGATION_DRIFT
            }
        }

        // Check 4: Repeated failures
        if (actionHistory.size >= 3) {
            val recent = actionHistory.takeLast(3)
            if (recent.all { !it.success }) {
                Log.w(tag, "🔴 STUCK: 3 consecutive failures")
                return StuckHint.REPEATED_FAILURES
            }
        }

        // Check 5: Cyclical repetition pattern (A-B-A-B or A-B-C-A-B-C)
        if (actionHistory.size >= 6) {
            val keys = actionHistory.map { "${it.toolName}:${it.targetText}" }
            // Check for period-2 cycle: A-B-A-B
            if (keys.size >= 4) {
                val last4 = keys.takeLast(4)
                if (last4[0] == last4[2] && last4[1] == last4[3]) {
                    Log.w(tag, "🔴 STUCK: Cyclical repetition (period 2)")
                    return StuckHint.REPETITION_CYCLE
                }
            }
            // Check for period-3 cycle: A-B-C-A-B-C
            if (keys.size >= 6) {
                val last6 = keys.takeLast(6)
                if (last6[0] == last6[3] && last6[1] == last6[4] && last6[2] == last6[5]) {
                    Log.w(tag, "🔴 STUCK: Cyclical repetition (period 3)")
                    return StuckHint.REPETITION_CYCLE
                }
            }
        }

        return null
    }

    /**
     * Reset state — call when starting a new goal/conversation.
     */
    fun reset() {
        screenSnapshots.clear()
        actionHistory.clear()
    }

    /**
     * Generate the current screen fingerprint from AccessibilityService data.
     */
    companion object {
        fun fingerprint(screenContent: ScreenContent?): String {
            if (screenContent == null) return "null"
            val elements = screenContent.elements.take(20)
            val texts = elements.joinToString("|") {
                "${it.className}:${it.text.take(30)}"
            }
            return "${screenContent.packageName}|${screenContent.elementCount}|$texts"
        }
    }
}

/**
 * Types of stuck conditions detected.
 * Each type has a specific recovery hint for the LLM.
 */
enum class StuckHint {
    /**
     * Screen hasn't changed for N rounds.
     * Recovery: Try different approach, check if element exists, try scrolling.
     */
    SCREEN_FROZEN,

    /**
     * Same action repeated N times (e.g., tapping same button).
     * Recovery: Stop repeating, try alternative element or different strategy.
     */
    ACTION_LOOP,

    /**
     * Only navigation actions (back, home, scroll) without interacting.
     * Recovery: Stop navigating, interact with a specific element.
     */
    NAVIGATION_DRIFT,

    /**
     * Multiple consecutive actions failed.
     * Recovery: Read screen first, verify target exists, try simpler approach.
     */
    REPEATED_FAILURES,

    /**
     * Cyclical repetition: A-B-A-B or A-B-C-A-B-C pattern detected.
     * Recovery: Break the cycle completely, try entirely different approach.
     */
    REPETITION_CYCLE;

    /**
     * Get the recovery hint text to inject into the LLM prompt.
     * Bilingual (Vietnamese + English) for better LLM understanding.
     */
    fun recoveryHint(): String = when (this) {
        SCREEN_FROZEN -> """
            |⚠️ STUCK DETECTED: The screen has not changed after multiple attempts.
            |Your previous actions had NO effect on the screen.
            |
            |Recovery suggestions:
            |1. Use screen_read() to check what's actually on screen
            |2. The element you're looking for may not exist — try scrolling down
            |3. Try using screen_tap(x, y) with coordinates instead of screen_click
            |4. The app might need time to load — try a different approach
            |5. If nothing works, inform the user that automation failed
            |
            |DO NOT repeat the same action. Try something DIFFERENT.
        """.trimMargin()

        ACTION_LOOP -> """
            |⚠️ STUCK DETECTED: You are repeating the same action multiple times.
            |This action is not working. STOP and try a different approach.
            |
            |Recovery suggestions:
            |1. Use screen_read() to see what elements are available
            |2. The text you're clicking might not be exactly matching — check spelling
            |3. Try clicking a parent element or nearby element instead
            |4. Use screen_tap(x, y) with specific coordinates
            |5. Try pressing back and re-navigating
            |
            |DO NOT repeat the same action again.
        """.trimMargin()

        NAVIGATION_DRIFT -> """
            |⚠️ STUCK DETECTED: You are only pressing back/home/scrolling without interacting.
            |This means you're lost and not making progress toward the goal.
            |
            |Recovery suggestions:
            |1. STOP navigating. Use screen_read() to see where you are
            |2. Identify a specific element to interact with
            |3. If you're in the wrong app, use open_app() to go to the right one
            |4. Take a direct action — click something, type something
            |
            |Focus on the original goal and take DIRECT action.
        """.trimMargin()

        REPEATED_FAILURES -> """
            |⚠️ STUCK DETECTED: Multiple actions have failed in a row.
            |The tools are not finding the elements you're targeting.
            |
            |Recovery suggestions:
            |1. Use screen_read() FIRST to see what's actually on the screen
            |2. Check if the app has loaded — elements might not be ready yet
            |3. Accessibility Service might not see this content (WebView or Flutter)
            |4. Try screen_tap(x, y) with coordinates as a fallback
            |5. If the app is not responding, try open_app() again
            |
            |Read the screen state BEFORE trying any more actions.
        """.trimMargin()

        REPETITION_CYCLE -> """
            |⚠️ STUCK DETECTED: You are stuck in a CYCLE — repeating the same pattern of actions.
            |Pattern: you keep doing the same sequence of actions over and over.
            |
            |Recovery suggestions:
            |1. STOP completely. Do not repeat any recent actions.
            |2. Think about WHY the previous approach failed.
            |3. Try a COMPLETELY different strategy to achieve the goal.
            |4. If targeting a specific element, try screen_capture() for vision analysis.
            |5. Consider breaking the task into smaller steps.
            |6. If the task seems impossible, inform the user honestly.
            |
            |Break the cycle. Do something you have NOT tried before.
        """.trimMargin()
    }
}
