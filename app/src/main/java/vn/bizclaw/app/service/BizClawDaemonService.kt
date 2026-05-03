package vn.bizclaw.app.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import vn.bizclaw.app.MainActivity


/**
 * BizClaw Daemon Service — runs the Rust engine as an Android foreground service.
 *
 * This is the heart of BizClaw Android:
 * - START_STICKY: auto-restart if killed by system
 * - Foreground notification: prevents OS from killing the process
 * - WakeLock: keeps CPU running for scheduled tasks
 * - Calls bizclaw-ffi native functions
 *
 * Lifecycle:
 * 1. User taps "Start" → startForegroundService()
 * 2. Service calls native start_daemon() via JNI/UniFFI
 * 3. Rust Tokio runtime spawns on dedicated threads
 * 4. Agents run 24/7, execute Hands on schedule
 * 5. User taps "Stop" → calls native stop_daemon()
 */
class BizClawDaemonService : Service() {

    companion object {
        const val CHANNEL_ID = "bizclaw_daemon"
        const val NOTIFICATION_ID = 1
        const val ACTION_START = "vn.bizclaw.START_DAEMON"
        const val ACTION_STOP = "vn.bizclaw.STOP_DAEMON"

        private var isRunning = false

        fun isRunning(): Boolean = isRunning

        fun start(context: Context) {
            val intent = Intent(context, BizClawDaemonService::class.java).apply {
                action = ACTION_START
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, BizClawDaemonService::class.java).apply {
                action = ACTION_STOP
            }
            context.startService(intent)
        }
    }

    private var wakeLock: PowerManager.WakeLock? = null
    private var commandReceiver: CommandReceiver? = null
    private var jobScheduler: AutomationJobScheduler? = null
    private var platformSync: PlatformSync? = null

    private fun getCommandServerUrl(): String {
        val prefs = getSharedPreferences("bizclaw", MODE_PRIVATE)
        return prefs.getString("command_server_url", null)
            ?: "wss://bizclaw.vn/ws/device" // Default
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> startDaemon()
            ACTION_STOP -> stopDaemon()
        }
        return START_STICKY // Auto-restart if killed
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun startDaemon() {
        if (isRunning) return

        // Start as foreground service
        startForeground(NOTIFICATION_ID, buildNotification("Đang khởi động..."))

        // Acquire WakeLock for background execution
        val pm = getSystemService(POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "BizClaw::DaemonWakeLock"
        ).apply {
            acquire(10 * 60 * 1000L) // 10 min max, renewed by engine
        }

        // Start WebSocket command receiver
        try {
            val wsUrl = getCommandServerUrl()
            commandReceiver = CommandReceiver(this, wsUrl)
            commandReceiver?.connect()
            android.util.Log.i("BizClaw", "📡 Command receiver started: $wsUrl")
        } catch (e: Exception) {
            android.util.Log.e("BizClaw", "⚠️ Command receiver failed: ${e.message}")
        }

        // Start automation job scheduler
        jobScheduler = AutomationJobScheduler(this)
        jobScheduler?.start()
        android.util.Log.i("BizClaw", "⏰ Job scheduler started")

        // Start Platform Sync (agents, knowledge, server-pushed tasks)
        try {
            platformSync = PlatformSync(this)
            platformSync?.start()
            android.util.Log.i("BizClaw", "🔗 Platform Sync started")
        } catch (e: Exception) {
            android.util.Log.e("BizClaw", "⚠️ Platform Sync failed: ${e.message}")
        }

        isRunning = true
        updateNotification("Đang chạy — sẵn sàng nhận lệnh")

        android.util.Log.i("BizClaw", "🤖 Daemon started")
    }

    private fun stopDaemon() {
        // Stop Platform Sync
        platformSync?.stop()
        platformSync = null

        // Stop automation scheduler
        jobScheduler?.stop()
        jobScheduler = null

        // Disconnect WebSocket
        commandReceiver?.disconnect()
        commandReceiver = null

        isRunning = false
        wakeLock?.release()
        wakeLock = null

        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()

        android.util.Log.i("BizClaw", "🛑 Daemon stopped")
    }

    fun updateNotification(text: String) {
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, buildNotification(text))
    }

    private fun buildNotification(text: String): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val stopIntent = PendingIntent.getService(
            this, 1,
            Intent(this, BizClawDaemonService::class.java).apply {
                action = ACTION_STOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("BizClaw Agent")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setContentIntent(pendingIntent)
            .addAction(android.R.drawable.ic_media_pause, "Dừng", stopIntent)
            .setOngoing(true)
            .setSilent(true)
            .build()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "BizClaw Daemon",
                NotificationManager.IMPORTANCE_LOW, // Silent, no sound
            ).apply {
                description = "BizClaw AI agent chạy nền"
                setShowBadge(false)
            }
            val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
            nm.createNotificationChannel(channel)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        wakeLock?.release()
        isRunning = false
    }

    override fun onTaskRemoved(rootIntent: Intent?) {
        // Auto-restart when swiped from recents
        super.onTaskRemoved(rootIntent)
        if (isRunning) {
            start(this)
        }
    }
}
