package vn.bizclaw.app.service

import android.app.Notification
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import vn.bizclaw.app.engine.GlobalLLM
import vn.bizclaw.app.engine.LocalAgentManager
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.engine.ProviderChat

/**
 * BizClaw Notification Listener — catches messages from Zalo, FB, Messenger.
 *
 * When a notification arrives from a monitored app AND an agent has
 * autoReply=true for that app → triggers AI reply via Accessibility Service.
 *
 * Requires user to enable in Settings → Notifications → Notification access
 */
class BizClawNotificationListener : NotificationListenerService() {

    companion object {
        const val TAG = "BizClawNotify"

        // App packages to monitor (including clones/dual apps)
        val MONITORED_APPS = mapOf(
            // Zalo
            "com.zing.zalo" to "Zalo",
            "com.zing.zalo.clone" to "Zalo 2",
            // Facebook
            "com.facebook.katana" to "Facebook",
            "com.facebook.katana.clone" to "Facebook 2",
            "com.facebook.lite" to "Facebook Lite",
            // Messenger
            "com.facebook.orca" to "Messenger",
            "com.facebook.orca.clone" to "Messenger 2",
            "com.facebook.mlite" to "Messenger Lite",
            // Instagram
            "com.instagram.android" to "Instagram",
            "com.instagram.android.clone" to "Instagram 2",
            // Threads
            "com.instagram.barcelona" to "Threads",
            // Gmail & Outlook
            "com.google.android.gm" to "Gmail",
            "com.microsoft.office.outlook" to "Outlook",
            // Lark
            "com.larksuite.suite" to "Lark",
            "com.ss.android.lark" to "Lark CN",
            // Telegram
            "org.telegram.messenger" to "Telegram",
            "org.telegram.messenger.clone" to "Telegram 2",
            "org.telegram.messenger.web" to "Telegram X",
            "org.thunderdog.challegram" to "Telegram X",
        )

        /** Resolve clone/dual package → app name (handles Vivo/Samsung/Xiaomi suffixes) */
        fun resolveAppName(packageName: String): String? {
            MONITORED_APPS[packageName]?.let { return it }
            val basePkg = packageName
                .removeSuffix(".clone").removeSuffix(".dual").removeSuffix("_clone")
            MONITORED_APPS[basePkg]?.let { return "$it (Clone)" }
            return null
        }

        var instance: BizClawNotificationListener? = null
            private set

        // Callback for UI to show received notifications
        var onNotificationReceived: ((SocialNotification) -> Unit)? = null

        // Recent notifications for display
        val recentNotifications = mutableListOf<SocialNotification>()
    }

    data class SocialNotification(
        val app: String,         // "Zalo", "Messenger", etc.
        val packageName: String,
        val sender: String,
        val message: String,
        val timestamp: Long = System.currentTimeMillis(),
        val replied: Boolean = false,
        val replyContent: String = "",
    )

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onListenerConnected() {
        super.onListenerConnected()
        instance = this
        Log.i(TAG, "🔔 NotificationListener connected — monitoring social apps")
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        sbn ?: return

        val pkg = sbn.packageName
        val appName = resolveAppName(pkg) ?: return // Ignore non-monitored apps

        val notification = sbn.notification ?: return
        val extras = notification.extras ?: return

        val title = extras.getCharSequence(Notification.EXTRA_TITLE)?.toString() ?: ""
        val text = extras.getCharSequence(Notification.EXTRA_TEXT)?.toString() ?: ""

        if (text.isBlank()) return

        // Deduplicate: ignore exact same sender + text within 3 seconds
        synchronized(recentNotifications) {
            val isDuplicate = recentNotifications.any {
                it.sender == title && 
                it.message == text && 
                System.currentTimeMillis() - it.timestamp < 3000
            }
            if (isDuplicate) {
                Log.d(TAG, "⏭️ Skipping duplicate notification from $title: $text")
                return
            }
        }

        Log.i(TAG, "📩 $appName — $title: $text")

        val socialNotif = SocialNotification(
            app = appName,
            packageName = pkg,
            sender = title,
            message = text,
        )

        // Store in recent list (cap at 50)
        synchronized(recentNotifications) {
            recentNotifications.add(0, socialNotif)
            if (recentNotifications.size > 50) {
                recentNotifications.removeAt(recentNotifications.lastIndex)
            }
        }

        // Notify UI
        onNotificationReceived?.invoke(socialNotif)

        // ─── MAMA: Check if this is a boss command ───
        val mama = MamaAgent(applicationContext)
        if (mama.isBossCommand(title, pkg)) {
            Log.i(TAG, "👑 MAMA: Boss command detected from $title")
            processMamaCommand(sbn, socialNotif, mama)
            return // Don't auto-reply — Mama handles this
        }

        // Check if any agent should auto-reply
        checkAutoReply(socialNotif)
    }

    /**
     * Process a MAMA boss command — delegate to MamaAgent, reply results.
     */
    private fun processMamaCommand(
        sbn: StatusBarNotification,
        notif: SocialNotification,
        mama: MamaAgent,
    ) {
        scope.launch {
            try {
                // Process the command through Mama
                val report = mama.processCommand(notif.sender, notif.message)

                if (report.isBlank()) {
                    Log.w(TAG, "👑 MAMA: Empty result — skipping reply")
                    return@launch
                }

                Log.i(TAG, "👑 MAMA report: ${report.take(200)}")

                // Update notification record
                synchronized(recentNotifications) {
                    val idx = recentNotifications.indexOfFirst {
                        it.timestamp == notif.timestamp && it.message == notif.message
                    }
                    if (idx >= 0) {
                        recentNotifications[idx] = notif.copy(
                            replied = true,
                            replyContent = "👑 $report",
                        )
                    }
                }

                // ─── Reply back to boss via Zalo ───
                // Method 1: Notification inline reply
                var replied = false
                val activeNotifs = getActiveNotifications()
                val targetNotif = activeNotifs?.find { it.packageName == notif.packageName }

                if (targetNotif != null) {
                    replied = CommandExecutor.replySocialNotification(
                        applicationContext, targetNotif, "👑 $report"
                    )
                }

                // Method 2: Fallback to AppController
                if (!replied) {
                    Log.w(TAG, "👑 Inline reply failed — using AppController")
                    val controller = AppController(applicationContext)
                    val result = controller.zaloSendMessage(notif.sender, "👑 $report")
                    replied = result.success
                }

                if (replied) {
                    Log.i(TAG, "👑 MAMA replied to boss ${notif.sender}")
                } else {
                    Log.w(TAG, "👑 MAMA could not reply — check services")
                }

            } catch (e: Exception) {
                Log.e(TAG, "👑 MAMA command failed: ${e.message?.take(100)}")
            }
        }
    }

    private fun checkAutoReply(notif: SocialNotification) {
        val manager = LocalAgentManager(applicationContext)
        val agents = manager.loadAgents()

        // Find an agent with autoReply=true that handles this app
        val agent = agents.find { it.autoReply && notif.packageName in it.triggerApps }
            ?: return

        Log.i(TAG, "🤖 Auto-reply triggered: ${agent.name} → ${notif.app}")

        val provider = ProviderManager(applicationContext).loadProviders().find { it.id == agent.providerId }
            ?: return
        
        // Make sure ProviderChat can interact if tools are somehow used
        ProviderChat.appContext = applicationContext

        scope.launch {
            try {
                // Build prompt with RAG context
                val fullPrompt = manager.buildPromptForAgent(agent, notif.message)
                val userMsg = "Tin nhắn từ ${notif.sender} trên ${notif.app}: ${notif.message}"

                val replyText = withContext(kotlinx.coroutines.Dispatchers.IO) {
                    ProviderChat.chat(provider, fullPrompt, userMsg).trim()
                }

                if (replyText.isBlank()) {
                    Log.w(TAG, "⚠️ Empty AI response — skipping reply")
                    return@launch
                }
                Log.i(TAG, "✅ Reply generated: ${replyText.take(100)}")

                // Update notification with reply
                synchronized(recentNotifications) {
                    val idx = recentNotifications.indexOfFirst {
                        it.timestamp == notif.timestamp && it.message == notif.message
                    }
                    if (idx >= 0) {
                        recentNotifications[idx] = notif.copy(
                            replied = true,
                            replyContent = replyText,
                        )
                    }
                }

                // ─── SEND THE REPLY ───────────────────────
                // Method 1: Notification inline reply (works for Zalo, Messenger, etc.)
                val activeNotifs = getActiveNotifications()
                val targetNotif = activeNotifs?.find { it.packageName == notif.packageName }
                var replied = false

                if (targetNotif != null) {
                    replied = CommandExecutor.replySocialNotification(
                        applicationContext, targetNotif, replyText
                    )
                }

                // Method 2: Fallback to AccessibilityService
                if (!replied) {
                    Log.w(TAG, "📎 Inline reply failed — trying Accessibility fallback")
                    withContext(kotlinx.coroutines.Dispatchers.Main) {
                        replied = CommandExecutor.replyViaAccessibility(
                            notif.packageName, replyText
                        )
                    }
                }

                if (replied) {
                    Log.i(TAG, "✅ Auto-reply SENT to ${notif.app}: ${replyText.take(60)}")
                } else {
                    Log.w(TAG, "⚠️ Could not send reply — user needs to enable services")
                }

            } catch (e: Exception) {
                Log.e(TAG, "❌ Auto-reply failed: ${e.message?.take(100)}")
            }
        }
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification?) {
        // Not needed
    }

    override fun onListenerDisconnected() {
        super.onListenerDisconnected()
        instance = null
        Log.i(TAG, "🔔 NotificationListener disconnected")
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
        instance = null
    }
}
