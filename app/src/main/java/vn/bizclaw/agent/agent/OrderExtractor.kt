package vn.bizclaw.agent.agent

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import vn.bizclaw.agent.data.fold

/** What the model believes the customer is ordering, before the app validates it. */
data class ExtractedOrder(
    val hasOrder: Boolean,
    val customerName: String,
    val phone: String,
    val address: String,
    val product: String,
    val quantity: Int,
) {
    /** Fields still needed before this can become a real order. */
    val missing: List<String>
        get() = buildList {
            if (product.isBlank()) add("sản phẩm")
            if (customerName.isBlank()) add("tên")
            if (phone.isBlank()) add("số điện thoại")
            if (address.isBlank()) add("địa chỉ")
        }
}

/**
 * Pulls order details out of a customer message.
 *
 * Runs as a separate constrained-JSON pass rather than as a tool call: at 4B parameters
 * free-form tool invocation is unreliable, while constrained decoding cannot produce
 * anything that fails to parse.
 */
object OrderExtractor {

    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    /** Vietnamese mobile numbers, with or without country code. */
    private val PHONE = Regex("(?:\\+?84|0)\\d{8,10}")

    private val ORDER_WORDS = listOf(
        "chot don", "chot", "dat hang", "dat mua", "lay", "mua", "order",
        "giao den", "ship den", "dia chi", "so luong", "goi cho", "gui cho",
    )

    /** JSON Schema handed to the decoder so the reply is always machine-readable. */
    const val SCHEMA = """
    {
      "type": "object",
      "properties": {
        "has_order": { "type": "boolean" },
        "customer_name": { "type": "string" },
        "phone": { "type": "string" },
        "address": { "type": "string" },
        "product": { "type": "string" },
        "quantity": { "type": "integer" }
      },
      "required": ["has_order","customer_name","phone","address","product","quantity"]
    }
    """

    /**
     * Cheap gate so ordinary questions do not pay for a second inference.
     *
     * Deliberately loose: a false positive costs a few seconds, a false negative loses
     * the order entirely.
     */
    fun looksLikeOrder(message: String): Boolean {
        if (PHONE.containsMatchIn(message)) return true
        val folded = fold(message)
        return ORDER_WORDS.any { folded.contains(it) }
    }

    fun buildPrompt(catalog: String, sender: String, message: String): String = buildString {
        appendLine("Trích thông tin đơn hàng từ tin nhắn của khách. Chỉ xuất JSON.")
        appendLine()
        appendLine("DANH SÁCH SẢN PHẨM:")
        appendLine(catalog)
        appendLine()
        appendLine("QUY TẮC:")
        appendLine("- has_order = true nếu khách đang muốn mua/đặt hàng, false nếu chỉ hỏi thông tin.")
        appendLine("- Trường nào khách chưa nói thì để chuỗi rỗng \"\". KHÔNG được bịa.")
        appendLine("- product phải lấy đúng tên trong danh sách sản phẩm ở trên.")
        appendLine("- quantity mặc định 1 nếu khách không nói rõ.")
        appendLine("- phone giữ nguyên chữ số khách gõ.")
        appendLine()
        appendLine("TÊN HIỂN THỊ CỦA KHÁCH: $sender")
        appendLine("TIN NHẮN:")
        appendLine(message.trim())
    }

    /** Parses the constrained JSON. Returns null when the payload is unusable. */
    fun parse(raw: String): ExtractedOrder? = runCatching {
        val obj = json.parseToJsonElement(raw.trim()).jsonObject
        fun str(key: String) = obj[key]?.jsonPrimitive?.content?.trim().orEmpty()
        ExtractedOrder(
            hasOrder = obj["has_order"]?.jsonPrimitive?.content?.toBooleanStrictOrNull() ?: false,
            customerName = str("customer_name"),
            // Models like to echo a placeholder when the customer gave no number.
            phone = str("phone").takeIf { PHONE.matches(it.replace(" ", "")) }.orEmpty(),
            address = str("address"),
            product = str("product"),
            quantity = obj["quantity"]?.jsonPrimitive?.content?.toIntOrNull()?.coerceAtLeast(1) ?: 1,
        )
    }.getOrNull()
}
