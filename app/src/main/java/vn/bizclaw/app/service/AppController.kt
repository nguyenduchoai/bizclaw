package vn.bizclaw.app.service

import android.content.Context
import android.content.Intent
import android.net.Uri
import kotlinx.coroutines.delay

/**
 * AppController — high-level automation for popular apps.
 *
 * Uses BizClawAccessibilityService to control apps like Facebook, Messenger, Zalo.
 * Each method is a complete "workflow" that agents can call as a single tool.
 *
 * ⚙️ Architecture:
 *   Agent tool call "facebook.post"
 *       → AppController.facebookPost()
 *           → Open Facebook app
 *           → Find "Bạn đang nghĩ gì?" field
 *           → Tap → Type content → Tap "Đăng"
 *
 * ⚠️ IMPORTANT:
 * - Accessibility Service must be enabled by user
 * - UI elements may change with app updates (Facebook, Messenger...)
 * - Use Vietnamese localized text for element matching
 * - Add delays between actions for UI to render
 */
class AppController(private val context: Context) {

    private val a11y get() = BizClawAccessibilityService

    // ─── Facebook ─────────────────────────────────────────────────

    /**
     * Post content to Facebook feed.
     *
     * Flow:
     * 1. Open Facebook app
     * 2. Find "Bạn đang nghĩ gì?" or "What's on your mind?"
     * 3. Tap to open composer
     * 4. Type content
     * 5. Tap "Đăng" / "Post"
     */
    suspend fun facebookPost(content: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            // Step 1: Open Facebook
            openApp("com.facebook.katana")
            delay(2000) // Wait for app to launch

            // Step 2: Find and tap the "What's on your mind?" field
            val tapped = a11y.clickByText("Bạn đang nghĩ gì")
                || a11y.clickByText("Bạn nghĩ gì")
                || a11y.clickByText("What's on your mind")
                || a11y.clickByText("Viết gì đó")
            if (!tapped) return AutomationResult.error("Cannot find post composer field")
            delay(2000)

            // Step 3: Type content
            val typed = a11y.typeIntoField("Bạn đang nghĩ gì", content)
                || a11y.typeText(content)
            if (!typed) return AutomationResult.error("Cannot type into post field")
            delay(1000)

            // Step 4: Tap Post button
            val posted = a11y.clickByText("Đăng")
                || a11y.clickByText("Post")
            if (!posted) return AutomationResult.error("Cannot find Post button")

            AutomationResult.success("Posted to Facebook: ${content.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Facebook post failed: ${e.message}")
        }
    }

    /**
     * Comment on the first/current post visible on Facebook.
     */
    suspend fun facebookComment(comment: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            // Find and tap Comment button/icon
            val tapped = a11y.clickByText("Bình luận")
                || a11y.clickByText("Comment")
            if (!tapped) return AutomationResult.error("Cannot find Comment button")
            delay(1000)

            // Type comment
            val typed = a11y.typeText(comment)
            if (!typed) return AutomationResult.error("Cannot type comment")
            delay(300)

            // Send comment (Enter or send button)
            a11y.pressEnter()

            AutomationResult.success("Commented on Facebook: ${comment.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Facebook comment failed: ${e.message}")
        }
    }

    // ─── Messenger ────────────────────────────────────────────────

    /**
     * Reply to a Messenger conversation by contact name.
     *
     * Flow:
     * 1. Open Messenger
     * 2. Find conversation by name
     * 3. Tap to open
     * 4. Type and send message
     */
    suspend fun messengerReply(contactName: String, message: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            // Step 1: Open Messenger
            openApp("com.facebook.orca")
            delay(2000)

            // Step 2: Find and tap conversation
            val found = a11y.clickByText(contactName)
            if (!found) return AutomationResult.error("Cannot find conversation: $contactName")
            delay(1500)

            // Step 3: Find message input and type
            val typed = a11y.typeIntoField("Aa", message)
                || a11y.typeIntoField("Message", message)
                || a11y.typeIntoField("Nhắn tin", message)
                || a11y.typeText(message)
            if (!typed) return AutomationResult.error("Cannot type into message field")
            delay(300)

            // Step 4: Send (tap send button or press enter)
            val sent = a11y.clickByText("Gửi")
                || a11y.clickByText("Send")
                || a11y.pressEnter()
            if (!sent) return AutomationResult.error("Cannot send message")

            AutomationResult.success("Sent to $contactName: ${message.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Messenger reply failed: ${e.message}")
        }
    }

    // ─── Zalo ─────────────────────────────────────────────────────

    /**
     * Send a Zalo message to a contact.
     */
    suspend fun zaloSendMessage(contactName: String, message: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp("com.zing.zalo")
            delay(2000)

            val found = a11y.clickByText(contactName)
            if (!found) return AutomationResult.error("Cannot find: $contactName")
            delay(1500)

            val typed = a11y.typeIntoField("Nhắn tin", message)
                || a11y.typeIntoField("Tin nhắn", message)
                || a11y.typeText(message)
            if (!typed) return AutomationResult.error("Cannot type message")
            delay(300)

            a11y.clickByText("Gửi") || a11y.pressEnter()

            AutomationResult.success("Zalo sent to $contactName: ${message.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Zalo failed: ${e.message}")
        }
    }

    /** Post to Zalo Timeline/Nhật ký (đăng bài mạng xã hội Zalo) */
    suspend fun zaloPost(content: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp("com.zing.zalo")
            delay(2000)

            // Navigate to Nhật ký (Timeline) tab
            val goToTimeline = a11y.clickByText("Nhật ký")
                || a11y.clickByText("Timeline")
                || a11y.clickByText("Cá nhân")
            if (!goToTimeline) {
                // Try tab index 3 (usually Nhật ký is the 3rd or 4th tab)
                a11y.clickByText("Khám phá")
            }
            delay(1500)

            // Tap compose / create post
            val compose = a11y.clickByText("Hôm nay bạn")
                || a11y.clickByText("Bạn đang nghĩ gì")
                || a11y.clickByText("Đăng gì đó")
                || a11y.clickByText("Viết bài")
                || a11y.clickByText("Tạo bài viết")
                || a11y.clickByText("What's on your mind")
            if (!compose) return AutomationResult.error("Không tìm thấy ô đăng bài Zalo")
            delay(2000)

            // Type post content
            val typed = a11y.typeIntoField("nhĩ gì", content)
                || a11y.typeIntoField("Hãy chia sẻ", content)
                || a11y.typeIntoField("Chia sẻ", content)
                || a11y.typeText(content)
            if (!typed) return AutomationResult.error("Không nhập được nội dung bài Zalo")
            delay(1000)

            // Post / Đăng
            val posted = a11y.clickByText("Đăng")
                || a11y.clickByText("Post")
                || a11y.clickByText("Chia sẻ")
            if (!posted) return AutomationResult.error("Không tìm nút Đăng bài Zalo")

            AutomationResult.success("📝 Zalo Timeline posted: ${content.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Zalo post failed: ${e.message}")
        }
    }



    /**
     * Compose and send an email via Gmail.
     *
     * Flow:
     * 1. Open Gmail app
     * 2. Tap Compose button
     * 3. Fill To, Subject, Body
     * 4. Tap Send
     */
    suspend fun gmailCompose(to: String, subject: String, body: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp("com.google.android.gm")
            delay(2000)

            // Tap Compose / Soạn thư
            val tapped = a11y.clickByText("Compose")
                || a11y.clickByText("Soạn thư")
                || a11y.clickByText("✏️")
            if (!tapped) return AutomationResult.error("Cannot find Compose button")
            delay(1500)

            // Fill To field
            val toFilled = a11y.typeIntoField("To", to)
                || a11y.typeIntoField("Tới", to)
            if (!toFilled) return AutomationResult.error("Cannot fill To field")
            delay(300)

            // Fill Subject
            val subFilled = a11y.typeIntoField("Subject", subject)
                || a11y.typeIntoField("Chủ đề", subject)
            if (!subFilled) return AutomationResult.error("Cannot fill Subject field")
            delay(300)

            // Fill Body
            val bodyFilled = a11y.typeIntoField("Compose email", body)
                || a11y.typeIntoField("Soạn email", body)
                || a11y.typeText(body)
            if (!bodyFilled) return AutomationResult.error("Cannot fill email body")
            delay(300)

            // Send
            val sent = a11y.clickByText("Send")
                || a11y.clickByText("Gửi")
                || a11y.clickByText("➤")
            if (!sent) return AutomationResult.error("Cannot find Send button")

            AutomationResult.success("📧 Email sent to $to: $subject")
        } catch (e: Exception) {
            AutomationResult.error("Gmail compose failed: ${e.message}")
        }
    }

    // ─── Instagram ────────────────────────────────────────────────

    /**
     * Post a caption to Instagram (assumes image is already selected or camera is open).
     * For full posting, user needs to select image first.
     */
    suspend fun instagramCaption(caption: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            // If Instagram isn't open, open it
            openApp("com.instagram.android")
            delay(2000)

            // Tap the create/post button (+ icon)
            val createTapped = a11y.clickByText("Create")
                || a11y.clickByText("Tạo")
                || a11y.clickByText("+")
            if (!createTapped) return AutomationResult.error("Cannot find Create button")
            delay(1500)

            // Select Post option
            a11y.clickByText("Post") || a11y.clickByText("Bài viết")
            delay(1000)

            // Select first image (tap "Next" to proceed)
            a11y.clickByText("Next") || a11y.clickByText("Tiếp")
            delay(1000)

            // Skip filters
            a11y.clickByText("Next") || a11y.clickByText("Tiếp")
            delay(1000)

            // Type caption
            val typed = a11y.typeIntoField("Write a caption", caption)
                || a11y.typeIntoField("Viết chú thích", caption)
                || a11y.typeText(caption)
            if (!typed) return AutomationResult.error("Cannot type caption")
            delay(300)

            // Share
            val shared = a11y.clickByText("Share")
                || a11y.clickByText("Chia sẻ")
            if (!shared) return AutomationResult.error("Cannot find Share button")

            AutomationResult.success("📸 Instagram posted: ${caption.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Instagram post failed: ${e.message}")
        }
    }

    // ─── Threads (Meta) ─────────────────────────────────────

    /** Post to Threads */
    suspend fun threadsPost(content: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp("com.instagram.barcelona")
            delay(2000)

            // Tap compose/new post button
            val newPost = a11y.clickByText("New thread")
                || a11y.clickByText("Tạo thread mới")
                || a11y.clickByText("+")
                || a11y.clickByText("Đăng")
            if (!newPost) {
                // Try tapping the FAB/compose icon at bottom
                a11y.clickByText("Compose")
                    || a11y.clickByText("Soạn")
            }
            delay(1500)

            // Type content
            val typed = a11y.typeText(content)
            if (!typed) return AutomationResult.error("Cannot type in Threads")
            delay(300)

            // Post
            val posted = a11y.clickByText("Post")
                || a11y.clickByText("Đăng")
                || a11y.clickByText("Share")
            if (!posted) return AutomationResult.error("Cannot find Post button")

            AutomationResult.success("🧵 Threads posted: ${content.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Threads post failed: ${e.message}")
        }
    }

    // ─── Lark (Feishu) ────────────────────────────────────────

    /** Lark package — international version; CN = com.ss.android.lark */
    private val larkPackage: String
        get() {
            // Try international first, fall back to CN
            val intent = context.packageManager.getLaunchIntentForPackage("com.larksuite.suite")
            return if (intent != null) "com.larksuite.suite" else "com.ss.android.lark"
        }

    /**
     * Send a Lark message to a contact/group.
     */
    suspend fun larkSendMessage(contactName: String, message: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp(larkPackage)
            delay(2000)

            // Find and tap conversation
            val found = a11y.clickByText(contactName)
            if (!found) return AutomationResult.error("Cannot find Lark chat: $contactName")
            delay(1500)

            // Type message
            val typed = a11y.typeIntoField("Message", message)
                || a11y.typeIntoField("Tin nhắn", message)
                || a11y.typeIntoField("输入消息", message)
                || a11y.typeText(message)
            if (!typed) return AutomationResult.error("Cannot type message in Lark")
            delay(300)

            // Send
            val sent = a11y.clickByText("Send")
                || a11y.clickByText("Gửi")
                || a11y.clickByText("发送")
                || a11y.pressEnter()
            if (!sent) return AutomationResult.error("Cannot send Lark message")

            AutomationResult.success("💬 Lark sent to $contactName: ${message.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Lark send failed: ${e.message}")
        }
    }

    /**
     * Compose and send a Lark Mail.
     */
    suspend fun larkComposeMail(to: String, subject: String, body: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp(larkPackage)
            delay(2000)

            // Go to Mail
            a11y.clickByText("Mail") || a11y.clickByText("邮箱")
                || a11y.clickByText("Hộp thư")
            delay(1500)

            // Tap Compose
            val composeTapped = a11y.clickByText("Compose")
                || a11y.clickByText("写邮件")
                || a11y.clickByText("Soạn thư")
                || a11y.clickByText("✏️")
            if (!composeTapped) return AutomationResult.error("Cannot find Compose in Lark Mail")
            delay(1500)

            // Fill To
            val toFilled = a11y.typeIntoField("To", to)
                || a11y.typeIntoField("收件人", to)
                || a11y.typeIntoField("Tới", to)
            if (!toFilled) return AutomationResult.error("Cannot fill To field")
            delay(300)

            // Fill Subject
            val subFilled = a11y.typeIntoField("Subject", subject)
                || a11y.typeIntoField("主题", subject)
                || a11y.typeIntoField("Chủ đề", subject)
            if (!subFilled) return AutomationResult.error("Cannot fill Subject")
            delay(300)

            // Fill Body
            val bodyFilled = a11y.typeIntoField("Content", body)
                || a11y.typeIntoField("正文", body)
                || a11y.typeText(body)
            if (!bodyFilled) return AutomationResult.error("Cannot fill mail body")
            delay(300)

            // Send
            val sent = a11y.clickByText("Send")
                || a11y.clickByText("发送")
                || a11y.clickByText("Gửi")
            if (!sent) return AutomationResult.error("Cannot find Send in Lark Mail")

            AutomationResult.success("📧 Lark Mail sent to $to: $subject")
        } catch (e: Exception) {
            AutomationResult.error("Lark mail compose failed: ${e.message}")
        }
    }

    // ─── Telegram ─────────────────────────────────────────────

    /**
     * Send a Telegram message to a contact/group.
     */
    suspend fun telegramSendMessage(contactName: String, message: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        return try {
            openApp("org.telegram.messenger")
            delay(2000)

            // Find and tap conversation
            val found = a11y.clickByText(contactName)
            if (!found) return AutomationResult.error("Cannot find Telegram chat: $contactName")
            delay(1500)

            // Type message
            val typed = a11y.typeIntoField("Message", message)
                || a11y.typeIntoField("Tin nhắn", message)
                || a11y.typeText(message)
            if (!typed) return AutomationResult.error("Cannot type Telegram message")
            delay(300)

            // Send
            a11y.clickByText("Send") || a11y.pressEnter()

            AutomationResult.success("✈️ Telegram sent to $contactName: ${message.take(50)}...")
        } catch (e: Exception) {
            AutomationResult.error("Telegram send failed: ${e.message}")
        }
    }

    // ─── Generic App Control ──────────────────────────────────────

    /**
     * Click any button/element by its text on the current screen.
     */
    fun clickElement(text: String): AutomationResult {
        if (!a11y.isRunning()) return AutomationResult.error("Accessibility service not enabled")

        val clicked = a11y.clickByText(text)
        return if (clicked) {
            AutomationResult.success("Clicked: $text")
        } else {
            AutomationResult.error("Element not found: $text")
        }
    }

    /**
     * Open an app by package name.
     */
    fun openApp(packageName: String) {
        val intent = context.packageManager.getLaunchIntentForPackage(packageName)
        if (intent != null) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(intent)
        }
    }

    /**
     * Open a URL in the default browser.
     */
    fun openUrl(url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
    }
}

// ─── Result Type ──────────────────────────────────────────────────────

data class AutomationResult(
    val success: Boolean,
    val message: String,
) {
    companion object {
        fun success(message: String) = AutomationResult(true, message)
        fun error(message: String) = AutomationResult(false, message)
    }
}
