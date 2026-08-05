package vn.bizclaw.agent.agent

import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.MainActivity
import vn.bizclaw.agent.SERVICE_CHANNEL_ID
import java.util.concurrent.TimeUnit

private const val WORK_NAME = "bizclaw_follow_up"
private const val NOTIFICATION_ID = 2001

/**
 * Daily sweep for orders due a post-sale check-in.
 *
 * It notifies the owner rather than messaging the customer: replying through a
 * notification's reply action only works while that notification is live, so the app
 * cannot open a conversation on its own. The draft is prepared, the sending is manual.
 */
class FollowUpWorker(
    context: Context,
    params: WorkerParameters,
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        val app = BizClawApp.from(applicationContext)
        val due = app.orders.dueFollowUps()
        if (due.isEmpty()) return Result.success()

        val text = if (due.size == 1) {
            val order = due.first()
            "Đến hạn hỏi thăm ${order.customerName} về ${order.productName}"
        } else {
            "${due.size} khách đến hạn hỏi thăm sau mua"
        }
        notify(text)
        return Result.success()
    }

    private fun notify(text: String) {
        val manager = applicationContext.getSystemService(NotificationManager::class.java)
            ?: return
        val open = PendingIntent.getActivity(
            applicationContext,
            1,
            Intent(applicationContext, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE,
        )
        manager.notify(
            NOTIFICATION_ID,
            Notification.Builder(applicationContext, SERVICE_CHANNEL_ID)
                .setContentTitle("Chăm sóc sau bán")
                .setContentText(text)
                .setSmallIcon(android.R.drawable.ic_dialog_email)
                .setContentIntent(open)
                .setAutoCancel(true)
                .build(),
        )
    }

    companion object {
        /** Default gap between an order being confirmed and the check-in nudge. */
        val DEFAULT_DELAY_MS: Long = TimeUnit.DAYS.toMillis(3)

        fun schedule(context: Context) {
            val request = PeriodicWorkRequestBuilder<FollowUpWorker>(1, TimeUnit.DAYS).build()
            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                request,
            )
        }
    }
}
