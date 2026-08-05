package vn.bizclaw.agent

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import vn.bizclaw.agent.data.ExchangeStore
import vn.bizclaw.agent.data.KnowledgeStore
import vn.bizclaw.agent.data.Settings
import vn.bizclaw.agent.llm.ModelDownloader

const val SERVICE_CHANNEL_ID = "bizclaw_agent"

/**
 * Single source of truth for app-wide state.
 *
 * The notification listener runs in the same process but outside any Activity, so these
 * stores hang off Application rather than a ViewModel.
 */
class BizClawApp : Application() {

    lateinit var settings: Settings
        private set
    lateinit var exchanges: ExchangeStore
        private set
    lateinit var knowledge: KnowledgeStore
        private set
    lateinit var downloader: ModelDownloader
        private set

    override fun onCreate() {
        super.onCreate()
        settings = Settings(this)
        exchanges = ExchangeStore(this)
        knowledge = KnowledgeStore(this)
        downloader = ModelDownloader(this)
        registerServiceChannel()
    }

    private fun registerServiceChannel() {
        val manager = getSystemService(NotificationManager::class.java) ?: return
        manager.createNotificationChannel(
            NotificationChannel(
                SERVICE_CHANNEL_ID,
                "Trạng thái agent",
                // LOW: this notification is a foreground-service requirement, not news.
                NotificationManager.IMPORTANCE_LOW,
            ),
        )
    }

    companion object {
        fun from(context: Context): BizClawApp =
            context.applicationContext as BizClawApp
    }
}
