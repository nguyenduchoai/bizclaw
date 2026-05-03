package vn.bizclaw.app.service

import android.util.Log

/**
 * ScreenDiffer — So sánh 2 lần đọc screen, chỉ gửi thay đổi cho LLM.
 *
 * Thay vì gửi toàn bộ screen content mỗi lần (30-50 elements × ~50 tokens/element = 2500 tokens),
 * chỉ gửi phần thay đổi (typically 5-10 elements = 500 tokens) → tiết kiệm 80% tokens.
 *
 * ```
 * Round N: screenRead() → [A, B, C, D, E]
 * Round N+1: screenRead() → [A, B, C, F, G]
 *
 * ScreenDiff:
 *   unchanged: 3 (A, B, C)
 *   removed:   2 (D, E) — no longer visible
 *   added:     2 (F, G) — newly appeared
 *
 * → Send to LLM: "3 elements unchanged. Removed: D, E. Added: F, G."
 *   Instead of sending all 5 elements again.
 * ```
 *
 * Integration:
 *   ToolDispatcher.screenRead() → ScreenDiffer.diff() → compact output → LLM
 */
class ScreenDiffer {

    companion object {
        private const val TAG = "ScreenDiffer"
    }

    // Previous screen state for comparison
    private var lastPackageName: String = ""
    private var lastElements: List<ElementSignature> = emptyList()
    private var isFirstRead = true

    /**
     * Compute diff between current screen and previous screen.
     * Returns formatted diff string for LLM consumption.
     *
     * @param content Current screen content from AccessibilityService
     * @return ScreenDiffResult with added/removed/unchanged counts + formatted string
     */
    fun diff(content: ScreenContent): ScreenDiffResult {
        val currentSigs = content.elements.map { it.toSignature() }
        val currentPkg = content.packageName

        // First read — no diff possible, return full content
        if (isFirstRead) {
            isFirstRead = false
            lastPackageName = currentPkg
            lastElements = currentSigs
            return ScreenDiffResult(
                appChanged = false,
                added = currentSigs,
                removed = emptyList(),
                unchanged = emptyList(),
                isFirstRead = true,
                formatted = formatFullScreen(content)
            )
        }

        // App changed — everything is "new"
        if (currentPkg != lastPackageName) {
            val result = ScreenDiffResult(
                appChanged = true,
                added = currentSigs,
                removed = lastElements,
                unchanged = emptyList(),
                isFirstRead = false,
                formatted = buildString {
                    appendLine("📱 App changed: $lastPackageName → $currentPkg")
                    appendLine(formatFullScreen(content))
                }
            )
            lastPackageName = currentPkg
            lastElements = currentSigs
            return result
        }

        // Same app — compute diff
        val added = currentSigs.filter { it !in lastElements }
        val removed = lastElements.filter { it !in currentSigs }
        val unchanged = currentSigs.filter { it in lastElements }

        val formatted = formatDiff(currentPkg, added, removed, unchanged, content.elementCount)

        val result = ScreenDiffResult(
            appChanged = false,
            added = added,
            removed = removed,
            unchanged = unchanged,
            isFirstRead = false,
            formatted = formatted
        )

        // Update state
        lastPackageName = currentPkg
        lastElements = currentSigs

        Log.d(TAG, "📊 Diff: +${added.size} -${removed.size} =${unchanged.size}")
        return result
    }

    /**
     * Reset state — call when starting a new conversation or goal.
     */
    fun reset() {
        lastPackageName = ""
        lastElements = emptyList()
        isFirstRead = true
    }

    // ─── Formatting ──────────────────────────────────────────────

    private fun formatFullScreen(content: ScreenContent): String = buildString {
        appendLine("App: ${content.packageName}")
        appendLine("Elements: ${content.elementCount}")
        appendLine("---")
        for (el in content.elements) {
            appendLine(formatElement(el))
        }
    }

    private fun formatDiff(
        pkg: String,
        added: List<ElementSignature>,
        removed: List<ElementSignature>,
        unchanged: List<ElementSignature>,
        totalCount: Int
    ): String = buildString {
        appendLine("App: $pkg (${totalCount} elements)")

        if (added.isEmpty() && removed.isEmpty()) {
            appendLine("📌 Screen unchanged")
            return@buildString
        }

        appendLine("📊 Changes: +${added.size} new, -${removed.size} removed, ${unchanged.size} unchanged")
        appendLine("---")

        if (added.isNotEmpty()) {
            appendLine("🟢 NEW elements:")
            for (sig in added) {
                appendLine("  + ${sig.formatted()}")
            }
        }

        if (removed.isNotEmpty()) {
            appendLine("🔴 REMOVED elements:")
            for (sig in removed) {
                appendLine("  - ${sig.formatted()}")
            }
        }

        // Only show unchanged count, not full details (saves tokens)
        if (unchanged.isNotEmpty()) {
            appendLine("⚪ ${unchanged.size} elements unchanged")
        }
    }

    private fun formatElement(el: ScreenElement): String {
        val tags = mutableListOf<String>()
        if (el.isClickable) tags.add("clickable")
        if (el.isEditable) tags.add("editable")
        if (el.isScrollable) tags.add("scrollable")
        val tagStr = if (tags.isNotEmpty()) " [${tags.joinToString(",")}]" else ""
        return when {
            el.text.isNotEmpty() -> "• ${el.className}: \"${el.text}\"$tagStr"
            el.contentDescription.isNotEmpty() -> "• ${el.className}: (${el.contentDescription})$tagStr"
            el.hint.isNotEmpty() -> "• ${el.className}: hint=\"${el.hint}\"$tagStr"
            else -> "• ${el.className}$tagStr"
        }
    }
}

// ─── Data Types ──────────────────────────────────────────────────

/**
 * Compact signature of a ScreenElement for comparison.
 * Two elements are "the same" if they have same class + text + bounds.
 */
data class ElementSignature(
    val className: String,
    val text: String,
    val contentDescription: String,
    val hint: String,
    val isClickable: Boolean,
    val isEditable: Boolean,
    val boundsKey: String,  // "left,top,right,bottom"
) {
    fun formatted(): String {
        val tags = mutableListOf<String>()
        if (isClickable) tags.add("clickable")
        if (isEditable) tags.add("editable")
        val tagStr = if (tags.isNotEmpty()) " [${tags.joinToString(",")}]" else ""
        return when {
            text.isNotEmpty() -> "$className: \"$text\"$tagStr"
            contentDescription.isNotEmpty() -> "$className: ($contentDescription)$tagStr"
            hint.isNotEmpty() -> "$className: hint=\"$hint\"$tagStr"
            else -> "$className$tagStr"
        }
    }
}

fun ScreenElement.toSignature(): ElementSignature = ElementSignature(
    className = className,
    text = text,
    contentDescription = contentDescription,
    hint = hint,
    isClickable = isClickable,
    isEditable = isEditable,
    boundsKey = "${bounds.left},${bounds.top},${bounds.right},${bounds.bottom}",
)

data class ScreenDiffResult(
    val appChanged: Boolean,
    val added: List<ElementSignature>,
    val removed: List<ElementSignature>,
    val unchanged: List<ElementSignature>,
    val isFirstRead: Boolean,
    val formatted: String,
) {
    val hasChanges: Boolean get() = added.isNotEmpty() || removed.isNotEmpty() || appChanged
    val tokensSaved: Int get() = unchanged.size * 8  // ~8 tokens per unchanged element not sent
}
