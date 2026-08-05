package vn.bizclaw.agent.messaging

import android.content.Context
import android.content.Intent
import kotlinx.coroutines.delay
import vn.bizclaw.agent.data.Channel

/** Where a proactive send got to, so a failure names the step that broke. */
sealed interface SendStep {
    data object Done : SendStep
    data class Failed(val reason: String) : SendStep
}

/**
 * Opens a chat app and sends a message to a named contact.
 *
 * Used for post-sale check-ins, where no incoming notification exists to reply to. The
 * sequence is deliberately conservative: it never guesses at coordinates, only at
 * labels, and it stops with a named reason the moment a step doesn't find what it
 * expects. Worst case the owner sees "không tìm thấy ô tìm kiếm" and sends by hand.
 */
object ProactiveSender {

    /** Search-entry labels each app uses, in the order worth trying. */
    private val SEARCH_LABELS = listOf("Tìm kiếm", "Tìm", "Search", "Tìm bạn bè, tin nhắn")

    private val SEND_LABELS = listOf("Gửi", "Send", "Gửi tin nhắn")

    private val COMPOSE_HINTS = listOf("Nhắn tin", "Tin nhắn", "Aa", "Message", "Nhập tin nhắn")

    suspend fun send(
        context: Context,
        channel: Channel,
        contactName: String,
        text: String,
    ): SendStep {
        val service = ChatAutomationService.current()
            ?: return SendStep.Failed("Chưa bật quyền Accessibility cho BizClaw")
        val packageName = packageFor(channel)
            ?: return SendStep.Failed("Chưa hỗ trợ tự nhắn trên ${channel.label}")

        val launch = context.packageManager.getLaunchIntentForPackage(packageName)
            ?: return SendStep.Failed("Chưa cài ${channel.label}")
        context.startActivity(launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        delay(APP_LAUNCH_MS)

        // 1. Open search.
        val search = SEARCH_LABELS.firstNotNullOfOrNull { label ->
            service.awaitNode(label, timeoutMs = 4_000)
        } ?: return SendStep.Failed("Không tìm thấy ô tìm kiếm trong ${channel.label}")
        if (!service.click(search)) {
            return SendStep.Failed("Không bấm được ô tìm kiếm")
        }
        delay(STEP_MS)

        // 2. Type the contact name into whatever field search focused.
        val searchField = service.awaitEditable(timeoutMs = 4_000)
            ?: return SendStep.Failed("Không tìm thấy ô nhập tìm kiếm")
        if (!service.setText(searchField, contactName)) {
            return SendStep.Failed("Không gõ được tên khách vào ô tìm kiếm")
        }
        delay(SEARCH_SETTLE_MS)

        // 3. Open the matching conversation. Require an exact-ish label so a partial
        //    match never opens the wrong customer's chat.
        val result = service.awaitNode(contactName, timeoutMs = 5_000) { node ->
            node.isVisibleToUser && !node.isEditable
        } ?: return SendStep.Failed("Không thấy \"$contactName\" trong kết quả tìm kiếm")
        if (!service.click(result)) {
            return SendStep.Failed("Không mở được cuộc trò chuyện với $contactName")
        }
        delay(CONVERSATION_OPEN_MS)

        // 4. Type into the compose box.
        val compose = COMPOSE_HINTS.firstNotNullOfOrNull { hint ->
            service.awaitNode(hint, timeoutMs = 2_000) { it.isEditable }
        } ?: service.awaitEditable(timeoutMs = 4_000)
        ?: return SendStep.Failed("Không tìm thấy ô soạn tin nhắn")
        if (!service.setText(compose, text)) {
            return SendStep.Failed("Không gõ được nội dung tin nhắn")
        }
        delay(STEP_MS)

        // 5. Send.
        val sendButton = SEND_LABELS.firstNotNullOfOrNull { label ->
            service.awaitNode(label, timeoutMs = 2_500) { it.isVisibleToUser }
        } ?: return SendStep.Failed("Không tìm thấy nút Gửi — nội dung đã gõ sẵn, anh bấm gửi tay")
        if (!service.click(sendButton)) {
            return SendStep.Failed("Không bấm được nút Gửi — nội dung đã gõ sẵn")
        }

        return SendStep.Done
    }

    private fun packageFor(channel: Channel): String? = when (channel) {
        Channel.ZALO -> "com.zing.zalo"
        Channel.MESSENGER -> "com.facebook.orca"
        Channel.FACEBOOK, Channel.UNKNOWN -> null
    }

    private const val APP_LAUNCH_MS = 2_500L
    private const val STEP_MS = 600L
    private const val SEARCH_SETTLE_MS = 1_500L
    private const val CONVERSATION_OPEN_MS = 2_000L
}
