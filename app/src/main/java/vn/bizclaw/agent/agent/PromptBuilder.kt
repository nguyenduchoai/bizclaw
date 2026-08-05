package vn.bizclaw.agent.agent

import vn.bizclaw.agent.data.Channel
import vn.bizclaw.agent.data.KnowledgeDoc

/** Marker the model emits when the knowledge base cannot answer the question. */
const val ESCALATE_TOKEN = "[CHUYEN_CHU_SHOP]"

/**
 * Turns a customer message into a single prompt for Gemma.
 *
 * The hard constraint is anti-hallucination: at 4B parameters the model will happily
 * invent a price or a warranty term, and that reply goes straight to a paying customer.
 * So facts are supplied inline and the model is told to escalate rather than guess.
 */
object PromptBuilder {

    fun build(
        businessName: String,
        persona: String,
        channel: Channel,
        sender: String,
        message: String,
        knowledge: List<KnowledgeDoc>,
        catalog: String = "",
        orderHint: String? = null,
    ): String = buildString {
        appendLine("Bạn là trợ lý trả lời tin nhắn khách hàng cho ${businessName.ifBlank { "cửa hàng" }}.")
        appendLine("Vai trò: ${persona.trim()}")
        appendLine()
        appendLine("QUY TẮC BẮT BUỘC:")
        appendLine("1. Chỉ dùng thông tin trong phần BẢNG GIÁ và THÔNG TIN CỬA HÀNG bên dưới.")
        appendLine("2. Nếu câu hỏi cần dữ liệu không có ở đó (giá, phí ship, tồn kho, bảo hành, thời gian giao, khuyến mãi), KHÔNG được đoán.")
        appendLine("   Trả lời ngắn rằng em sẽ kiểm tra và báo lại, rồi kết thúc bằng đúng ký hiệu $ESCALATE_TOKEN.")
        appendLine("3. Không hứa giảm giá, hoàn tiền, đền bù hay bất kỳ cam kết nào không có trong thông tin cửa hàng.")
        appendLine("4. Sản phẩm ghi HẾT HÀNG thì phải nói thật là đang hết, không được nhận đơn.")
        appendLine("5. Viết tiếng Việt tự nhiên, tối đa 3 câu, như đang nhắn tin. Không markdown, không gạch đầu dòng.")
        appendLine("6. Chỉ xuất nội dung tin nhắn trả lời, không giải thích gì thêm.")
        appendLine()

        appendLine("BẢNG GIÁ:")
        appendLine(catalog.ifBlank { "(chưa có sản phẩm nào)" })
        appendLine()

        appendLine("THÔNG TIN CỬA HÀNG:")
        if (knowledge.isEmpty()) {
            appendLine("(trống — chưa nạp tài liệu nào)")
        } else {
            knowledge.forEach { doc ->
                appendLine("### ${doc.title}")
                appendLine(doc.body.trim())
            }
        }
        appendLine()

        if (orderHint != null) {
            appendLine("TÌNH HUỐNG: $orderHint")
            appendLine()
        }

        appendLine("TIN NHẮN ĐẾN (${channel.label}, từ \"$sender\"):")
        appendLine(message.trim())
        appendLine()
        append("TRẢ LỜI:")
    }

    /**
     * System prompt for a tool-capable model.
     *
     * Deliberately short on facts and long on when-to-call: the catalog and policies
     * live behind tools, so the only thing this text has to get right is that the model
     * must look things up instead of recalling them.
     */
    fun toolSystem(businessName: String, persona: String, channel: Channel): String = buildString {
        appendLine("Bạn là trợ lý trả lời tin nhắn khách hàng cho ${businessName.ifBlank { "cửa hàng" }} trên ${channel.label}.")
        appendLine("Vai trò: ${persona.trim()}")
        appendLine()
        appendLine("QUY TẮC BẮT BUỘC:")
        appendLine("1. Mọi con số về giá, tồn kho, phí ship, bảo hành, đổi trả PHẢI lấy từ tool.")
        appendLine("   Tuyệt đối không trả lời từ trí nhớ và không đoán.")
        appendLine("2. Tool báo không có dữ liệu → nói với khách là em kiểm tra rồi báo lại,")
        appendLine("   và kết thúc tin nhắn bằng đúng ký hiệu $ESCALATE_TOKEN.")
        appendLine("3. Chỉ gọi tao_don_hang khi đã có đủ tên, số điện thoại, địa chỉ và sản phẩm.")
        appendLine("   Thiếu mục nào thì hỏi khách mục đó, hỏi gọn trong một tin.")
        appendLine("4. Sản phẩm hết hàng thì nói thật là hết, không nhận đơn.")
        appendLine("5. Không hứa giảm giá, hoàn tiền hay cam kết nào không có trong dữ liệu tool trả về.")
        appendLine("6. Viết tiếng Việt tự nhiên, tối đa 3 câu, như đang nhắn tin.")
        appendLine("   Không markdown, không gạch đầu dòng, chỉ xuất nội dung tin nhắn trả lời.")
    }

    /** Wording that steers the reply toward collecting what the order still needs. */
    fun orderHint(missing: List<String>, productName: String, totalLabel: String): String =
        if (missing.isEmpty()) {
            "Khách đang chốt đơn $productName, tổng $totalLabel. Xác nhận lại đơn ngắn gọn " +
                "và báo shop sẽ liên hệ giao hàng. Không hỏi thêm thông tin gì nữa."
        } else {
            "Khách đang muốn đặt hàng nhưng còn thiếu: ${missing.joinToString(", ")}. " +
                "Hỏi khách đúng những mục còn thiếu này, hỏi gọn trong một tin."
        }

    /** Message the owner can send when checking in after a sale. */
    fun followUpPrompt(businessName: String, persona: String, customer: String, product: String) =
        buildString {
            appendLine("Bạn là trợ lý của ${businessName.ifBlank { "cửa hàng" }}.")
            appendLine("Vai trò: ${persona.trim()}")
            appendLine()
            appendLine("Viết một tin nhắn ngắn hỏi thăm khách sau khi mua hàng.")
            appendLine("Khách: $customer. Sản phẩm đã mua: $product.")
            appendLine("Tối đa 2 câu, thân thiện, hỏi khách đã nhận hàng và dùng có ưng không.")
            appendLine("Không khuyến mãi, không chào bán thêm. Chỉ xuất nội dung tin nhắn.")
        }

    /**
     * Splits the escalation marker off the reply text.
     *
     * @return reply without the marker, plus whether the owner needs to step in.
     */
    fun parse(raw: String): Pair<String, Boolean> {
        val needsOwner = raw.contains(ESCALATE_TOKEN)
        val cleaned = raw.replace(ESCALATE_TOKEN, "")
            .trim()
            .removeSurrounding("\"")
            .trim()
        return cleaned to needsOwner
    }
}
