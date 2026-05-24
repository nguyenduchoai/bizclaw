package vn.bizclaw.companion

import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification

class CompanionNotificationListener : NotificationListenerService() {
    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val extras = sbn.notification.extras ?: return
        val title = extras.getCharSequence("android.title")?.toString().orEmpty()
        val text = extras.getCharSequence("android.text")?.toString().orEmpty()
        val content = listOf(title, text).filter { it.isNotBlank() }.joinToString(": ").trim()
        if (content.isBlank()) return
        if (!isBusinessChannel(sbn.packageName)) return

        if (CompanionPrefs.mode(this) == CompanionPrefs.MODE_DESKTOP) {
            Thread {
                runCatching {
                    LocalMama.postToDesktop(
                        context = this,
                        request = "[Notification ${sbn.packageName}] $content",
                        source = "android_notification",
                        customerRef = sbn.packageName,
                    )
                }.onFailure {
                    LocalMama.appendTicket(this, LocalMama.route(content, "notification_fallback"))
                }
            }.start()
        } else {
            LocalMama.appendTicket(this, LocalMama.route(content, "notification_local"))
        }
    }

    private fun isBusinessChannel(packageName: String): Boolean {
        val normalized = packageName.lowercase()
        return normalized.contains("zalo") ||
            normalized.contains("facebook") ||
            normalized.contains("messenger") ||
            normalized.contains("business") ||
            normalized.contains("mail")
    }
}
