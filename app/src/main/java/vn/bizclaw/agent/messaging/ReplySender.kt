package vn.bizclaw.agent.messaging

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.core.app.RemoteInput

private const val TAG = "ReplySender"
private const val MAX_PENDING = 50

/**
 * Delivers text back into Zalo/Messenger by firing the notification's own reply action.
 *
 * This is the same mechanism a smartwatch uses, so it works without Accessibility, with
 * the screen off, and without driving anyone's UI. The catch is lifetime: the
 * [PendingIntent] is only valid while the notification is live, which is why handles are
 * held in memory and dropped once used.
 */
object ReplySender {

    private class Handle(val action: Notification.Action, val createdAt: Long)

    private val handles = LinkedHashMap<String, Handle>()

    /** Remembers the reply action belonging to [exchangeId]. */
    @Synchronized
    fun remember(exchangeId: String, action: Notification.Action) {
        handles[exchangeId] = Handle(action, System.currentTimeMillis())
        while (handles.size > MAX_PENDING) {
            handles.remove(handles.keys.first())
        }
    }

    @Synchronized
    fun canSend(exchangeId: String): Boolean = handles.containsKey(exchangeId)

    @Synchronized
    fun forget(exchangeId: String) {
        handles.remove(exchangeId)
    }

    /** Finds the first action on [notification] that accepts free text. */
    fun replyActionOf(notification: Notification): Notification.Action? =
        notification.actions?.firstOrNull { action ->
            action.remoteInputs?.any { it.allowFreeFormInput } == true
        }

    /**
     * Fires the stored reply action with [text].
     *
     * @return null on success, or a human-readable reason on failure.
     */
    @Synchronized
    fun send(context: Context, exchangeId: String, text: String): String? {
        val handle = handles[exchangeId] ?: return "Thông báo đã bị đóng, không gửi lại được"
        val action = handle.action
        val inputs = action.remoteInputs?.filter { it.allowFreeFormInput }
        if (inputs.isNullOrEmpty()) return "Thông báo không có ô trả lời nhanh"

        return runCatching {
            val intent = Intent().addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            val results = Bundle().apply {
                inputs.forEach { putCharSequence(it.resultKey, text) }
            }
            // Convert to the AndroidX wrapper so addResultsToIntent writes the payload in
            // the format the receiving app expects.
            val compat = inputs.map { input ->
                RemoteInput.Builder(input.resultKey)
                    .setLabel(input.label)
                    .setAllowFreeFormInput(true)
                    .build()
            }.toTypedArray()
            RemoteInput.addResultsToIntent(compat, intent, results)
            RemoteInput.setResultsSource(intent, RemoteInput.SOURCE_FREE_FORM_INPUT)

            action.actionIntent.send(context, 0, intent)
            handles.remove(exchangeId)
            null
        }.getOrElse { error ->
            Log.e(TAG, "Reply failed for $exchangeId", error)
            handles.remove(exchangeId)
            when (error) {
                is PendingIntent.CanceledException -> "Thông báo đã hết hiệu lực"
                else -> error.message ?: "Không gửi được trả lời"
            }
        }
    }
}
