package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import vn.bizclaw.app.engine.*
import java.io.File

/**
 * AutomationJob — Agent-driven scheduled automation.
 *
 * Each job:
 * 1. Collects data from sources (Zalo groups, FB comments, Gmail inbox)
 * 2. Uses an Agent to analyze/summarize (with system prompt + provider)
 * 3. Delivers the report via chosen channel (Zalo, email, etc.)
 * 4. Runs on schedule (e.g., every 4 hours)
 *
 * Example: "Tổng hợp tin nhắn Zalo nhóm 'Khách hàng' mỗi 4 tiếng,
 *           AI phân tích → gửi báo cáo qua Zalo cho số 0901234567"
 */

private val jobJson = Json {
    ignoreUnknownKeys = true
    prettyPrint = true
    encodeDefaults = true
}

@Serializable
data class AutomationJob(
    val id: String,
    val name: String,
    val emoji: String = "🤖",
    val enabled: Boolean = true,

    // ─── Agent Configuration ───
    /** Agent ID to use for analysis/summarization */
    val agentId: String,

    // ─── Data Sources ───
    /** What data to collect */
    val sources: List<DataSource> = emptyList(),

    // ─── Schedule ───
    /** Interval in minutes (e.g., 240 = every 4 hours) */
    val intervalMinutes: Int = 240,
    /** Specific times to run (e.g., ["09:00", "13:00", "17:00", "21:00"]) */
    val scheduleTimes: List<String> = emptyList(),

    // ─── Delivery ───
    /** How to deliver the report */
    val delivery: DeliveryConfig = DeliveryConfig(),

    // ─── State ───
    val createdAt: Long = System.currentTimeMillis(),
    val lastRunAt: Long = 0,
    val runCount: Int = 0,
)

@Serializable
data class DataSource(
    val type: SourceType,
    /** For Zalo: group name. For Gmail: search query. For FB: page/post hint. */
    val target: String = "",
    /** Max items to collect */
    val maxItems: Int = 20,
)

@Serializable
enum class SourceType {
    /** Collect Zalo messages from a specific group/contact */
    ZALO_MESSAGES,
    /** Collect Facebook comments */
    FACEBOOK_COMMENTS,
    /** Collect Gmail inbox emails */
    GMAIL_INBOX,
    /** Collect Lark chat messages */
    LARK_CHATS,
    /** Collect Lark Mail */
    LARK_MAIL,
    /** Collect Telegram messages */
    TELEGRAM_MESSAGES,
    /** Collect all recent notifications */
    NOTIFICATIONS,
    /** Read current screen content */
    SCREEN_CONTENT,
}

@Serializable
data class DeliveryConfig(
    val method: DeliveryMethod = DeliveryMethod.ZALO,
    /** For Zalo: contact name or phone number */
    val target: String = "",
    /** Whether to ask user confirmation before sending */
    val confirmBeforeSend: Boolean = true,
)

@Serializable
enum class DeliveryMethod {
    ZALO,
    GMAIL,
    LARK,
    TELEGRAM,
    LOG_ONLY,
}

// ═══════════════════════════════════════════════════════
// Job Manager — CRUD + execution
// ═══════════════════════════════════════════════════════

class AutomationJobManager(private val context: Context) {
    companion object {
        private const val TAG = "AutoJob"
    }

    private val jobsFile = File(context.filesDir, "automation_jobs.json")
    private val dataDir = File(context.filesDir, "collected_data").also { it.mkdirs() }

    fun loadJobs(): List<AutomationJob> {
        if (!jobsFile.exists()) return emptyList()
        return try {
            jobJson.decodeFromString<List<AutomationJob>>(jobsFile.readText())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load jobs: ${e.message}")
            emptyList()
        }
    }

    fun saveJobs(jobs: List<AutomationJob>) {
        jobsFile.writeText(jobJson.encodeToString(jobs))
    }

    fun addJob(job: AutomationJob) {
        val jobs = loadJobs().toMutableList()
        jobs.add(job)
        saveJobs(jobs)
    }

    fun updateJob(job: AutomationJob) {
        val jobs = loadJobs().toMutableList()
        val idx = jobs.indexOfFirst { it.id == job.id }
        if (idx >= 0) {
            jobs[idx] = job
            saveJobs(jobs)
        }
    }

    fun deleteJob(jobId: String) {
        val jobs = loadJobs().toMutableList()
        jobs.removeAll { it.id == jobId }
        saveJobs(jobs)
    }

    fun getJob(jobId: String): AutomationJob? = loadJobs().find { it.id == jobId }

    // ─── Execution ─────────────────────────────

    /**
     * Execute a job: collect → analyze → deliver.
     *
     * Returns the generated report text.
     */
    suspend fun executeJob(job: AutomationJob): String {
        Log.i(TAG, "🚀 Executing job: ${job.emoji} ${job.name}")

        // 1. Resolve agent
        val agentManager = LocalAgentManager(context)
        val agent = agentManager.getAgent(job.agentId)
            ?: return "❌ Agent không tồn tại: ${job.agentId}"

        // 2. Collect data from all sources
        val collectedData = mutableListOf<String>()
        for (source in job.sources) {
            val data = collectDataFromSource(source)
            if (data.isNotBlank()) {
                collectedData.add("── ${source.type.name} (${source.target}) ──\n$data")
            }
        }

        if (collectedData.isEmpty()) {
            Log.w(TAG, "No data collected for job ${job.name}")
            return "⚠️ Không có dữ liệu để tổng hợp."
        }

        val rawData = collectedData.joinToString("\n\n")
        Log.i(TAG, "📥 Collected ${collectedData.size} sources, ${rawData.length} chars")

        // Save raw data for reference
        saveCollectedData(job.id, rawData)

        // 3. Use agent to analyze/summarize
        val report = analyzeWithAgent(agent, rawData, job.name)

        // 4. Deliver report
        if (report.isNotBlank()) {
            deliverReport(job, report)
        }

        // 5. Update job state
        updateJob(job.copy(
            lastRunAt = System.currentTimeMillis(),
            runCount = job.runCount + 1,
        ))

        return report
    }

    /**
     * Collect data from a source using AccessibilityService + notifications.
     */
    private suspend fun collectDataFromSource(source: DataSource): String {
        return when (source.type) {
            SourceType.NOTIFICATIONS -> {
                // Get recent notifications, optionally filtered by app
                val notifications = BizClawNotificationListener.recentNotifications
                    .filter { notif ->
                        source.target.isBlank() || notif.app.contains(source.target, ignoreCase = true)
                    }
                    .take(source.maxItems)

                if (notifications.isEmpty()) return ""
                notifications.joinToString("\n") { notif ->
                    "[${notif.app}] ${notif.sender}: ${notif.message}"
                }
            }

            SourceType.ZALO_MESSAGES -> {
                // Collect Zalo messages from notifications
                val zaloNotifs = BizClawNotificationListener.recentNotifications
                    .filter { it.packageName == "com.zing.zalo" }
                    .filter { notif ->
                        source.target.isBlank() || notif.sender.contains(source.target, ignoreCase = true)
                    }
                    .take(source.maxItems)

                if (zaloNotifs.isEmpty()) {
                    return ""
                }

                zaloNotifs.joinToString("\n") { "[${it.sender}] ${it.message}" }
            }

            SourceType.FACEBOOK_COMMENTS -> {
                // Collect Facebook notifications (comments)
                val fbNotifs = BizClawNotificationListener.recentNotifications
                    .filter {
                        it.packageName == "com.facebook.katana" ||
                        it.packageName == "com.facebook.orca"
                    }
                    .take(source.maxItems)

                if (fbNotifs.isEmpty()) return ""
                fbNotifs.joinToString("\n") { "[${it.app}] ${it.sender}: ${it.message}" }
            }

            SourceType.GMAIL_INBOX -> {
                "" // Unsupported without A11y UI scraping
            }

            SourceType.LARK_CHATS -> {
                // Collect Lark notifications
                val larkNotifs = BizClawNotificationListener.recentNotifications
                    .filter {
                        it.packageName == "com.larksuite.suite" ||
                        it.packageName == "com.ss.android.lark"
                    }
                    .filter { notif ->
                        source.target.isBlank() || notif.sender.contains(source.target, ignoreCase = true)
                    }
                    .take(source.maxItems)

                if (larkNotifs.isEmpty()) {
                    return ""
                }
                larkNotifs.joinToString("\n") { "[Lark] ${it.sender}: ${it.message}" }
            }

            SourceType.LARK_MAIL -> {
                "" // Unsupported without A11y UI scraping
            }

            SourceType.TELEGRAM_MESSAGES -> {
                val tgNotifs = BizClawNotificationListener.recentNotifications
                    .filter { it.packageName == "org.telegram.messenger" }
                    .filter { notif ->
                        source.target.isBlank() || notif.sender.contains(source.target, ignoreCase = true)
                    }
                    .take(source.maxItems)

                if (tgNotifs.isEmpty()) {
                    return ""
                }
                tgNotifs.joinToString("\n") { "[Telegram] ${it.sender}: ${it.message}" }
            }

            SourceType.SCREEN_CONTENT -> {
                "" // Unsupported without A11y UI scraping
            }
        }
    }

    /**
     * Use the assigned agent (with its system prompt + provider) to analyze data.
     */
    private suspend fun analyzeWithAgent(
        agent: LocalAgent,
        rawData: String,
        jobName: String,
    ): String {
        val agentManager = LocalAgentManager(context)
        val providerManager = ProviderManager(context)

        // Build the analysis prompt
        val analysisPrompt = """
Bạn đang thực hiện công việc tự động: "$jobName"

Dưới đây là dữ liệu thu thập được. Hãy phân tích và tạo báo cáo tổng hợp ngắn gọn, 
chuyên nghiệp, bằng tiếng Việt. Nêu bật các điểm quan trọng.

═══ DỮ LIỆU ═══
$rawData
═══════════════

Hãy tạo báo cáo tổng hợp.
        """.trimIndent()

        // Get agent's system prompt with RAG context
        val systemPrompt = agentManager.buildPromptForAgent(agent, analysisPrompt)

        // Find the agent's provider
        val providers = providerManager.loadProviders()
        val provider = providers.find { it.id == agent.providerId }
            ?: providers.firstOrNull { it.enabled }
            ?: return "❌ Không tìm thấy provider cho agent ${agent.name}"

        // Call AI
        return try {
            val response = withContext(Dispatchers.IO) {
                ProviderChat.chat(provider, systemPrompt, analysisPrompt)
            }
            Log.i(TAG, "🤖 Agent ${agent.name} generated report: ${response.take(100)}...")
            response
        } catch (e: Exception) {
            Log.e(TAG, "AI analysis failed: ${e.message}")
            "❌ AI phân tích thất bại: ${e.message?.take(100)}"
        }
    }

    /**
     * Deliver the report via configured channel.
     */
    private suspend fun deliverReport(job: AutomationJob, report: String) {
        val delivery = job.delivery
        when (delivery.method) {
            DeliveryMethod.ZALO -> {
                if (delivery.target.isBlank()) {
                    Log.w(TAG, "Zalo delivery target is empty — skipping")
                    return
                }
                val controller = AppController(context)
                val fullReport = "${job.emoji} Báo cáo: ${job.name}\n\n$report"
                val result = controller.zaloSendMessage(delivery.target, fullReport)
                if (result.success) {
                    Log.i(TAG, "✅ Report sent via Zalo to ${delivery.target}")
                } else {
                    Log.e(TAG, "❌ Zalo delivery failed: ${result.message}")
                }
            }

            DeliveryMethod.GMAIL -> {
                if (delivery.target.isBlank()) return
                val controller = AppController(context)
                val subject = "${job.emoji} Báo cáo: ${job.name} — ${
                    java.text.SimpleDateFormat("dd/MM HH:mm", java.util.Locale.getDefault())
                        .format(java.util.Date())
                }"
                val result = controller.gmailCompose(delivery.target, subject, report)
                if (result.success) {
                    Log.i(TAG, "✅ Report sent via Gmail to ${delivery.target}")
                } else {
                    Log.e(TAG, "❌ Gmail delivery failed: ${result.message}")
                }
            }

            DeliveryMethod.LARK -> {
                if (delivery.target.isBlank()) return
                val controller = AppController(context)
                val fullReport = "${job.emoji} ${job.name}\n\n$report"
                val result = controller.larkSendMessage(delivery.target, fullReport)
                if (result.success) {
                    Log.i(TAG, "✅ Report sent via Lark to ${delivery.target}")
                } else {
                    Log.e(TAG, "❌ Lark delivery failed: ${result.message}")
                }
            }

            DeliveryMethod.TELEGRAM -> {
                if (delivery.target.isBlank()) return
                val controller = AppController(context)
                val fullReport = "${job.emoji} ${job.name}\n\n$report"
                val result = controller.telegramSendMessage(delivery.target, fullReport)
                if (result.success) {
                    Log.i(TAG, "✅ Report sent via Telegram to ${delivery.target}")
                } else {
                    Log.e(TAG, "❌ Telegram delivery failed: ${result.message}")
                }
            }

            DeliveryMethod.LOG_ONLY -> {
                Log.i(TAG, "📋 Report (log only):\n$report")
            }
        }
    }

    private fun saveCollectedData(jobId: String, data: String) {
        try {
            val file = File(dataDir, "job_${jobId}_${System.currentTimeMillis()}.txt")
            file.writeText(data)
            // Keep only last 10 data files per job
            dataDir.listFiles()
                ?.filter { it.name.startsWith("job_${jobId}_") }
                ?.sortedByDescending { it.lastModified() }
                ?.drop(10)
                ?.forEach { it.delete() }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save collected data: ${e.message}")
        }
    }
}

// ═══════════════════════════════════════════════════════
// Job Scheduler — runs jobs on their intervals
// ═══════════════════════════════════════════════════════

class AutomationJobScheduler(private val context: Context) {
    companion object {
        private const val TAG = "AutoJobSched"
        var instance: AutomationJobScheduler? = null
            private set
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var schedulerJob: Job? = null

    /** Start the scheduler — checks jobs every minute. */
    fun start() {
        if (schedulerJob?.isActive == true) return
        instance = this

        schedulerJob = scope.launch {
            Log.i(TAG, "⏰ Job scheduler started")
            while (isActive) {
                try {
                    checkAndRunDueJobs()
                } catch (e: Exception) {
                    Log.e(TAG, "Scheduler tick failed: ${e.message}")
                }
                delay(60_000) // Check every minute
            }
        }
    }

    fun stop() {
        schedulerJob?.cancel()
        schedulerJob = null
        instance = null
        Log.i(TAG, "⏰ Job scheduler stopped")
    }

    private suspend fun checkAndRunDueJobs() {
        val manager = AutomationJobManager(context)
        val jobs = manager.loadJobs()
        val now = System.currentTimeMillis()

        for (job in jobs) {
            if (!job.enabled) continue

            val intervalMs = job.intervalMinutes * 60 * 1000L
            val timeSinceLastRun = now - job.lastRunAt

            // Check if scheduled time has arrived
            val shouldRun = if (job.scheduleTimes.isNotEmpty()) {
                // Time-based schedule
                isScheduledTimeNow(job.scheduleTimes) && timeSinceLastRun > 55_000
            } else {
                // Interval-based schedule
                timeSinceLastRun >= intervalMs
            }

            if (shouldRun) {
                Log.i(TAG, "⏰ Job due: ${job.emoji} ${job.name}")
                try {
                    manager.executeJob(job)
                } catch (e: Exception) {
                    Log.e(TAG, "Job execution failed: ${job.name} — ${e.message}")
                }
            }
        }
    }

    private fun isScheduledTimeNow(times: List<String>): Boolean {
        val sdf = java.text.SimpleDateFormat("HH:mm", java.util.Locale.getDefault())
        val currentTime = sdf.format(java.util.Date())
        return times.any { it == currentTime }
    }
}
