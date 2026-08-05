package vn.bizclaw.agent.messaging

import android.app.Notification
import android.content.ComponentName
import android.content.Context
import android.provider.Settings
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.agent.ReplyAgent
import vn.bizclaw.agent.data.ReplyState

private const val TAG = "MessageListener"

/**
 * Watches Zalo/Messenger notifications, asks Gemma for a reply, and either sends it or
 * parks it as a draft.
 *
 * Notifications are the integration point rather than Accessibility because they are a
 * stable public API: chat apps redesign their UI constantly, but the reply action a
 * smartwatch uses has to keep working.
 */
class MessageListenerService : NotificationListenerService() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val agent by lazy { ReplyAgent(applicationContext) }
    private val seen = object : LinkedHashMap<String, Long>() {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Long>?) = size > 200
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val app = BizClawApp.from(applicationContext)
        if (!app.settings.agentEnabled) return

        val channel = SupportedApps.channelOf(sbn.packageName) ?: return
        val notification = sbn.notification ?: return

        // Group summaries repeat text already handled by the child notifications.
        if (notification.flags and Notification.FLAG_GROUP_SUMMARY != 0) return

        val sender = senderOf(notification) ?: return
        val body = bodyOf(notification)?.takeIf { it.isNotBlank() } ?: return

        // Chat apps repost the same notification on every UI tick; only the first copy
        // of a given sender+text is a new customer message.
        val fingerprint = "${sbn.packageName}|$sender|$body"
        if (seen.put(fingerprint, System.currentTimeMillis()) != null) return

        val replyAction = ReplySender.replyActionOf(notification)
        if (replyAction == null) {
            Log.d(TAG, "No inline reply action on ${sbn.packageName}; skipping")
            return
        }

        val exchangeId = "ex-${System.currentTimeMillis()}-${fingerprint.hashCode()}"
        ReplySender.remember(exchangeId, replyAction)

        scope.launch {
            val outcome = agent.handle(
                id = exchangeId,
                channel = channel,
                sender = sender,
                message = body,
                receivedAt = sbn.postTime,
            )
            app.exchanges.add(outcome.exchange)
            outcome.orders.forEach(app.orders::add)

            if (!outcome.safeToAutoSend) return@launch

            val error = ReplySender.send(applicationContext, exchangeId, outcome.exchange.reply)
            app.exchanges.update(exchangeId) { current ->
                if (error == null) {
                    current.copy(state = ReplyState.SENT)
                } else {
                    current.copy(state = ReplyState.FAILED, error = error)
                }
            }
        }
    }

    /** Prefers MessagingStyle, which names the actual person rather than the thread. */
    private fun senderOf(notification: Notification): String? {
        val style = NotificationCompat.MessagingStyle
            .extractMessagingStyleFromNotification(notification)
        val fromStyle = style?.messages?.lastOrNull()?.person?.name?.toString()
        val fromExtras = notification.extras.getCharSequence(Notification.EXTRA_TITLE)?.toString()
        return (fromStyle ?: fromExtras)?.trim()?.takeIf { it.isNotBlank() }
    }

    private fun bodyOf(notification: Notification): String? {
        val style = NotificationCompat.MessagingStyle
            .extractMessagingStyleFromNotification(notification)
        val fromStyle = style?.messages?.lastOrNull()?.text?.toString()
        val fromExtras = notification.extras.getCharSequence(Notification.EXTRA_TEXT)?.toString()
        return (fromStyle ?: fromExtras)?.trim()
    }

    companion object {
        /** True once the user has granted notification access in system settings. */
        fun isGranted(context: Context): Boolean {
            val enabled = Settings.Secure.getString(
                context.contentResolver,
                "enabled_notification_listeners",
            ).orEmpty()
            val component = ComponentName(context, MessageListenerService::class.java)
            return enabled.split(":").any {
                ComponentName.unflattenFromString(it) == component
            }
        }
    }
}
