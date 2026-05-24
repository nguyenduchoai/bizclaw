package vn.bizclaw.app

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Assignment
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.CloudDone
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.MenuBook
import androidx.compose.material.icons.filled.OpenInNew
import androidx.compose.material.icons.filled.PhoneAndroid
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Translate
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Divider
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL
import java.util.Locale
import java.util.UUID

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            BizClawMobileTheme {
                val appState = remember { MobileMamaState(applicationContext) }
                val sharedText = intent?.takeIf {
                    it.action == Intent.ACTION_SEND && it.type?.startsWith("text/") == true
                }?.getStringExtra(Intent.EXTRA_TEXT).orEmpty()
                if (sharedText.isNotBlank()) {
                    appState.pendingInput = sharedText
                }
                BizClawMobileApp(appState)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
    }
}

private enum class MobileTab(
    val title: String,
    val icon: ImageVector,
) {
    Chat("Mama", Icons.Default.Chat),
    Mission("Việc", Icons.Default.Assignment),
    Channels("Kênh", Icons.Default.Email),
    Translate("Phiên dịch", Icons.Default.Translate),
    Knowledge("Tri thức", Icons.Default.MenuBook),
    Bridge("Bridge", Icons.Default.PhoneAndroid),
}

private data class ChatItem(
    val id: String = UUID.randomUUID().toString(),
    val role: String,
    val content: String,
    val createdAt: Long = System.currentTimeMillis(),
)

private data class MobileTicket(
    val id: String,
    val title: String,
    val request: String,
    val source: String,
    val agent: String,
    val status: String,
    val priority: String,
    val approvalRequired: Boolean,
    val nextAction: String,
    val reply: String,
    val createdAt: Long = System.currentTimeMillis(),
)

private data class KnowledgeDoc(
    val id: String,
    val name: String,
    val content: String,
    val source: String,
    val createdAt: Long = System.currentTimeMillis(),
)

private data class EmailRule(
    val id: String,
    val instruction: String,
    val condition: String,
    val actions: List<String>,
)

private data class EmailThread(
    val id: String,
    val from: String,
    val subject: String,
    val body: String,
    val status: String,
    val labels: List<String>,
    val draft: String,
    val knowledgeSaved: Boolean = false,
    val createdAt: Long = System.currentTimeMillis(),
)

private class MobileMamaState(private val context: Context) {
    private val prefs = context.getSharedPreferences("bizclaw_mobile", Context.MODE_PRIVATE)
    private val client = DesktopMamaClient()
    val translator = TranslationController(context)

    val messages = mutableStateListOf<ChatItem>()
    val tickets = mutableStateListOf<MobileTicket>()
    val knowledgeDocs = mutableStateListOf<KnowledgeDoc>()
    val emailRules = mutableStateListOf<EmailRule>()
    val emailThreads = mutableStateListOf<EmailThread>()

    var mode by mutableStateOf(prefs.getString("mode", "local") ?: "local")
    var serverUrl by mutableStateOf(prefs.getString("server_url", BuildConfig.DEFAULT_SERVER_URL) ?: BuildConfig.DEFAULT_SERVER_URL)
    var apiKey by mutableStateOf(prefs.getString("api_key", "") ?: "")
    var pendingInput by mutableStateOf("")
    var notice by mutableStateOf("Sẵn sàng.")
    var isBusy by mutableStateOf(false)
    var query by mutableStateOf("")

    init {
        loadTickets()
        loadKnowledge()
        loadEmailThreads()
        emailRules.addAll(loadRules())
        if (messages.isEmpty()) {
            messages.add(ChatItem(role = "assistant", content = "Mama Mobile đã sẵn sàng. Anh giao việc bằng chat; Android có thể xử lý local hoặc gửi về Desktop."))
        }
    }

    fun switchMode(nextMode: String) {
        mode = nextMode
        prefs.edit().putString("mode", nextMode).apply()
    }

    fun saveBridgeSettings() {
        prefs.edit()
            .putString("server_url", serverUrl.trim().trimEnd('/'))
            .putString("api_key", apiKey.trim())
            .putString("mode", mode)
            .apply()
        notice = "Đã lưu cấu hình Bridge."
    }

    fun submitMama(text: String, scope: CoroutineScope) {
        val request = text.trim()
        if (request.isEmpty()) return
        messages.add(ChatItem(role = "user", content = request))
        pendingInput = ""
        if (mode == "desktop") {
            sendToDesktop(request, "android_mobile_chat", scope)
        } else {
            val ticket = routeLocal(request, "android_mobile_chat")
            appendTicket(ticket)
            messages.add(ChatItem(role = "assistant", content = ticket.reply))
            notice = "Đã tạo local ticket ${ticket.id}."
        }
    }

    fun sendToDesktop(request: String, source: String, scope: CoroutineScope) {
        isBusy = true
        notice = "Đang gửi Desktop..."
        scope.launch {
            val result = client.postMama(
                serverUrl = serverUrl,
                token = apiKey,
                request = request,
                source = source,
                customerRef = android.os.Build.MODEL ?: "android-device",
            )
            result.onSuccess { reply ->
                messages.add(ChatItem(role = "assistant", content = reply))
                val ticket = routeLocal(request, source).copy(
                    status = "synced_desktop",
                    reply = reply,
                    nextAction = "Theo dõi ticket trên BizClaw Desktop Mission Control.",
                )
                appendTicket(ticket)
                notice = "Desktop đã nhận việc."
            }.onFailure { error ->
                val ticket = routeLocal(request, "desktop_fallback")
                appendTicket(ticket)
                messages.add(ChatItem(role = "assistant", content = "Không kết nối được Desktop nên Mama đã lưu ticket local.\n\n${ticket.reply}\n\nLỗi: ${error.message}"))
                notice = "Fallback local: ${ticket.id}."
            }
            isBusy = false
        }
    }

    fun healthCheck(scope: CoroutineScope) {
        saveBridgeSettings()
        isBusy = true
        scope.launch {
            notice = client.health(serverUrl)
                .fold(
                    onSuccess = { "Desktop/Gateway health: HTTP $it" },
                    onFailure = { "Không ping được Desktop: ${it.message}" },
                )
            isBusy = false
        }
    }

    fun syncLatest(scope: CoroutineScope) {
        val ticket = tickets.firstOrNull()
        if (ticket == null) {
            notice = "Chưa có local ticket để sync."
            return
        }
        sendToDesktop("[Sync Android ticket ${ticket.id}] ${ticket.request}\n\nLocal reply:\n${ticket.reply}", "android_mobile_sync", scope)
    }

    fun addKnowledge(name: String, content: String, source: String = "android_manual") {
        if (content.trim().isEmpty()) return
        val doc = KnowledgeDoc(
            id = "kb-${System.currentTimeMillis()}",
            name = name.trim().ifBlank { "tri-thuc-android.md" },
            content = content.trim(),
            source = source,
        )
        knowledgeDocs.add(0, doc)
        persistKnowledge()
        notice = "Đã lưu tri thức local: ${doc.name}."
    }

    fun addEmailRule(instruction: String) {
        val rule = buildEmailRule(instruction)
        emailRules.add(0, rule)
        persistRules()
        notice = "Đã thêm Email AI Rule."
    }

    fun ingestEmail(from: String, subject: String, body: String) {
        if (from.isBlank() || subject.isBlank() || body.isBlank()) return
        val folded = fold("$from $subject $body")
        val matched = emailRules.filter { rule ->
            rule.condition.split(" ").any { it.length >= 3 && folded.contains(it) }
        }
        val actions = matched.flatMap { it.actions }.ifEmpty {
            fallbackEmailActions(folded)
        }
        val labels = actions.mapNotNull { action -> action.removePrefix("label:").takeIf { action.startsWith("label:") } }.ifEmpty { listOf("Review") }
        val status = when {
            actions.contains("block_cold") -> "blocked_cold"
            actions.contains("archive") -> "archived"
            actions.contains("owner_review") && !actions.contains("draft_reply") -> "owner_review"
            else -> "reply_zero"
        }
        val draft = if (actions.contains("draft_reply") || status == "reply_zero") {
            buildEmailDraft(from, subject, body, actions)
        } else {
            ""
        }
        val thread = EmailThread(
            id = "email-${System.currentTimeMillis()}",
            from = from.trim(),
            subject = subject.trim(),
            body = body.trim(),
            status = status,
            labels = labels,
            draft = draft,
        )
        emailThreads.add(0, thread)
        persistEmailThreads()
        val ticket = routeLocal("Email từ $from\nChủ đề: $subject\n$body", "android_email").copy(
            title = subject.take(60),
            status = if (draft.isNotBlank()) "waiting_approval" else status,
            approvalRequired = draft.isNotBlank(),
            nextAction = if (draft.isNotBlank()) "Duyệt draft email trước khi gửi trong Gmail/Outlook." else "Đã phân loại email.",
            reply = draft.ifBlank { "Email đã được phân loại: ${labels.joinToString()}." },
        )
        appendTicket(ticket)
        notice = "Đã đưa email vào Reply Zero."
    }

    fun saveEmailKnowledge(thread: EmailThread) {
        addKnowledge(
            name = "email-${thread.id}.md",
            content = "From: ${thread.from}\nSubject: ${thread.subject}\nLabels: ${thread.labels.joinToString()}\n\n${thread.body}",
            source = "android_email",
        )
        val index = emailThreads.indexOfFirst { it.id == thread.id }
        if (index >= 0) {
            emailThreads[index] = thread.copy(knowledgeSaved = true, status = "knowledge_saved")
            persistEmailThreads()
        }
    }

    fun saveTranslationKnowledge() {
        val transcript = translator.transcriptMarkdown()
        if (transcript.isBlank()) {
            notice = "Chưa có transcript để lưu tri thức."
            return
        }
        addKnowledge(
            name = "phien-dich-${System.currentTimeMillis()}.md",
            content = transcript,
            source = "android_translation",
        )
        val ticket = routeLocal("Tóm tắt và khai thác phiên dịch:\n$transcript", "android_translation").copy(
            title = "Phiên dịch ${translator.provider.label} → ${translator.targetLanguage}",
            agent = "mama",
            status = "planned",
            approvalRequired = false,
            nextAction = "Dùng transcript để tạo tóm tắt, follow-up, FAQ hoặc bổ sung RAG.",
            reply = "Đã lưu transcript phiên dịch vào tri thức local. Mama có thể dùng nội dung này để tóm tắt, tạo việc và phản hồi khách chính xác hơn.",
        )
        appendTicket(ticket)
        notice = "Đã lưu phiên dịch vào Knowledge và Mission."
    }

    fun markTicket(ticket: MobileTicket, status: String) {
        val index = tickets.indexOfFirst { it.id == ticket.id }
        if (index >= 0) {
            tickets[index] = ticket.copy(status = status)
            persistTickets()
        }
    }

    fun openUrl(url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
    }

    fun openPackage(packageName: String, fallbackUrl: String) {
        val launchIntent = context.packageManager.getLaunchIntentForPackage(packageName)
        if (launchIntent != null) {
            launchIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(launchIntent)
        } else {
            openUrl(fallbackUrl)
        }
    }

    fun openAppSettings() {
        context.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
            data = Uri.parse("package:${context.packageName}")
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        })
    }

    fun openAccessibilitySettings() {
        context.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        })
    }

    fun filteredKnowledge(): List<KnowledgeDoc> {
        val q = fold(query)
        if (q.isBlank()) return knowledgeDocs
        return knowledgeDocs.filter { fold("${it.name} ${it.content} ${it.source}").contains(q) }
    }

    private fun routeLocal(rawRequest: String, source: String): MobileTicket {
        val request = rawRequest.trim().ifBlank { "Yêu cầu trống từ Android." }
        val normalized = fold(request)
        val agent = when {
            hasAny(normalized, "caption", "bai dang", "dang bai", "content", "facebook", "zalo oa") -> "content"
            hasAny(normalized, "don hang", "ton kho", "sku", "ecom", "san pham", "gio hang") -> "ops"
            hasAny(normalized, "phi ship", "bao hanh", "doi tra", "khieu nai", "khach hoi", "support") -> "support"
            hasAny(normalized, "gia", "bao gia", "chot don", "sale", "khuyen mai") -> "sales"
            hasAny(normalized, "email", "gmail", "outlook", "newsletter") -> "email"
            else -> "mama"
        }
        val approvalRequired = hasAny(
            normalized,
            "dang", "gui khach", "chot", "giam gia", "hoan tien", "xoa", "hop dong", "phap ly", "chuyen khoan"
        )
        val sensitive = hasAny(normalized, "gia", "phi ship", "ton kho", "bao hanh", "doi tra", "hoan tien")
        val hasKnowledge = knowledgeDocs.any { doc -> fold(doc.content).containsAny(listOf("gia", "phi ship", "bao hanh", "doi tra", "san pham")) }
        val stopGate = sensitive && !hasKnowledge
        val status = when {
            stopGate -> "blocked"
            approvalRequired -> "waiting_approval"
            else -> "planned"
        }
        val nextAction = when {
            stopGate -> "Bổ sung RAG: bảng giá, phí ship, tồn kho hoặc chính sách liên quan."
            approvalRequired -> "Soạn draft và chờ chủ doanh nghiệp duyệt trước khi gửi/đăng/chốt."
            agent == "email" -> "Phân loại email, tạo draft nếu cần, không tự gửi."
            agent == "content" -> "Soạn draft nội dung và chọn kênh đăng qua browser/app."
            agent == "sales" -> "Tư vấn theo RAG, nếu thiếu dữ liệu thì hỏi lại."
            else -> "Đưa vào queue để Mama theo dõi."
        }
        val reply = buildLocalReply(agent, request, status, nextAction)
        return MobileTicket(
            id = "and-${System.currentTimeMillis()}",
            title = request.lineSequence().firstOrNull()?.take(64) ?: "Android ticket",
            request = request,
            source = source,
            agent = agent,
            status = status,
            priority = if (hasAny(normalized, "gap", "ngay", "khan", "khieu nai")) "high" else "normal",
            approvalRequired = approvalRequired || stopGate,
            nextAction = nextAction,
            reply = reply,
        )
    }

    private fun buildLocalReply(agent: String, request: String, status: String, nextAction: String): String {
        val gate = if (status == "blocked") {
            "\nStop Gate: thiếu tri thức doanh nghiệp nên Mama không tự đoán giá/chính sách."
        } else {
            ""
        }
        return """
            Mama Mobile đã tạo ticket.
            Agent: $agent
            Trạng thái: $status$gate
            Next: $nextAction

            Draft:
            Mình đã nhận thông tin. Mình sẽ kiểm tra đúng sản phẩm, giá, phí ship, tồn kho và chính sách trước khi phản hồi khách. Nếu cần gửi/đăng/chốt ra ngoài, chủ doanh nghiệp sẽ duyệt trước.

            Yêu cầu gốc:
            $request
        """.trimIndent()
    }

    private fun appendTicket(ticket: MobileTicket) {
        tickets.add(0, ticket)
        while (tickets.size > 80) tickets.removeAt(tickets.lastIndex)
        persistTickets()
    }

    private fun loadTickets() {
        val arr = JSONArray(prefs.getString("tickets", "[]"))
        tickets.clear()
        for (idx in 0 until arr.length()) {
            val obj = arr.getJSONObject(idx)
            tickets.add(
                MobileTicket(
                    id = obj.optString("id"),
                    title = obj.optString("title"),
                    request = obj.optString("request"),
                    source = obj.optString("source"),
                    agent = obj.optString("agent"),
                    status = obj.optString("status"),
                    priority = obj.optString("priority"),
                    approvalRequired = obj.optBoolean("approval_required"),
                    nextAction = obj.optString("next_action"),
                    reply = obj.optString("reply"),
                    createdAt = obj.optLong("created_at"),
                )
            )
        }
    }

    private fun persistTickets() {
        val arr = JSONArray()
        tickets.forEach { ticket ->
            arr.put(JSONObject()
                .put("id", ticket.id)
                .put("title", ticket.title)
                .put("request", ticket.request)
                .put("source", ticket.source)
                .put("agent", ticket.agent)
                .put("status", ticket.status)
                .put("priority", ticket.priority)
                .put("approval_required", ticket.approvalRequired)
                .put("next_action", ticket.nextAction)
                .put("reply", ticket.reply)
                .put("created_at", ticket.createdAt))
        }
        prefs.edit().putString("tickets", arr.toString()).apply()
    }

    private fun loadKnowledge() {
        val arr = JSONArray(prefs.getString("knowledge", "[]"))
        knowledgeDocs.clear()
        for (idx in 0 until arr.length()) {
            val obj = arr.getJSONObject(idx)
            knowledgeDocs.add(
                KnowledgeDoc(
                    id = obj.optString("id"),
                    name = obj.optString("name"),
                    content = obj.optString("content"),
                    source = obj.optString("source"),
                    createdAt = obj.optLong("created_at"),
                )
            )
        }
    }

    private fun persistKnowledge() {
        val arr = JSONArray()
        knowledgeDocs.forEach { doc ->
            arr.put(JSONObject()
                .put("id", doc.id)
                .put("name", doc.name)
                .put("content", doc.content)
                .put("source", doc.source)
                .put("created_at", doc.createdAt))
        }
        prefs.edit().putString("knowledge", arr.toString()).apply()
    }

    private fun loadRules(): List<EmailRule> {
        val raw = prefs.getString("email_rules", null)
        if (raw.isNullOrBlank()) return defaultEmailRules()
        val arr = JSONArray(raw)
        return (0 until arr.length()).map { idx ->
            val obj = arr.getJSONObject(idx)
            EmailRule(
                id = obj.optString("id"),
                instruction = obj.optString("instruction"),
                condition = obj.optString("condition"),
                actions = obj.optJSONArray("actions")?.let { actions ->
                    (0 until actions.length()).map { actions.optString(it) }
                } ?: emptyList(),
            )
        }
    }

    private fun persistRules() {
        val arr = JSONArray()
        emailRules.forEach { rule ->
            arr.put(JSONObject()
                .put("id", rule.id)
                .put("instruction", rule.instruction)
                .put("condition", rule.condition)
                .put("actions", JSONArray(rule.actions)))
        }
        prefs.edit().putString("email_rules", arr.toString()).apply()
    }

    private fun loadEmailThreads() {
        val arr = JSONArray(prefs.getString("email_threads", "[]"))
        emailThreads.clear()
        for (idx in 0 until arr.length()) {
            val obj = arr.getJSONObject(idx)
            val labels = obj.optJSONArray("labels")?.let { value ->
                (0 until value.length()).map { value.optString(it) }
            } ?: emptyList()
            emailThreads.add(
                EmailThread(
                    id = obj.optString("id"),
                    from = obj.optString("from"),
                    subject = obj.optString("subject"),
                    body = obj.optString("body"),
                    status = obj.optString("status"),
                    labels = labels,
                    draft = obj.optString("draft"),
                    knowledgeSaved = obj.optBoolean("knowledge_saved"),
                    createdAt = obj.optLong("created_at"),
                )
            )
        }
    }

    private fun persistEmailThreads() {
        val arr = JSONArray()
        emailThreads.forEach { thread ->
            arr.put(JSONObject()
                .put("id", thread.id)
                .put("from", thread.from)
                .put("subject", thread.subject)
                .put("body", thread.body)
                .put("status", thread.status)
                .put("labels", JSONArray(thread.labels))
                .put("draft", thread.draft)
                .put("knowledge_saved", thread.knowledgeSaved)
                .put("created_at", thread.createdAt))
        }
        prefs.edit().putString("email_threads", arr.toString()).apply()
    }

    private fun buildEmailRule(instruction: String): EmailRule {
        val folded = fold(instruction)
        val pair = when {
            hasAny(folded, "newsletter", "marketing", "quang cao", "unsubscribe") ->
                "newsletter marketing unsubscribe quang cao" to listOf("label:Newsletter", "archive")
            hasAny(folded, "cold", "spam", "seo", "backlink", "agency") ->
                "cold outreach seo agency backlink spam" to listOf("label:Cold", "block_cold")
            hasAny(folded, "nha cung cap", "hoa don", "invoice", "van don", "bao gia") ->
                "nha cung cap supplier hoa don invoice van don bao gia" to listOf("label:Supplier", "save_knowledge", "owner_review")
            hasAny(folded, "khach", "san pham", "dich vu", "gia", "ship", "bao hanh") ->
                "khach hang san pham dich vu gia ship bao hanh" to listOf("label:Customer", "draft_reply", "reply_zero")
            else -> folded.split(" ").take(12).joinToString(" ") to listOf("label:Review", "draft_reply", "owner_review")
        }
        return EmailRule(
            id = "rule-${System.currentTimeMillis()}",
            instruction = instruction.trim(),
            condition = pair.first,
            actions = pair.second,
        )
    }

    private fun defaultEmailRules(): List<EmailRule> = listOf(
        EmailRule("rule-newsletter", "Newsletter/marketing thì gắn nhãn Newsletter và archive.", "newsletter marketing unsubscribe quang cao", listOf("label:Newsletter", "archive")),
        EmailRule("rule-customer", "Khách hỏi sản phẩm, dịch vụ, giá, phí ship hoặc bảo hành thì đưa vào Reply Zero và soạn draft.", "khach san pham dich vu gia ship bao hanh", listOf("label:Customer", "draft_reply", "reply_zero")),
        EmailRule("rule-supplier", "Báo giá, hóa đơn hoặc vận đơn từ nhà cung cấp thì lưu tri thức sau khi duyệt.", "nha cung cap bao gia hoa don van don invoice", listOf("label:Supplier", "save_knowledge", "owner_review")),
        EmailRule("rule-cold", "Cold email không liên quan thì gắn nhãn Cold và chặn khỏi Reply Zero.", "cold outreach spam seo backlink agency", listOf("label:Cold", "block_cold")),
    )

    private fun fallbackEmailActions(folded: String): List<String> = when {
        hasAny(folded, "unsubscribe", "newsletter", "quang cao", "marketing") -> listOf("label:Newsletter", "archive")
        hasAny(folded, "cold", "spam", "backlink", "agency") -> listOf("label:Cold", "block_cold")
        hasAny(folded, "hoa don", "invoice", "van don", "nha cung cap") -> listOf("label:Supplier", "save_knowledge", "owner_review")
        else -> listOf("label:Customer", "draft_reply", "reply_zero")
    }

    private fun buildEmailDraft(from: String, subject: String, body: String, actions: List<String>): String {
        val caution = if (actions.contains("owner_review")) {
            "Em sẽ đối chiếu lại dữ liệu nội bộ trước khi xác nhận chi tiết."
        } else {
            "Em sẽ kiểm tra đúng bảng giá, tồn kho, phí ship và chính sách hiện hành trước khi chốt."
        }
        return """
            Chào anh/chị,

            Cảm ơn anh/chị đã gửi email về "$subject".

            Tóm tắt Mama đang thấy: ${body.compact(220)}

            $caution Nếu cần thêm thông tin, bên em sẽ phản hồi lại ngay trong luồng email này.

            Trân trọng,
            BizClaw Mama

            Draft cho: $from. Vui lòng duyệt trước khi gửi.
        """.trimIndent()
    }
}

private class DesktopMamaClient {
    suspend fun postMama(
        serverUrl: String,
        token: String,
        request: String,
        source: String,
        customerRef: String,
    ): Result<String> = withContext(Dispatchers.IO) {
        runCatching {
            val payload = JSONObject()
                .put("request", request.trim())
                .put("source_channel", source)
                .put("customer_ref", customerRef)
            val response = httpPost("${serverUrl.trim().trimEnd('/')}/api/v1/mam-agents/chat", payload, token)
            val json = JSONObject(response)
            if (json.optBoolean("ok")) {
                json.optString("reply")
            } else {
                throw IllegalStateException(json.optString("error", response))
            }
        }
    }

    suspend fun health(serverUrl: String): Result<Int> = withContext(Dispatchers.IO) {
        runCatching {
            val connection = URL("${serverUrl.trim().trimEnd('/')}/health").openConnection() as HttpURLConnection
            connection.connectTimeout = 8_000
            connection.readTimeout = 12_000
            connection.requestMethod = "GET"
            connection.responseCode.also { connection.disconnect() }
        }
    }

    private fun httpPost(url: String, payload: JSONObject, token: String): String {
        val connection = URL(url).openConnection() as HttpURLConnection
        connection.connectTimeout = 10_000
        connection.readTimeout = 60_000
        connection.requestMethod = "POST"
        connection.doOutput = true
        connection.setRequestProperty("Content-Type", "application/json")
        token.trim().takeIf { it.isNotEmpty() }?.let {
            connection.setRequestProperty("Authorization", "Bearer $it")
        }
        OutputStreamWriter(connection.outputStream).use { it.write(payload.toString()) }
        val stream = if (connection.responseCode in 200..299) connection.inputStream else connection.errorStream
        return stream.bufferedReader().use { it.readText() }.also { connection.disconnect() }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BizClawMobileApp(state: MobileMamaState) {
    var tab by remember { mutableStateOf(MobileTab.Chat) }
    val scope = rememberCoroutineScope()

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("BizClaw Mama", fontWeight = FontWeight.Bold)
                        Text(
                            if (state.mode == "desktop") "Kết nối Desktop/Gateway" else "Chạy độc lập trên Android",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                },
                actions = {
                    ModeChip(state.mode, "local") { state.switchMode("local") }
                    Spacer(Modifier.width(6.dp))
                    ModeChip(state.mode, "desktop") { state.switchMode("desktop") }
                    IconButton(onClick = { tab = MobileTab.Bridge }) {
                        Icon(Icons.Default.Settings, contentDescription = "Cấu hình")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f)),
            )
        },
        bottomBar = {
            NavigationBar {
                MobileTab.entries.forEach { item ->
                    NavigationBarItem(
                        selected = tab == item,
                        onClick = { tab = item },
                        icon = { Icon(item.icon, contentDescription = item.title) },
                        label = { Text(item.title, maxLines = 1) },
                    )
                }
            }
        },
    ) { padding ->
        Surface(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize(),
            color = MaterialTheme.colorScheme.background,
        ) {
            when (tab) {
                MobileTab.Chat -> ChatTab(state, scope)
                MobileTab.Mission -> MissionTab(state)
                MobileTab.Channels -> ChannelsTab(state)
                MobileTab.Translate -> TranslationTab(state)
                MobileTab.Knowledge -> KnowledgeTab(state)
                MobileTab.Bridge -> BridgeTab(state, scope)
            }
        }
    }
}

@Composable
private fun ModeChip(current: String, value: String, onClick: () -> Unit) {
    FilterChip(
        selected = current == value,
        onClick = onClick,
        label = { Text(if (value == "local") "Local" else "Desktop") },
        leadingIcon = {
            Icon(
                if (value == "local") Icons.Default.PhoneAndroid else Icons.Default.Computer,
                contentDescription = null,
                modifier = Modifier.size(16.dp),
            )
        },
    )
}

@Composable
private fun ChatTab(state: MobileMamaState, scope: CoroutineScope) {
    var input by remember(state.pendingInput) { mutableStateOf(state.pendingInput) }
    Column(Modifier.fillMaxSize()) {
        NoticeBar(state.notice, state.isBusy)
        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = PaddingValues(14.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            items(state.messages, key = { it.id }) { item ->
                ChatBubble(item)
            }
        }
        QuickPrompts { prompt ->
            input = prompt
        }
        Row(
            Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.Bottom,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            OutlinedTextField(
                value = input,
                onValueChange = {
                    input = it
                    state.pendingInput = it
                },
                modifier = Modifier.weight(1f),
                placeholder = { Text("Giao việc cho Mama...") },
                minLines = 1,
                maxLines = 4,
            )
            Button(
                enabled = input.isNotBlank() && !state.isBusy,
                onClick = { state.submitMama(input, scope) },
                modifier = Modifier.height(56.dp),
            ) {
                Icon(Icons.AutoMirrored.Filled.Send, contentDescription = "Gửi")
            }
        }
    }
}

@Composable
private fun QuickPrompts(onPick: (String) -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        listOf(
            "Khách hỏi phí ship Hà Nội",
            "Soạn bài bán combo cuối tuần",
            "Kiểm tra email báo giá nhà cung cấp",
        ).forEach { prompt ->
            AssistChip(onClick = { onPick(prompt) }, label = { Text(prompt, maxLines = 1) })
        }
    }
}

@Composable
private fun ChatBubble(item: ChatItem) {
    val isUser = item.role == "user"
    Row(Modifier.fillMaxWidth(), horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start) {
        Surface(
            shape = RoundedCornerShape(18.dp),
            color = if (isUser) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceVariant,
            tonalElevation = 3.dp,
            modifier = Modifier.fillMaxWidth(if (isUser) 0.86f else 0.94f),
        ) {
            Text(
                text = item.content,
                modifier = Modifier.padding(14.dp),
                color = if (isUser) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodyMedium,
            )
        }
    }
}

@Composable
private fun MissionTab(state: MobileMamaState) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            MetricGrid(
                listOf(
                    "Ticket" to state.tickets.size.toString(),
                    "Chờ duyệt" to state.tickets.count { it.status == "waiting_approval" }.toString(),
                    "Blocked" to state.tickets.count { it.status == "blocked" }.toString(),
                    "Email" to state.emailThreads.size.toString(),
                )
            )
        }
        items(state.tickets, key = { it.id }) { ticket ->
            TicketCard(ticket, state)
        }
        if (state.tickets.isEmpty()) {
            item { EmptyCard("Chưa có ticket. Giao việc ở tab Mama hoặc share nội dung vào app.") }
        }
    }
}

@Composable
private fun TicketCard(ticket: MobileTicket, state: MobileMamaState) {
    ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface)) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(ticket.title, modifier = Modifier.weight(1f), fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
                StatusPill(ticket.status)
            }
            Text("${ticket.agent} · ${ticket.source} · ${ticket.priority}", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
            Text(ticket.request.compact(180), style = MaterialTheme.typography.bodyMedium)
            Text("Next: ${ticket.nextAction}", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { state.markTicket(ticket, "done") }) {
                    Icon(Icons.Default.CheckCircle, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Xong")
                }
                OutlinedButton(onClick = { state.markTicket(ticket, "waiting_approval") }) {
                    Text("Chờ duyệt")
                }
            }
        }
    }
}

@Composable
private fun ChannelsTab(state: MobileMamaState) {
    var rule by remember { mutableStateOf("") }
    var from by remember { mutableStateOf("khach@example.com") }
    var subject by remember { mutableStateOf("Khách hỏi phí ship Hà Nội") }
    var body by remember { mutableStateOf("Mình muốn mua sản phẩm này, phí ship Hà Nội bao nhiêu và có bảo hành không?") }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            MetricGrid(
                listOf(
                    "Reply Zero" to state.emailThreads.count { it.status == "reply_zero" }.toString(),
                    "Draft" to state.emailThreads.count { it.draft.isNotBlank() }.toString(),
                    "Archive" to state.emailThreads.count { it.status == "archived" }.toString(),
                    "RAG" to state.emailThreads.count { it.knowledgeSaved }.toString(),
                )
            )
        }
        item {
            SectionCard("Kênh trình duyệt/app") {
                ChannelButtons(state)
            }
        }
        item {
            SectionCard("Email Assistant") {
                Text("AI Rules", fontWeight = FontWeight.SemiBold)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = rule,
                        onValueChange = { rule = it },
                        modifier = Modifier.weight(1f),
                        placeholder = { Text("Nếu newsletter thì archive") },
                        minLines = 1,
                    )
                    Button(onClick = {
                        if (rule.isNotBlank()) {
                            state.addEmailRule(rule)
                            rule = ""
                        }
                    }) {
                        Icon(Icons.Default.Add, contentDescription = "Thêm")
                    }
                }
                state.emailRules.take(4).forEach { item ->
                    Text("• ${item.instruction}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Divider(Modifier.padding(vertical = 8.dp))
                OutlinedTextField(from, { from = it }, label = { Text("From") }, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(subject, { subject = it }, label = { Text("Subject") }, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(body, { body = it }, label = { Text("Body") }, modifier = Modifier.fillMaxWidth(), minLines = 4)
                Button(onClick = { state.ingestEmail(from, subject, body) }, modifier = Modifier.fillMaxWidth()) {
                    Text("Đưa email cho Mama")
                }
            }
        }
        items(state.emailThreads, key = { it.id }) { thread ->
            EmailThreadCard(thread, state)
        }
    }
}

@Composable
private fun ChannelButtons(state: MobileMamaState) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            ChannelButton("Gmail", Icons.Default.Email, Modifier.weight(1f)) { state.openPackage("com.google.android.gm", "https://mail.google.com/") }
            ChannelButton("Outlook", Icons.Default.Email, Modifier.weight(1f)) { state.openPackage("com.microsoft.office.outlook", "https://outlook.office.com/mail/") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            ChannelButton("Zalo", Icons.Default.OpenInNew, Modifier.weight(1f)) { state.openPackage("com.zing.zalo", "https://chat.zalo.me/") }
            ChannelButton("Facebook", Icons.Default.OpenInNew, Modifier.weight(1f)) { state.openPackage("com.facebook.katana", "https://www.facebook.com/") }
        }
    }
}

@Composable
private fun ChannelButton(label: String, icon: ImageVector, modifier: Modifier = Modifier, onClick: () -> Unit) {
    OutlinedButton(onClick = onClick, modifier = modifier) {
        Icon(icon, contentDescription = null, modifier = Modifier.size(16.dp))
        Spacer(Modifier.width(6.dp))
        Text(label)
    }
}

@Composable
private fun EmailThreadCard(thread: EmailThread, state: MobileMamaState) {
    ElevatedCard {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(thread.subject, modifier = Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                StatusPill(thread.status)
            }
            Text("${thread.from} · ${thread.labels.joinToString()}", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
            Text(thread.body.compact(180))
            if (thread.draft.isNotBlank()) {
                Surface(shape = RoundedCornerShape(12.dp), color = MaterialTheme.colorScheme.surfaceVariant) {
                    Text(thread.draft, modifier = Modifier.padding(12.dp), style = MaterialTheme.typography.bodySmall)
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { state.saveEmailKnowledge(thread) }, enabled = !thread.knowledgeSaved) {
                    Text(if (thread.knowledgeSaved) "Đã lưu RAG" else "Lưu RAG")
                }
                OutlinedButton(onClick = { state.openPackage("com.google.android.gm", "https://mail.google.com/") }) {
                    Text("Mở Gmail")
                }
            }
        }
    }
}

@Composable
private fun TranslationTab(state: MobileMamaState) {
    val translator = state.translator
    val context = LocalContext.current
    val permissionLauncher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) translator.start() else translator.permissionDenied()
    }

    fun startWithPermission() {
        if (context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) {
            translator.start()
        } else {
            permissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
        }
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            MetricGrid(
                listOf(
                    "Trạng thái" to translator.status.label,
                    "Đoạn dịch" to translator.rows.size.toString(),
                    "Provider" to translator.provider.label,
                    "Target" to translator.targetLanguage,
                )
            )
        }
        item {
            SectionCard("Phiên dịch realtime") {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TranslationProvider.entries.forEach { item ->
                        FilterChip(
                            selected = translator.provider == item,
                            onClick = {
                                if (!translator.isRunning) {
                                    translator.provider = item
                                    translator.saveSettings()
                                }
                            },
                            label = { Text(item.label) },
                        )
                    }
                }
                OutlinedTextField(
                    value = translator.targetLanguage,
                    onValueChange = {
                        translator.targetLanguage = it.trim().ifBlank { "vi" }
                        translator.saveSettings()
                    },
                    label = { Text("Ngôn ngữ đích") },
                    placeholder = { Text("vi, en, ja, ko...") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    enabled = !translator.isRunning,
                )
                OutlinedTextField(
                    value = translator.activeKey(),
                    onValueChange = {
                        translator.updateActiveKey(it)
                        translator.saveSettings()
                    },
                    label = { Text("${translator.provider.label} API key") },
                    placeholder = { Text("Dán key của nhà cung cấp") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    enabled = !translator.isRunning,
                    visualTransformation = PasswordVisualTransformation(),
                )
                translator.errorMessage?.let { message ->
                    Surface(shape = RoundedCornerShape(12.dp), color = Color(0xFFFF5D6C).copy(alpha = 0.14f)) {
                        Text(message, modifier = Modifier.padding(12.dp), color = Color(0xFFFF9CA5))
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(
                        onClick = { if (translator.isRunning) translator.stop() else startWithPermission() },
                        modifier = Modifier.weight(1f),
                    ) {
                        Text(if (translator.isRunning) "Dừng" else "Bắt đầu nghe")
                    }
                    OutlinedButton(
                        onClick = { translator.clear() },
                        enabled = !translator.isRunning && translator.rows.isNotEmpty(),
                        modifier = Modifier.weight(1f),
                    ) {
                        Text("Xóa")
                    }
                }
                OutlinedButton(
                    onClick = { state.saveTranslationKnowledge() },
                    enabled = translator.rows.isNotEmpty(),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Lưu transcript vào Knowledge")
                }
            }
        }
        if (translator.sourceProvisional.isNotBlank() || translator.targetProvisional.isNotBlank()) {
            item {
                TranslationLineCard(
                    source = translator.sourceProvisional,
                    translation = translator.targetProvisional,
                    status = "live",
                )
            }
        }
        items(translator.rows, key = { it.id }) { row ->
            TranslationLineCard(
                source = row.source,
                translation = row.translation,
                status = row.provider.label,
            )
        }
        if (translator.rows.isEmpty() && translator.sourceProvisional.isBlank() && translator.targetProvisional.isBlank()) {
            item { EmptyCard("Chưa có transcript. Bấm Bắt đầu nghe để Mama phiên dịch realtime và lưu kết quả vào tri thức.") }
        }
    }
}

@Composable
private fun TranslationLineCard(source: String, translation: String, status: String) {
    ElevatedCard {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Phiên dịch", modifier = Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                StatusPill(status)
            }
            if (source.isNotBlank()) {
                Text("Gốc", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
                Text(source, style = MaterialTheme.typography.bodyMedium)
            }
            if (translation.isNotBlank()) {
                Text("Dịch", color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
                Text(translation, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            }
        }
    }
}

@Composable
private fun KnowledgeTab(state: MobileMamaState) {
    var name by remember { mutableStateOf("chinh-sach-ban-hang.md") }
    var content by remember { mutableStateOf("") }
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            SectionCard("Nạp tri thức") {
                OutlinedTextField(name, { name = it }, label = { Text("Tên tài liệu") }, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(content, { content = it }, label = { Text("FAQ, bảng giá, phí ship, bảo hành...") }, modifier = Modifier.fillMaxWidth(), minLines = 6)
                Button(onClick = {
                    state.addKnowledge(name, content)
                    content = ""
                }, modifier = Modifier.fillMaxWidth()) {
                    Text("Lưu tri thức local")
                }
            }
        }
        item {
            OutlinedTextField(
                value = state.query,
                onValueChange = { state.query = it },
                label = { Text("Tìm trong tri thức") },
                modifier = Modifier.fillMaxWidth(),
            )
        }
        items(state.filteredKnowledge(), key = { it.id }) { doc ->
            ElevatedCard {
                Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(doc.name, fontWeight = FontWeight.SemiBold)
                    Text(doc.source, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
                    Text(doc.content.compact(260), style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
        if (state.knowledgeDocs.isEmpty()) {
            item { EmptyCard("Chưa có tri thức. Nạp chính sách/FAQ/sản phẩm để Mama bớt ảo giác.") }
        }
    }
}

@Composable
private fun BridgeTab(state: MobileMamaState, scope: CoroutineScope) {
    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        NoticeBar(state.notice, state.isBusy)
        SectionCard("Kết nối Desktop") {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(selected = state.mode == "local", onClick = { state.switchMode("local") }, label = { Text("Độc lập") })
                FilterChip(selected = state.mode == "desktop", onClick = { state.switchMode("desktop") }, label = { Text("Gắn Desktop") })
            }
            OutlinedTextField(state.serverUrl, { state.serverUrl = it }, label = { Text("Desktop/Gateway URL") }, modifier = Modifier.fillMaxWidth())
            OutlinedTextField(state.apiKey, { state.apiKey = it }, label = { Text("API key / Bearer token") }, modifier = Modifier.fillMaxWidth())
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { state.saveBridgeSettings() }, modifier = Modifier.weight(1f)) {
                    Text("Lưu")
                }
                OutlinedButton(onClick = { state.healthCheck(scope) }, modifier = Modifier.weight(1f)) {
                    Icon(Icons.Default.CloudDone, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Ping")
                }
            }
            OutlinedButton(onClick = { state.syncLatest(scope) }, modifier = Modifier.fillMaxWidth()) {
                Text("Sync ticket mới nhất lên Desktop")
            }
        }
        SectionCard("Quyền Android") {
            Text("Android dùng app/browser thật: Gmail, Outlook, Zalo, Facebook. Gửi/đăng vẫn phải có người duyệt.", color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { state.openAppSettings() }, modifier = Modifier.weight(1f)) {
                    Text("App settings")
                }
                OutlinedButton(onClick = { state.openAccessibilitySettings() }, modifier = Modifier.weight(1f)) {
                    Text("Accessibility")
                }
            }
        }
    }
}

@Composable
private fun MetricGrid(items: List<Pair<String, String>>) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items.take(2).forEach { MetricCard(it.first, it.second, Modifier.weight(1f)) }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items.drop(2).take(2).forEach { MetricCard(it.first, it.second, Modifier.weight(1f)) }
        }
    }
}

@Composable
private fun MetricCard(label: String, value: String, modifier: Modifier = Modifier) {
    Card(modifier = modifier, colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
            Text(value, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        }
    }
}

@Composable
private fun SectionCard(title: String, content: @Composable ColumnScope.() -> Unit) {
    ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface)) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            content()
        }
    }
}

@Composable
private fun NoticeBar(text: String, busy: Boolean) {
    Surface(color = MaterialTheme.colorScheme.surfaceVariant) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 14.dp, vertical = 9.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                Modifier
                    .size(8.dp)
                    .clip(CircleShape)
                    .background(if (busy) Color(0xFFFFB15F) else Color(0xFF32D583))
            )
            Spacer(Modifier.width(8.dp))
            Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
private fun StatusPill(status: String) {
    val color = when (status) {
        "done", "synced_desktop", "knowledge_saved" -> Color(0xFF32D583)
        "blocked", "blocked_cold" -> Color(0xFFFF5D6C)
        "waiting_approval", "reply_zero" -> Color(0xFFFFB15F)
        else -> MaterialTheme.colorScheme.primary
    }
    Surface(shape = RoundedCornerShape(999.dp), color = color.copy(alpha = 0.16f)) {
        Text(status, modifier = Modifier.padding(horizontal = 9.dp, vertical = 4.dp), color = color, style = MaterialTheme.typography.labelSmall)
    }
}

@Composable
private fun EmptyCard(text: String) {
    Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)) {
        Text(text, modifier = Modifier.padding(16.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun BizClawMobileTheme(content: @Composable () -> Unit) {
    val colors = androidx.compose.material3.darkColorScheme(
        primary = Color(0xFF69A9FF),
        secondary = Color(0xFF32D583),
        tertiary = Color(0xFFE69CFF),
        background = Color(0xFF080A0F),
        surface = Color(0xFF111827),
        surfaceVariant = Color(0xFF1F2937),
        onPrimary = Color.White,
        onSecondary = Color(0xFF05140D),
        onBackground = Color(0xFFF4F7FB),
        onSurface = Color(0xFFF4F7FB),
        onSurfaceVariant = Color(0xFFA7B0C0),
    )
    MaterialTheme(colorScheme = colors) {
        Box(
            Modifier
                .fillMaxSize()
                .background(
                    Brush.verticalGradient(
                        listOf(Color(0xFF132033), Color(0xFF080A0F)),
                    )
                )
        ) {
            content()
        }
    }
}

private fun hasAny(value: String, vararg needles: String): Boolean =
    needles.any { value.contains(it) }

private fun String.containsAny(values: List<String>): Boolean =
    values.any { contains(it) }

private fun String.compact(limit: Int): String {
    val value = trim().replace(Regex("\\s+"), " ")
    return if (value.length <= limit) value else value.take(limit) + "..."
}

private fun fold(value: String): String =
    value.lowercase(Locale.ROOT)
        .replace(Regex("[àáạảãâầấậẩẫăằắặẳẵ]"), "a")
        .replace(Regex("[èéẹẻẽêềếệểễ]"), "e")
        .replace(Regex("[ìíịỉĩ]"), "i")
        .replace(Regex("[òóọỏõôồốộổỗơờớợởỡ]"), "o")
        .replace(Regex("[ùúụủũưừứựửữ]"), "u")
        .replace(Regex("[ỳýỵỷỹ]"), "y")
        .replace("đ", "d")
