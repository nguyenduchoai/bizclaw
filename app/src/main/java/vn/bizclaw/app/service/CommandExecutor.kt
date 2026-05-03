package vn.bizclaw.app.service

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.service.notification.StatusBarNotification
import android.util.Log
import kotlinx.coroutines.*
import vn.bizclaw.app.engine.*

/**
 * CommandExecutor — executes commands on the device.
 *
 * Routes commands to the appropriate service:
 * - ui_action → AccessibilityService (click, type, scroll, etc.)
 * - chat → ProviderChat / LocalLLM
 * - app → Launch intent
 * - social_reply → Notification inline reply (Zalo, Messenger, etc.)
 * - device → Status info
 *
 * This is the "bridge" that connects external commands to device capabilities.
 */
object CommandExecutor {
    private const val TAG = "CommandExec"

    suspend fun execute(context: Context, cmd: DeviceCommand): CommandResult {
        val startTime = System.currentTimeMillis()
        return try {
            val result = when (cmd.type) {
                CommandType.ui_action -> executeUiAction(cmd)
                CommandType.chat -> executeChat(context, cmd)
                CommandType.app -> executeApp(context, cmd)
                CommandType.social_reply -> executeSocialReply(context, cmd)
                CommandType.automation -> executeAutomation(context, cmd)
                CommandType.schedule -> executeSchedule(context, cmd)
                CommandType.mama -> executeMama(context, cmd)
                CommandType.device -> executeDeviceInfo(context, cmd)
                CommandType.flow -> executeFlow(context, cmd)
                CommandType.voice -> executeVoice(context, cmd)
            }
            result.copy(durationMs = System.currentTimeMillis() - startTime)
        } catch (e: Exception) {
            Log.e(TAG, "Command failed: ${cmd.id} — ${e.message}")
            CommandResult(
                id = cmd.id,
                status = CommandStatus.failed,
                error = e.message?.take(200),
                durationMs = System.currentTimeMillis() - startTime,
            )
        }
    }

    // ═══════════════════════════════════════════════════════
    // UI Actions — via AccessibilityService
    // ═══════════════════════════════════════════════════════

    private fun executeUiAction(cmd: DeviceCommand): CommandResult {
        if (!BizClawAccessibilityService.isRunning()) {
            return CommandResult(
                id = cmd.id,
                status = CommandStatus.failed,
                error = "AccessibilityService chưa bật. Vào Settings → Accessibility → BizClaw",
            )
        }

        val success = when (cmd.action) {
            "click_text" -> {
                val text = cmd.params["text"] ?: return errorResult(cmd, "Missing 'text' param")
                BizClawAccessibilityService.clickByText(text)
            }
            "type_text" -> {
                val text = cmd.params["text"] ?: return errorResult(cmd, "Missing 'text' param")
                val fieldHint = cmd.params["field_hint"]
                if (fieldHint != null) {
                    BizClawAccessibilityService.typeIntoField(fieldHint, text)
                } else {
                    BizClawAccessibilityService.typeText(text)
                }
            }
            "tap" -> {
                val x = cmd.params["x"]?.toFloatOrNull() ?: return errorResult(cmd, "Missing 'x'")
                val y = cmd.params["y"]?.toFloatOrNull() ?: return errorResult(cmd, "Missing 'y'")
                BizClawAccessibilityService.tapAt(x, y)
                true
            }
            "swipe" -> {
                val sx = cmd.params["start_x"]?.toFloatOrNull() ?: return errorResult(cmd, "Missing coords")
                val sy = cmd.params["start_y"]?.toFloatOrNull() ?: return errorResult(cmd, "Missing coords")
                val ex = cmd.params["end_x"]?.toFloatOrNull() ?: return errorResult(cmd, "Missing coords")
                val ey = cmd.params["end_y"]?.toFloatOrNull() ?: return errorResult(cmd, "Missing coords")
                val dur = cmd.params["duration_ms"]?.toLongOrNull() ?: 300L
                BizClawAccessibilityService.swipe(sx, sy, ex, ey, dur)
                true
            }
            "scroll_down" -> BizClawAccessibilityService.scrollDown()
            "scroll_up" -> BizClawAccessibilityService.scrollUp()
            "read_screen" -> {
                val content = BizClawAccessibilityService.readScreen()
                val text = if (content != null) {
                    content.elements.joinToString("\n") { el ->
                        "[${el.className.substringAfterLast('.')}] ${el.text} ${el.contentDescription}".trim()
                    }
                } else {
                    "Không đọc được màn hình"
                }
                return CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf("screen_content" to text),
                )
            }
            "press_enter" -> BizClawAccessibilityService.pressEnter()
            else -> return CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown ui_action: ${cmd.action}",
            )
        }

        return CommandResult(
            id = cmd.id,
            status = if (success) CommandStatus.success else CommandStatus.failed,
            result = mapOf("action" to cmd.action, "success" to success.toString()),
        )
    }

    // ═══════════════════════════════════════════════════════
    // Chat — via ProviderChat / LocalLLM
    // ═══════════════════════════════════════════════════════

    private suspend fun executeChat(context: Context, cmd: DeviceCommand): CommandResult {
        val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
        val agentId = cmd.params["agent_id"]
        val providerId = cmd.params["provider_id"]

        val providerManager = ProviderManager(context)
        val agentManager = LocalAgentManager(context)

        // Build the prompt based on agent
        val prompt = if (agentId != null) {
            val agents = agentManager.loadAgents()
            val agent = agents.find { it.id == agentId }
                ?: return errorResult(cmd, "Agent '$agentId' not found")
            agentManager.buildPromptForAgent(agent, message)
        } else {
            message
        }

        // Determine provider
        val providers = providerManager.loadProviders()
        val provider = when {
            providerId != null -> providers.find { it.id == providerId }
                ?: return errorResult(cmd, "Provider '$providerId' not found")
            agentId != null -> {
                val agent = agentManager.loadAgents().find { it.id == agentId }
                providers.find { it.id == agent?.providerId }
                    ?: providers.firstOrNull { it.type == ProviderType.LOCAL_GGUF }
                    ?: return errorResult(cmd, "No provider available")
            }
            else -> providers.firstOrNull { it.enabled }
                ?: return errorResult(cmd, "No provider available")
        }

        val response = withContext(Dispatchers.IO) {
            ProviderChat.chat(provider, prompt, message)
        }

        return CommandResult(
            id = cmd.id,
            status = CommandStatus.success,
            result = mapOf(
                "response" to response,
                "provider" to provider.name,
                "agent" to (agentId ?: "default"),
            ),
        )
    }

    // ═══════════════════════════════════════════════════════
    // Social Reply — via Notification inline reply
    // ═══════════════════════════════════════════════════════

    /**
     * Reply to a social app (Zalo, Messenger, etc.) using Notification inline reply.
     *
     * This is MORE RELIABLE than AccessibilityService because:
     * 1. Works even if the app is not open
     * 2. No need to navigate to the conversation
     * 3. Uses Android's built-in RemoteInput mechanism
     *
     * Flow:
     * 1. Find the notification's reply action (Notification.Action with RemoteInput)
     * 2. Fill the RemoteInput with our text
     * 3. Fire the PendingIntent
     */
    fun replySocialNotification(
        context: Context,
        sbn: StatusBarNotification,
        replyText: String,
    ): Boolean {
        val notification = sbn.notification ?: return false
        val actions = notification.actions ?: return false

        for (action in actions) {
            val remoteInputs = action.remoteInputs ?: continue

            // Found a reply action with RemoteInput!
            for (remoteInput in remoteInputs) {
                try {
                    val intent = Intent()
                    val bundle = Bundle()
                    bundle.putCharSequence(remoteInput.resultKey, replyText)
                    android.app.RemoteInput.addResultsToIntent(arrayOf(remoteInput), intent, bundle)

                    action.actionIntent.send(context, 0, intent)
                    Log.i(TAG, "✅ Replied via notification inline: ${replyText.take(50)}")
                    return true
                } catch (e: PendingIntent.CanceledException) {
                    Log.e(TAG, "Reply PendingIntent cancelled: ${e.message}")
                }
            }
        }

        Log.w(TAG, "No inline reply action found in notification — trying Accessibility fallback")
        return false
    }

    /**
     * Fallback: Reply via AccessibilityService (open app → find input → type → send)
     */
    fun replyViaAccessibility(packageName: String, replyText: String): Boolean {
        if (!BizClawAccessibilityService.isRunning()) return false

        // Type text into the focused input
        val typed = BizClawAccessibilityService.typeText(replyText)
        if (!typed) return false

        // Press send
        return BizClawAccessibilityService.pressEnter()
    }

    private suspend fun executeSocialReply(context: Context, cmd: DeviceCommand): CommandResult {
        val replyText = cmd.params["text"] ?: return errorResult(cmd, "Missing 'text'")
        val packageName = cmd.params["package"] ?: "com.zing.zalo"

        // Try notification inline reply first
        val listener = BizClawNotificationListener.instance
        if (listener != null) {
            val activeNotifs = listener.getActiveNotifications()
            val targetNotif = activeNotifs?.find { it.packageName == packageName }
            if (targetNotif != null) {
                val replied = replySocialNotification(context, targetNotif, replyText)
                if (replied) {
                    return CommandResult(
                        id = cmd.id,
                        status = CommandStatus.success,
                        result = mapOf("method" to "notification_inline", "text" to replyText),
                    )
                }
            }
        }

        // Fallback: try AccessibilityService
        val fallback = withContext(Dispatchers.Main) {
            replyViaAccessibility(packageName, replyText)
        }

        return CommandResult(
            id = cmd.id,
            status = if (fallback) CommandStatus.success else CommandStatus.failed,
            result = mapOf("method" to if (fallback) "accessibility" else "none", "text" to replyText),
            error = if (!fallback) "Không thể reply — cần bật Notification Listener hoặc Accessibility" else null,
        )
    }

    // ═══════════════════════════════════════════════════════
    // App Control
    // ═══════════════════════════════════════════════════════

    private fun executeApp(context: Context, cmd: DeviceCommand): CommandResult {
        return when (cmd.action) {
            "open" -> {
                val pkg = cmd.params["package"] ?: return errorResult(cmd, "Missing 'package'")
                val intent = context.packageManager.getLaunchIntentForPackage(pkg)
                if (intent != null) {
                    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    context.startActivity(intent)
                    CommandResult(
                        id = cmd.id,
                        status = CommandStatus.success,
                        result = mapOf("opened" to pkg),
                    )
                } else {
                    errorResult(cmd, "App not found: $pkg")
                }
            }
            "press_back" -> {
                if (BizClawAccessibilityService.isRunning()) {
                    BizClawAccessibilityService.pressBack()
                    CommandResult(id = cmd.id, status = CommandStatus.success)
                } else errorResult(cmd, "Accessibility not enabled")
            }
            "press_home" -> {
                if (BizClawAccessibilityService.isRunning()) {
                    BizClawAccessibilityService.pressHome()
                    CommandResult(id = cmd.id, status = CommandStatus.success)
                } else errorResult(cmd, "Accessibility not enabled")
            }
            else -> CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown app action: ${cmd.action}",
            )
        }
    }

    // ═══════════════════════════════════════════════════════
    // Device Info
    // ═══════════════════════════════════════════════════════

    private fun executeDeviceInfo(context: Context, cmd: DeviceCommand): CommandResult {
        return when (cmd.action) {
            "status" -> {
                val battery = getBatteryPercent(context)
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "battery" to "$battery%",
                        "model_loaded" to (GlobalLLM.loadedModelName ?: "none"),
                        "accessibility" to BizClawAccessibilityService.isRunning().toString(),
                        "notification_listener" to (BizClawNotificationListener.instance != null).toString(),
                        "daemon" to BizClawDaemonService.isRunning().toString(),
                        "android_version" to Build.VERSION.SDK_INT.toString(),
                    ),
                )
            }
            "battery" -> {
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf("percent" to "${getBatteryPercent(context)}"),
                )
            }
            else -> CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown device action: ${cmd.action}",
            )
        }
    }

    // ═══════════════════════════════════════════════════════
    // Flow — multi-step automation
    // ═══════════════════════════════════════════════════════

    private suspend fun executeFlow(context: Context, cmd: DeviceCommand): CommandResult {
        val stepsJson = cmd.params["steps"] ?: return errorResult(cmd, "Missing 'steps'")
        val steps = try {
            commandJson.decodeFromString<List<DeviceCommand>>(stepsJson)
        } catch (e: Exception) {
            return errorResult(cmd, "Invalid steps JSON: ${e.message?.take(100)}")
        }

        val results = mutableListOf<String>()
        for ((idx, step) in steps.withIndex()) {
            val delayMs = step.params["delay_ms"]?.toLongOrNull() ?: 500L
            delay(delayMs)

            val result = execute(context, step)
            results.add("Step ${idx + 1}: ${result.status}")

            if (result.status == CommandStatus.failed) {
                return CommandResult(
                    id = cmd.id,
                    status = CommandStatus.failed,
                    result = mapOf("results" to results.joinToString("\n")),
                    error = "Flow failed at step ${idx + 1}: ${result.error}",
                )
            }
        }

        return CommandResult(
            id = cmd.id,
            status = CommandStatus.success,
            result = mapOf(
                "steps_completed" to steps.size.toString(),
                "results" to results.joinToString("\n"),
            ),
        )
    }
    // ═══════════════════════════════════════════════════════
    // Automation — high-level app workflows via AppController
    // ═══════════════════════════════════════════════════════

    private suspend fun executeAutomation(context: Context, cmd: DeviceCommand): CommandResult {
        val controller = AppController(context)

        val automationResult = when (cmd.action) {
            // Gmail
            "gmail_read" -> AutomationResult.error("gmail_read is no longer supported")
            "gmail_compose" -> {
                val to = cmd.params["to"] ?: return errorResult(cmd, "Missing 'to'")
                val subject = cmd.params["subject"] ?: return errorResult(cmd, "Missing 'subject'")
                val body = cmd.params["body"] ?: ""
                controller.gmailCompose(to, subject, body)
            }
            "gmail_search" -> AutomationResult.error("gmail_search is no longer supported")
            "gmail_archive" -> AutomationResult.error("gmail_archive is no longer supported")
            "gmail_label" -> AutomationResult.error("gmail_label is no longer supported")
            "gmail_mark_read" -> AutomationResult.error("gmail_mark_read is no longer supported")
            "gmail_mark_unread" -> AutomationResult.error("gmail_mark_unread is no longer supported")

            // Facebook
            "facebook_post" -> {
                val content = cmd.params["content"] ?: return errorResult(cmd, "Missing 'content'")
                controller.facebookPost(content)
            }
            "facebook_comment" -> {
                val comment = cmd.params["comment"] ?: return errorResult(cmd, "Missing 'comment'")
                controller.facebookComment(comment)
            }

            // Messenger
            "messenger_reply" -> {
                val contact = cmd.params["contact"] ?: return errorResult(cmd, "Missing 'contact'")
                val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
                controller.messengerReply(contact, message)
            }
            "messenger_read" -> AutomationResult.error("messenger_read is no longer supported")

            // Zalo
            "zalo_send" -> {
                val contact = cmd.params["contact"] ?: return errorResult(cmd, "Missing 'contact'")
                val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
                controller.zaloSendMessage(contact, message)
            }
            "zalo_post" -> {
                val content = cmd.params["content"] ?: return errorResult(cmd, "Missing 'content'")
                controller.zaloPost(content)
            }
            "zalo_timeline" -> AutomationResult.error("zalo_timeline is no longer supported")

            // Zalo OA (server-side, no UI needed)
            "zalo_oa_send" -> {
                val userId = cmd.params["user_id"] ?: return errorResult(cmd, "Missing 'user_id'")
                val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
                val zaloOA = ZaloOA(context)
                val result = zaloOA.sendMessage(userId, message)
                AutomationResult(result.success, result.message)
            }
            "zalo_oa_broadcast" -> {
                val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
                val zaloOA = ZaloOA(context)
                val result = zaloOA.broadcast(message)
                AutomationResult(result.success, result.message)
            }

            // Instagram
            "instagram_post" -> {
                val caption = cmd.params["caption"] ?: return errorResult(cmd, "Missing 'caption'")
                controller.instagramCaption(caption)
            }

            // Lark
            "lark_read_chats" -> AutomationResult.error("lark_read_chats is no longer supported")
            "lark_send" -> {
                val contact = cmd.params["contact"] ?: return errorResult(cmd, "Missing 'contact'")
                val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
                controller.larkSendMessage(contact, message)
            }
            "lark_read_mail" -> AutomationResult.error("lark_read_mail is no longer supported")
            "lark_compose_mail" -> {
                val to = cmd.params["to"] ?: return errorResult(cmd, "Missing 'to'")
                val subject = cmd.params["subject"] ?: return errorResult(cmd, "Missing 'subject'")
                val body = cmd.params["body"] ?: ""
                controller.larkComposeMail(to, subject, body)
            }

            // Telegram
            "telegram_read" -> AutomationResult.error("telegram_read is no longer supported")
            "telegram_send" -> {
                val contact = cmd.params["contact"] ?: return errorResult(cmd, "Missing 'contact'")
                val message = cmd.params["message"] ?: return errorResult(cmd, "Missing 'message'")
                controller.telegramSendMessage(contact, message)
            }

            // Threads
            "threads_post" -> {
                val content = cmd.params["content"] ?: return errorResult(cmd, "Missing 'content'")
                controller.threadsPost(content)
            }
            "threads_read" -> AutomationResult.error("threads_read is no longer supported")

            // Screen reading
            "read_screen" -> AutomationResult.error("read_screen is no longer supported")
            "click" -> {
                val text = cmd.params["text"] ?: return errorResult(cmd, "Missing 'text'")
                controller.clickElement(text)
            }

            else -> return CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown automation: ${cmd.action}. Available: gmail_*, facebook_*, messenger_*, zalo_send, zalo_post, zalo_timeline, instagram_post, threads_*, lark_*, telegram_*",
            )
        }

        return CommandResult(
            id = cmd.id,
            status = if (automationResult.success) CommandStatus.success else CommandStatus.failed,
            result = mapOf("message" to automationResult.message),
            error = if (!automationResult.success) automationResult.message else null,
        )
    }

    // ─── Helpers ─────────────────────────────────
    // ═══════════════════════════════════════════════════════
    // Schedule — manage automation jobs
    // ═══════════════════════════════════════════════════════

    private suspend fun executeSchedule(context: Context, cmd: DeviceCommand): CommandResult {
        val manager = AutomationJobManager(context)

        return when (cmd.action) {
            "list" -> {
                val jobs = manager.loadJobs()
                val summary = if (jobs.isEmpty()) {
                    "Chưa có job nào"
                } else {
                    jobs.joinToString("\n") { job ->
                        "${job.emoji} ${job.name} [${if (job.enabled) "ON" else "OFF"}] " +
                        "| Agent: ${job.agentId} | Every ${job.intervalMinutes}m | " +
                        "Runs: ${job.runCount} | Deliver: ${job.delivery.method} → ${job.delivery.target}"
                    }
                }
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "count" to jobs.size.toString(),
                        "jobs" to summary,
                    ),
                )
            }

            "create" -> {
                val name = cmd.params["name"] ?: return errorResult(cmd, "Missing 'name'")
                val agentId = cmd.params["agent_id"] ?: return errorResult(cmd, "Missing 'agent_id'")
                val interval = cmd.params["interval_minutes"]?.toIntOrNull() ?: 240
                val deliveryMethod = cmd.params["delivery_method"] ?: "ZALO"
                val deliveryTarget = cmd.params["delivery_target"] ?: ""
                val emoji = cmd.params["emoji"] ?: "📊"

                // Parse sources
                val sourceTypes = cmd.params["sources"]?.split(",") ?: listOf("NOTIFICATIONS")
                val sourceTarget = cmd.params["source_target"] ?: ""
                val sources = sourceTypes.map { sourceStr ->
                    DataSource(
                        type = try { SourceType.valueOf(sourceStr.trim()) } catch (_: Exception) { SourceType.NOTIFICATIONS },
                        target = sourceTarget,
                    )
                }

                // Parse schedule times if provided
                val scheduleTimes = cmd.params["schedule_times"]
                    ?.split(",")
                    ?.map { it.trim() }
                    ?: emptyList()

                val job = AutomationJob(
                    id = java.util.UUID.randomUUID().toString().take(8),
                    name = name,
                    emoji = emoji,
                    agentId = agentId,
                    sources = sources,
                    intervalMinutes = interval,
                    scheduleTimes = scheduleTimes,
                    delivery = DeliveryConfig(
                        method = try { DeliveryMethod.valueOf(deliveryMethod) } catch (_: Exception) { DeliveryMethod.ZALO },
                        target = deliveryTarget,
                    ),
                )

                manager.addJob(job)

                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "job_id" to job.id,
                        "message" to "✅ Job '${job.name}' created — runs every ${interval}m, delivers via $deliveryMethod",
                    ),
                )
            }

            "run" -> {
                val jobId = cmd.params["job_id"] ?: return errorResult(cmd, "Missing 'job_id'")
                val job = manager.getJob(jobId) ?: return errorResult(cmd, "Job not found: $jobId")

                val report = manager.executeJob(job)

                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf("report" to report),
                )
            }

            "delete" -> {
                val jobId = cmd.params["job_id"] ?: return errorResult(cmd, "Missing 'job_id'")
                manager.deleteJob(jobId)
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf("message" to "🗑️ Job $jobId deleted"),
                )
            }

            "toggle" -> {
                val jobId = cmd.params["job_id"] ?: return errorResult(cmd, "Missing 'job_id'")
                val job = manager.getJob(jobId) ?: return errorResult(cmd, "Job not found: $jobId")
                val updated = job.copy(enabled = !job.enabled)
                manager.updateJob(updated)
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "message" to "${if (updated.enabled) "✅ ON" else "🚫 OFF"}: ${job.name}",
                    ),
                )
            }

            else -> CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown schedule action: ${cmd.action}. Available: list, create, run, delete, toggle",
            )
        }
    }

    // ═══════════════════════════════════════════════════════
    // Mama — boss command configuration
    // ═══════════════════════════════════════════════════════

    private suspend fun executeMama(context: Context, cmd: DeviceCommand): CommandResult {
        val mama = MamaAgent(context)

        return when (cmd.action) {
            "setup" -> {
                val bossContacts = cmd.params["boss_contacts"]?.split(",")?.map { it.trim() }
                    ?: return errorResult(cmd, "Missing 'boss_contacts' (comma-separated)")
                val agentId = cmd.params["agent_id"]
                    ?: return errorResult(cmd, "Missing 'agent_id' for Mama orchestrator")

                val config = mama.loadConfig().copy(
                    bossContacts = bossContacts,
                    mamaAgentId = agentId,
                    enabled = true,
                    replyToBoss = cmd.params["reply"] != "false",
                )
                mama.saveConfig(config)

                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "message" to "👑 Mama activated! Boss: ${bossContacts.joinToString()}, Agent: $agentId",
                        "boss_contacts" to bossContacts.joinToString(),
                        "agent_id" to agentId,
                    ),
                )
            }

            "status" -> {
                val config = mama.loadConfig()
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "enabled" to config.enabled.toString(),
                        "boss_contacts" to config.bossContacts.joinToString(),
                        "agent_id" to config.mamaAgentId,
                        "reply_to_boss" to config.replyToBoss.toString(),
                    ),
                )
            }

            "toggle" -> {
                val config = mama.loadConfig()
                val updated = config.copy(enabled = !config.enabled)
                mama.saveConfig(updated)
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "message" to "${if (updated.enabled) "👑 ON" else "🚫 OFF"}: Mama",
                    ),
                )
            }

            "test" -> {
                val message = cmd.params["message"]
                    ?: return errorResult(cmd, "Missing 'message' to test")
                val sender = cmd.params["sender"] ?: "Test Boss"

                val report = mama.processCommand(sender, message)
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf("report" to report),
                )
            }

            "logs" -> {
                val logs = mama.getCommandLogs()
                val summary = if (logs.isEmpty()) {
                    "Chưa có lệnh nào"
                } else {
                    logs.take(10).joinToString("\n") { log ->
                        val time = java.text.SimpleDateFormat("HH:mm dd/MM", java.util.Locale.getDefault())
                            .format(java.util.Date(log.timestamp))
                        "[$time] ${log.from}: ${log.command.take(50)} → ${if (log.success) "✅" else "❌"}"
                    }
                }
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "count" to logs.size.toString(),
                        "logs" to summary,
                    ),
                )
            }

            else -> CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown mama action: ${cmd.action}. Available: setup, status, toggle, test, logs",
            )
        }
    }

    // ═══════════════════════════════════════════════════════
    // Voice — record + transcribe via AudioRecorder
    // ═══════════════════════════════════════════════════════

    // Singleton recorder instance (shared across commands)
    private var audioRecorder: AudioRecorder? = null

    private fun getRecorder(context: Context): AudioRecorder {
        if (audioRecorder == null) {
            audioRecorder = AudioRecorder(context)
        }
        return audioRecorder!!
    }

    private suspend fun executeVoice(context: Context, cmd: DeviceCommand): CommandResult {
        val recorder = getRecorder(context)

        return when (cmd.action) {
            "start" -> {
                val started = recorder.startRecording()
                CommandResult(
                    id = cmd.id,
                    status = if (started) CommandStatus.success else CommandStatus.failed,
                    result = mapOf("message" to if (started) "🎙️ Recording started" else "Failed to start"),
                    error = if (!started) "Cannot start recording — check RECORD_AUDIO permission" else null,
                )
            }

            "stop" -> {
                val result = recorder.stopRecording()
                    ?: return errorResult(cmd, "No recording in progress or recording failed")

                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "file_path" to result.filePath,
                        "file_name" to result.fileName,
                        "duration" to result.durationFormatted,
                        "size" to result.sizeFormatted,
                        "message" to "✅ Recording saved: ${result.fileName} (${result.durationFormatted}, ${result.sizeFormatted})",
                    ),
                )
            }

            "cancel" -> {
                recorder.cancelRecording()
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf("message" to "🗑️ Recording cancelled"),
                )
            }

            "status" -> {
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "is_recording" to recorder.isRecording.toString(),
                        "elapsed_ms" to recorder.getElapsedMs().toString(),
                    ),
                )
            }

            "list" -> {
                val recordings = recorder.listRecordings()
                val summary = if (recordings.isEmpty()) {
                    "No recordings found"
                } else {
                    recordings.joinToString("\n") { r ->
                        "📁 ${r.fileName} (${r.sizeFormatted})"
                    }
                }
                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "count" to recordings.size.toString(),
                        "recordings" to summary,
                    ),
                )
            }

            "delete" -> {
                val path = cmd.params["file_path"]
                    ?: return errorResult(cmd, "Missing 'file_path'")
                val deleted = recorder.deleteRecording(path)
                CommandResult(
                    id = cmd.id,
                    status = if (deleted) CommandStatus.success else CommandStatus.failed,
                    result = mapOf("message" to if (deleted) "🗑️ Deleted" else "File not found"),
                )
            }

            "transcribe" -> {
                // Upload audio file to gateway for transcription
                val filePath = cmd.params["file_path"]
                    ?: return errorResult(cmd, "Missing 'file_path'. Record first with voice.start/stop")

                val file = java.io.File(filePath)
                if (!file.exists()) {
                    return errorResult(cmd, "Audio file not found: $filePath")
                }

                val provider = cmd.params["provider"] ?: "whisper"
                val language = cmd.params["language"] ?: "vi"

                // Use the gateway's voice_transcribe tool via agent chat
                val message = """Hãy dùng tool voice_transcribe để:
1. Transcribe file audio tại: $filePath
2. Provider: $provider, Language: $language
3. Action: both (transcribe + recap)
4. Sau khi có transcript, tạo bản recap ngắn gọn với các điểm chính."""

                val response = withContext(Dispatchers.IO) {
                    // Route through the default agent
                    val providerManager = ProviderManager(context)
                    val agentManager = LocalAgentManager(context)
                    val providers = providerManager.loadProviders()
                    val defaultProvider = providers.firstOrNull { it.enabled }

                    if (defaultProvider != null) {
                        ProviderChat.appContext = context
                        ProviderChat.chat(defaultProvider, "", message)
                    } else {
                        "⚠️ No AI provider configured. Transcript file saved at: $filePath"
                    }
                }

                CommandResult(
                    id = cmd.id,
                    status = CommandStatus.success,
                    result = mapOf(
                        "file" to filePath,
                        "transcript" to response,
                        "provider" to provider,
                        "language" to language,
                    ),
                )
            }

            else -> CommandResult(
                id = cmd.id,
                status = CommandStatus.unsupported,
                error = "Unknown voice action: ${cmd.action}. Available: start, stop, cancel, status, list, delete, transcribe",
            )
        }
    }

    private fun errorResult(cmd: DeviceCommand, error: String) = CommandResult(
        id = cmd.id,
        status = CommandStatus.failed,
        error = error,
    )

    private fun getBatteryPercent(context: Context): Int {
        return try {
            val batteryIntent = context.registerReceiver(
                null,
                android.content.IntentFilter(Intent.ACTION_BATTERY_CHANGED),
            )
            val level = batteryIntent?.getIntExtra(android.os.BatteryManager.EXTRA_LEVEL, -1) ?: -1
            val scale = batteryIntent?.getIntExtra(android.os.BatteryManager.EXTRA_SCALE, 100) ?: 100
            if (level >= 0 && scale > 0) (level * 100) / scale else -1
        } catch (_: Exception) { -1 }
    }
}
