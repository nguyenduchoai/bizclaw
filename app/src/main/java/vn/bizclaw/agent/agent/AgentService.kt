package vn.bizclaw.agent.agent

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.IBinder
import androidx.lifecycle.LifecycleService
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.MainActivity
import vn.bizclaw.agent.SERVICE_CHANNEL_ID
import vn.bizclaw.agent.llm.GemmaEngine

private const val NOTIFICATION_ID = 1001

/**
 * Keeps the loaded Gemma engine resident while the agent is on.
 *
 * Without a foreground service the process gets trimmed between messages and every
 * reply would pay the multi-second model load again.
 */
class AgentService : LifecycleService() {

    override fun onBind(intent: Intent): IBinder? {
        super.onBind(intent)
        return null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)
        val app = BizClawApp.from(applicationContext)

        startForeground(NOTIFICATION_ID, buildNotification("Đang nạp model..."))

        lifecycleScope.launch {
            val ok = GemmaEngine.ensureLoaded(applicationContext, app.settings.modelVariant)
            val text = if (ok) {
                "Sẵn sàng · ${app.settings.modelVariant.label} · ${GemmaEngine.activeBackend}"
            } else {
                GemmaEngine.lastError ?: "Không nạp được model"
            }
            notify(buildNotification(text))
            if (!ok) stopSelf()
        }

        return START_STICKY
    }

    override fun onDestroy() {
        lifecycleScope.launch { GemmaEngine.unload() }
        super.onDestroy()
    }

    private fun notify(notification: Notification) {
        val manager = getSystemService(android.app.NotificationManager::class.java) ?: return
        manager.notify(NOTIFICATION_ID, notification)
    }

    private fun buildNotification(text: String): Notification {
        val open = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE,
        )
        return Notification.Builder(this, SERVICE_CHANNEL_ID)
            .setContentTitle("BizClaw Agent")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_dialog_email)
            .setContentIntent(open)
            .setOngoing(true)
            .build()
    }

    companion object {
        fun start(context: Context) {
            context.startForegroundService(Intent(context, AgentService::class.java))
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, AgentService::class.java))
        }
    }
}
