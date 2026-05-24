package vn.bizclaw.companion

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent

class CompanionAccessibilityService : AccessibilityService() {
    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        val packageName = event?.packageName?.toString().orEmpty()
        if (packageName.isBlank()) return
        val text = event?.text?.joinToString(" ")?.trim().orEmpty()
        if (text.isNotBlank()) {
            LocalMama.recordScreen(this, packageName, text)
        }
    }

    override fun onInterrupt() = Unit
}
