package vn.bizclaw.companion

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL
import java.util.Locale

object CompanionPrefs {
    const val NAME = "bizclaw_companion"
    const val KEY_MODE = "mode"
    const val KEY_SERVER_URL = "server_url"
    const val KEY_TOKEN = "token"
    const val KEY_TICKETS = "tickets"
    const val KEY_LAST_SCREEN = "last_screen"
    const val MODE_LOCAL = "local"
    const val MODE_DESKTOP = "desktop"

    fun open(context: Context) = context.getSharedPreferences(NAME, Context.MODE_PRIVATE)

    fun mode(context: Context): String =
        open(context).getString(KEY_MODE, MODE_LOCAL) ?: MODE_LOCAL
}

data class LocalTicket(
    val id: String,
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

object LocalMama {
    fun route(rawRequest: String, source: String): LocalTicket {
        val request = rawRequest.trim().ifBlank { "Yêu cầu trống từ Android." }
        val normalized = request.lowercase(Locale.ROOT)
        val agent = when {
            hasAny(normalized, "caption", "bài đăng", "đăng bài", "content", "facebook", "zalo oa") -> "content-operator"
            hasAny(normalized, "đơn hàng", "tồn kho", "sku", "ecom", "sản phẩm", "giỏ hàng") -> "ecommerce-operator"
            hasAny(normalized, "phí ship", "bảo hành", "đổi trả", "khiếu nại", "khách hỏi", "support") -> "customer-support"
            hasAny(normalized, "giá", "báo giá", "chốt đơn", "sale", "khuyến mãi") -> "sales-assistant"
            else -> "mama-chief"
        }
        val approvalRequired = hasAny(
            normalized,
            "đăng", "gửi khách", "chốt", "giảm giá", "hoàn tiền", "xóa", "hợp đồng", "pháp lý", "chuyển khoản"
        )
        val priority = if (hasAny(normalized, "gấp", "ngay", "khẩn", "complaint", "khiếu nại")) "high" else "normal"
        val status = if (approvalRequired) "waiting_approval" else "planned"
        val nextAction = when (agent) {
            "content-operator" -> "Soạn draft nội dung, kiểm tra kênh đăng và chờ duyệt trước khi public."
            "ecommerce-operator" -> "Kiểm tra thông tin sản phẩm/đơn hàng; nếu thiếu dữ liệu thì hỏi lại người dùng."
            "customer-support" -> "Trả lời ngắn, đúng chính sách; nếu thiếu nguồn thì hẹn xác minh."
            "sales-assistant" -> "Tư vấn theo nhu cầu, không tự cam kết giá/ưu đãi nếu chưa được duyệt."
            else -> "Tách việc thành ticket nhỏ và chọn skill phù hợp."
        }
        val reply = buildReply(agent, request, approvalRequired, nextAction)
        return LocalTicket(
            id = "and-${System.currentTimeMillis()}",
            request = request,
            source = source,
            agent = agent,
            status = status,
            priority = priority,
            approvalRequired = approvalRequired,
            nextAction = nextAction,
            reply = reply,
        )
    }

    fun appendTicket(context: Context, ticket: LocalTicket) {
        val prefs = CompanionPrefs.open(context)
        val items = JSONArray(prefs.getString(CompanionPrefs.KEY_TICKETS, "[]"))
        val next = JSONArray()
        next.put(toJson(ticket))
        for (idx in 0 until minOf(items.length(), 49)) {
            next.put(items.getJSONObject(idx))
        }
        prefs.edit().putString(CompanionPrefs.KEY_TICKETS, next.toString()).apply()
    }

    fun latestTicket(context: Context): LocalTicket? {
        val items = JSONArray(CompanionPrefs.open(context).getString(CompanionPrefs.KEY_TICKETS, "[]"))
        if (items.length() == 0) return null
        return fromJson(items.getJSONObject(0))
    }

    fun clearTickets(context: Context) {
        CompanionPrefs.open(context).edit().putString(CompanionPrefs.KEY_TICKETS, "[]").apply()
    }

    fun ticketSummary(context: Context): String {
        val items = JSONArray(CompanionPrefs.open(context).getString(CompanionPrefs.KEY_TICKETS, "[]"))
        if (items.length() == 0) return "Chưa có ticket local."
        return (0 until minOf(items.length(), 12)).joinToString("\n\n") { idx ->
            val ticket = fromJson(items.getJSONObject(idx))
            "${ticket.id} | ${ticket.status} | ${ticket.agent}\n${ticket.request.take(140)}\nNext: ${ticket.nextAction}"
        }
    }

    fun postToDesktop(context: Context, request: String, source: String, customerRef: String): String {
        val prefs = CompanionPrefs.open(context)
        val server = prefs.getString(CompanionPrefs.KEY_SERVER_URL, BuildConfig.DEFAULT_SERVER_URL)
            ?.trim()
            ?.trimEnd('/')
            .orEmpty()
        val payload = JSONObject()
            .put("request", request.trim())
            .put("source_channel", source)
            .put("customer_ref", customerRef)
        val response = httpPost("$server/api/v1/mam-agents/chat", payload, prefs.getString(CompanionPrefs.KEY_TOKEN, "").orEmpty())
        val json = JSONObject(response)
        return if (json.optBoolean("ok")) {
            json.optString("reply")
        } else {
            "MAMA error: ${json.optString("error", response)}"
        }
    }

    fun httpGet(url: String): Int {
        return (URL(url).openConnection() as HttpURLConnection).run {
            connectTimeout = 10_000
            readTimeout = 20_000
            requestMethod = "GET"
            responseCode.also { disconnect() }
        }
    }

    fun recordScreen(context: Context, packageName: String, text: String) {
        CompanionPrefs.open(context).edit()
            .putString(CompanionPrefs.KEY_LAST_SCREEN, "$packageName\n${text.take(1000)}")
            .apply()
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

    private fun buildReply(agent: String, request: String, approvalRequired: Boolean, nextAction: String): String {
        val approval = if (approvalRequired) {
            "Cần người dùng duyệt trước khi gửi/đăng/chốt ra bên ngoài."
        } else {
            "Có thể xử lý độc lập trên Android ở mức draft/ticket local."
        }
        return """
            Local Mama đã tạo ticket.
            Agent: $agent
            Approval: $approval
            Next: $nextAction

            Draft trả lời:
            Mình đã nhận thông tin. Để trả lời chính xác, mình sẽ kiểm tra lại chính sách/sản phẩm liên quan và phản hồi ngắn gọn cho khách. Nếu cần cam kết giá, tồn kho, bảo hành hoặc hoàn tiền thì sẽ xin xác nhận trước.

            Yêu cầu gốc:
            $request
        """.trimIndent()
    }

    private fun hasAny(value: String, vararg needles: String): Boolean =
        needles.any { value.contains(it) }

    private fun toJson(ticket: LocalTicket) = JSONObject()
        .put("id", ticket.id)
        .put("request", ticket.request)
        .put("source", ticket.source)
        .put("agent", ticket.agent)
        .put("status", ticket.status)
        .put("priority", ticket.priority)
        .put("approval_required", ticket.approvalRequired)
        .put("next_action", ticket.nextAction)
        .put("reply", ticket.reply)
        .put("created_at", ticket.createdAt)

    private fun fromJson(json: JSONObject) = LocalTicket(
        id = json.optString("id"),
        request = json.optString("request"),
        source = json.optString("source"),
        agent = json.optString("agent"),
        status = json.optString("status"),
        priority = json.optString("priority"),
        approvalRequired = json.optBoolean("approval_required"),
        nextAction = json.optString("next_action"),
        reply = json.optString("reply"),
        createdAt = json.optLong("created_at"),
    )
}
