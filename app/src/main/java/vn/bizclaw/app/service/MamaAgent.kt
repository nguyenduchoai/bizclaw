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
 * MamaAgent — Tổng quản nhận lệnh từ Zalo, phân việc cho agents.
 *
 * ╔═══════════════════════════════════════════════════════════╗
 * ║  Zalo "Boss" gửi: "Tổng hợp email hôm nay rồi gửi      ║
 * ║                    báo cáo về nhóm Khách hàng"            ║
 * ║                          ↓                                ║
 * ║  NotificationListener → Mama detects "boss" contact       ║
 * ║                          ↓                                ║
 * ║  Mama Agent (AI) phân tích → hiểu cần:                    ║
 * ║    1. Đọc Gmail → delegate cho Agent "Thư ký"             ║
 * ║    2. Tổng hợp → delegate cho Agent "Content Writer"      ║
 * ║    3. Gửi Zalo "Khách hàng" → AppController               ║
 * ║                          ↓                                ║
 * ║  Kết quả → reply lại Zalo "Boss"                          ║
 * ╚═══════════════════════════════════════════════════════════╝
 *
 * Config saved in mama_config.json:
 * - bossContacts: ["Hoài", "0901234567"] — ai gửi lệnh
 * - mamaAgentId: agent dùng để phân tích lệnh
 * - resultDelivery: reply lại Zalo boss
 */

private val mamaJson = Json {
    ignoreUnknownKeys = true
    prettyPrint = true
    encodeDefaults = true
}

@Serializable
data class MamaConfig(
    /** Contacts/names in Zalo whose messages are treated as COMMANDS */
    val bossContacts: List<String> = emptyList(),
    /** The agent that acts as orchestrator (analyzes commands, delegates) */
    val mamaAgentId: String = "",
    /** Whether Mama is active */
    val enabled: Boolean = false,
    /** Reply results back to boss via Zalo */
    val replyToBoss: Boolean = true,
    /** Log all commands and results */
    val logCommands: Boolean = true,
)

@Serializable
data class MamaCommandLog(
    val timestamp: Long = System.currentTimeMillis(),
    val from: String,
    val command: String,
    val intent: String = "",
    val delegatedTo: List<String> = emptyList(),
    val result: String = "",
    val success: Boolean = true,
)

class MamaAgent(private val context: Context) {
    companion object {
        private const val TAG = "MamaAgent"
        private const val CONFIG_FILE = "mama_config.json"
        private const val LOG_FILE = "mama_command_log.json"

        /** Available actions the Mama agent can delegate */
        val AVAILABLE_ACTIONS = listOf(
            "gmail_read", "gmail_compose", "gmail_search", "gmail_archive", "gmail_label",
            "facebook_post", "facebook_comment",
            "messenger_reply", "messenger_read",
            "zalo_send",
            "zalo_post", "zalo_timeline",
            "instagram_post",
            "threads_post", "threads_read",
            "lark_read_chats", "lark_send", "lark_read_mail", "lark_compose_mail",
            "telegram_read", "telegram_send",
            "read_screen",
            "schedule_list", "schedule_create", "schedule_run",
        )
    }

    // ─── Config ─────────────────────────────

    fun loadConfig(): MamaConfig {
        val file = File(context.filesDir, CONFIG_FILE)
        if (!file.exists()) return MamaConfig()
        return try {
            mamaJson.decodeFromString<MamaConfig>(file.readText())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load Mama config: ${e.message}")
            MamaConfig()
        }
    }

    fun saveConfig(config: MamaConfig) {
        File(context.filesDir, CONFIG_FILE).writeText(mamaJson.encodeToString(config))
        Log.i(TAG, "💾 Mama config saved: ${config.bossContacts}")
    }

    // ─── Command Detection ─────────────────

    /**
     * Check if a notification is a command from a boss contact.
     * Returns true if this message should be handled by Mama.
     */
    fun isBossCommand(sender: String, packageName: String): Boolean {
        if (packageName != "com.zing.zalo") return false // Only Zalo for now
        val config = loadConfig()
        if (!config.enabled) return false
        if (config.bossContacts.isEmpty()) return false
        if (config.mamaAgentId.isBlank()) return false

        return config.bossContacts.any { boss ->
            sender.contains(boss, ignoreCase = true)
        }
    }

    // ─── Command Processing ─────────────────

    /**
     * Process a command from the boss.
     * 1. Use Mama agent to understand the command
     * 2. Execute the identified actions
     * 3. Report results back via Zalo
     */
    suspend fun processCommand(
        sender: String,
        message: String,
    ): String {
        Log.i(TAG, "👑 Boss command from $sender: $message")
        val config = loadConfig()

        // 1. Load the Mama agent
        val agentManager = LocalAgentManager(context)
        val mamaAgent = agentManager.getAgent(config.mamaAgentId)
            ?: return "❌ Mama agent chưa được cài đặt (ID: ${config.mamaAgentId})"

        // 2. Load available agents for delegation
        val allAgents = agentManager.loadAgents()
        val agentList = allAgents.joinToString("\n") { ag ->
            "- ${ag.emoji} ${ag.name} (ID: ${ag.id}) — ${ag.role}"
        }

        // 3. Use Mama agent to analyze the command
        val analysisPrompt = buildAnalysisPrompt(message, sender, agentList)
        val mamaSystemPrompt = agentManager.buildPromptForAgent(mamaAgent, message)

        // 4. Get AI analysis: what to do, which agents to use
        val providerManager = ProviderManager(context)
        val providers = providerManager.loadProviders()
        val provider = providers.find { it.id == mamaAgent.providerId }
            ?: providers.firstOrNull { it.enabled }
            ?: return "❌ Không có provider nào khả dụng"

        // Ensure ProviderChat has context to open apps if needed
        ProviderChat.appContext = context

        val analysis = try {
            withContext(Dispatchers.IO) {
                ProviderChat.chat(provider, mamaSystemPrompt, analysisPrompt)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Mama analysis failed: ${e.message}")
            return "❌ Mama không phân tích được lệnh: ${e.message?.take(100)}"
        }

        Log.i(TAG, "🧠 Mama analysis: ${analysis.take(200)}")

        // 5. Parse the analysis and execute
        val executionResult = executeAnalysis(analysis, message, allAgents)

        // 6. Generate final report
        val finalReport = generateReport(mamaAgent, provider, message, executionResult)

        // 7. Log the command
        logCommand(MamaCommandLog(
            from = sender,
            command = message,
            intent = analysis.take(200),
            result = finalReport.take(500),
            success = !finalReport.startsWith("❌"),
        ))

        return finalReport
    }

    private fun buildAnalysisPrompt(command: String, sender: String, agentList: String): String {
        return """
Bạn là MAMA — tổng quản AI. Sếp "$sender" vừa gửi lệnh qua Zalo:

"$command"

📋 Các Agent hiện có:
$agentList

🔧 Các hành động có thể thực hiện:
- gmail_compose: Gửi email (cần to, subject, body)
- facebook_post: Đăng bài Facebook (cần content)
- facebook_comment: Comment Facebook (cần comment)
- messenger_reply: Trả lời Messenger (cần contact, message)
- zalo_send: Gửi Zalo (cần contact, message)
- zalo_post: Đăng bài Nhật ký Zalo (cần content)
- instagram_post: Đăng Instagram (cần caption)
- threads_post: Đăng Threads (cần content)
- lark_send: Gửi tin nhắn Lark (cần contact, message)
- lark_compose_mail: Gửi mail Lark (cần to, subject, body)
- telegram_send: Gửi Telegram (cần contact, message)
- schedule_create: Tạo lịch tự động
- delegate_task: Giao việc cho Agent khác suy nghĩ, tư vấn, viết lách (cần agent_id, task)

Hãy phân tích lệnh và trả lời CHÍNH XÁC theo format:
INTENT: [mô tả ngắn gọn sếp muốn gì]
ACTIONS: [action1|param1=value1|param2=value2], [action2|...]
AGENT: [agent_id để delegate, hoặc "mama" nếu tự làm]
REPORT: [yes/no — có cần báo cáo lại không]

Ví dụ:
INTENT: Tổng hợp email và gửi báo cáo qua Zalo
ACTIONS: [zalo_send|contact=Khách hàng|message=Chương trình khuyến mãi giảm 50% bắt đầu vào ngày mai]
AGENT: secretary
REPORT: yes
        """.trimIndent()
    }

    /**
     * Parse AI analysis and execute the identified actions.
     */
    private suspend fun executeAnalysis(
        analysis: String,
        originalCommand: String,
        allAgents: List<LocalAgent>,
    ): String {
        val results = mutableListOf<String>()

        // Extract ACTIONS from analysis
        val actionsLine = analysis.lines()
            .find { it.startsWith("ACTIONS:") }
            ?.substringAfter("ACTIONS:")
            ?.trim()

        if (actionsLine.isNullOrBlank()) {
            // No specific actions parsed — just use the full analysis as context
            return analysis
        }

        // Extract AGENT
        val agentId = analysis.lines()
            .find { it.startsWith("AGENT:") }
            ?.substringAfter("AGENT:")
            ?.trim()

        // Parse actions: [action|param=val|param=val], [action|...]
        val actionPattern = Regex("""\[([^\]]+)\]""")
        val actions = actionPattern.findAll(actionsLine).map { it.groupValues[1] }.toList()

        for (actionStr in actions) {
            val parts = actionStr.split("|").map { it.trim() }
            val action = parts[0]
            val params = mutableMapOf<String, String>()
            for (i in 1 until parts.size) {
                val kv = parts[i].split("=", limit = 2)
                if (kv.size == 2) params[kv[0].trim()] = kv[1].trim()
            }

            Log.i(TAG, "⚡ Executing: $action with params: $params")

            try {
                val result = executeAction(action, params, agentId, allAgents)
                results.add("✅ $action: $result")
            } catch (e: Exception) {
                results.add("❌ $action: ${e.message?.take(100)}")
            }
        }

        return results.joinToString("\n")
    }

    /**
     * Execute a single action (delegates to AppController or automation).
     */
    private suspend fun executeAction(
        action: String,
        params: MutableMap<String, String>,
        delegateAgentId: String?,
        allAgents: List<LocalAgent>,
    ): String {
        val controller = AppController(context)

        // If a delegate agent is specified and the action needs AI processing,
        // use that agent's provider and prompt
        if (delegateAgentId != null && delegateAgentId != "mama") {
            val agentManager = LocalAgentManager(context)
            val agent = allAgents.find { it.id == delegateAgentId }
            if (agent != null) {
                Log.i(TAG, "📎 Delegating to agent: ${agent.emoji} ${agent.name}")
            }
        }

        return when (action) {
            // Gmail
            "gmail_compose" -> {
                val to = params["to"] ?: return "Missing 'to'"
                val subject = params["subject"] ?: "Từ BizClaw"
                val body = params["body"] ?: params["message"] ?: ""
                val result = controller.gmailCompose(to, subject, body)
                result.message
            }

            // Facebook
            "facebook_post" -> {
                val content = params["content"] ?: return "Missing 'content'"
                controller.facebookPost(content).message
            }
            "facebook_comment" -> {
                val comment = params["comment"] ?: return "Missing 'comment'"
                controller.facebookComment(comment).message
            }

            // Messenger
            "messenger_reply" -> {
                val contact = params["contact"] ?: return "Missing 'contact'"
                val message = params["message"] ?: return "Missing 'message'"
                controller.messengerReply(contact, message).message
            }

            // Zalo
            "zalo_send" -> {
                var message = params["message"] ?: ""
                val contact = params["contact"] ?: return "Missing 'contact'"

                // If message is "REPORT", replace with the collected results so far
                if (message == "REPORT" || message.isBlank()) {
                    message = "Mama đang thực hiện — sẽ gửi kết quả sau..."
                }

                controller.zaloSendMessage(contact, message).message
            }
            "zalo_post" -> {
                val content = params["content"] ?: return "Missing 'content'"
                controller.zaloPost(content).message
            }

            // Instagram
            "instagram_post" -> {
                val caption = params["caption"] ?: return "Missing 'caption'"
                controller.instagramCaption(caption).message
            }

            // Threads
            "threads_post" -> {
                val content = params["content"] ?: return "Missing 'content'"
                controller.threadsPost(content).message
            }

            // Schedule
            "schedule_create" -> {
                val jobManager = AutomationJobManager(context)
                val job = AutomationJob(
                    id = java.util.UUID.randomUUID().toString().take(8),
                    name = params["name"] ?: "Mama Job",
                    agentId = params["agent_id"] ?: delegateAgentId ?: "",
                    sources = listOf(DataSource(type = SourceType.NOTIFICATIONS)),
                    intervalMinutes = params["interval"]?.toIntOrNull() ?: 240,
                    delivery = DeliveryConfig(
                        method = DeliveryMethod.ZALO,
                        target = params["delivery_target"] ?: "",
                    ),
                )
                jobManager.addJob(job)
                "✅ Đã tạo lịch '${job.name}' — ${job.intervalMinutes} phút/lần"
            }

            // Lark
            "lark_send" -> {
                val contact = params["contact"] ?: return "Missing 'contact'"
                val message = params["message"] ?: return "Missing 'message'"
                controller.larkSendMessage(contact, message).message
            }
            "lark_compose_mail" -> {
                val to = params["to"] ?: return "Missing 'to'"
                val subject = params["subject"] ?: "Từ BizClaw"
                val body = params["body"] ?: ""
                controller.larkComposeMail(to, subject, body).message
            }

            // Telegram
            "telegram_send" -> {
                val contact = params["contact"] ?: return "Missing 'contact'"
                val message = params["message"] ?: return "Missing 'message'"
                controller.telegramSendMessage(contact, message).message
            }

            // Core delegation
            "delegate_task" -> {
                val targetAgentId = params["agent_id"] ?: return "Thiếu 'agent_id'"
                val task = params["task"] ?: return "Thiếu 'task'"
                
                val targetAgent = allAgents.find { it.id == targetAgentId }
                    ?: return "Không tìm thấy Agent: $targetAgentId"
                
                val targetProvider = ProviderManager(context).loadProviders().find { it.id == targetAgent.providerId }
                    ?: return "Agent '${targetAgent.name}' chưa cấu hình Nguồn AI"
                    
                val systemPrompt = LocalAgentManager(context).buildPromptForAgent(targetAgent, task)
                
                Log.i(TAG, "🤖 [delegate_task] Giao cho ${targetAgent.name}: $task")
                try {
                    val result = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                        ProviderChat.chat(targetProvider, systemPrompt, task)
                    }
                    "\n--- Trả lời từ ${targetAgent.name} ---\n$result\n------------------------------"
                } catch (e: Exception) {
                    "⚠️ Lỗi khi hỏi Agent ${targetAgent.name}: ${e.message}"
                }
            }

            else -> "⚠️ Hành động không rõ: $action"
        }
    }

    /**
     * Generate final report using Mama agent.
     */
    private suspend fun generateReport(
        mamaAgent: LocalAgent,
        provider: AIProvider,
        originalCommand: String,
        executionResult: String,
    ): String {
        val reportPrompt = """
Sếp yêu cầu: "$originalCommand"

Kết quả thực hiện:
$executionResult

Hãy tổng hợp thành báo cáo ngắn gọn, rõ ràng gửi lại cho sếp qua Zalo.
LƯU Ý QUAN TRỌNG: Nếu kết quả có chứa nội dung bài viết, kịch bản, email, hoặc văn bản nào do Agent khác tạo ra (hiển thị giữa dấu --- Trả lời từ... ---), hãy CHÉP NGUYÊN VĂN nội dung đó vào báo cáo của bạn. Đừng tóm tắt nó!
Dùng emoji cho dễ đọc.
        """.trimIndent()

        val agentManager = LocalAgentManager(context)
        val systemPrompt = agentManager.buildPromptForAgent(mamaAgent, reportPrompt)

        return try {
            withContext(Dispatchers.IO) {
                ProviderChat.chat(provider, systemPrompt, reportPrompt)
            }
        } catch (e: Exception) {
            // Fallback: return raw results
            "📋 Kết quả:\n$executionResult"
        }
    }

    // ─── Command Logging ─────────────────

    private fun logCommand(log: MamaCommandLog) {
        try {
            val file = File(context.filesDir, LOG_FILE)
            val existing = if (file.exists()) {
                try {
                    mamaJson.decodeFromString<List<MamaCommandLog>>(file.readText())
                } catch (_: Exception) { emptyList() }
            } else emptyList()

            val updated = (listOf(log) + existing).take(100) // Keep last 100
            file.writeText(mamaJson.encodeToString(updated))
        } catch (e: Exception) {
            Log.e(TAG, "Failed to log command: ${e.message}")
        }
    }

    fun getCommandLogs(): List<MamaCommandLog> {
        val file = File(context.filesDir, LOG_FILE)
        if (!file.exists()) return emptyList()
        return try {
            mamaJson.decodeFromString<List<MamaCommandLog>>(file.readText())
        } catch (_: Exception) { emptyList() }
    }
}
