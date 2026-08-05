package vn.bizclaw.agent.agent

import vn.bizclaw.agent.data.Channel
import vn.bizclaw.agent.data.KnowledgeDoc

/** Marker the model emits when the knowledge base cannot answer the question. */
const val ESCALATE_TOKEN = "[CHUYEN_CHU_SHOP]"

/**
 * Turns a customer message into a single prompt for Gemma.
 *
 * The hard constraint is anti-hallucination: at 2B parameters the model will happily
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
    ): String = buildString {
        appendLine("Bạn là trợ lý trả lời tin nhắn khách hàng cho ${businessName.ifBlank { "cửa hàng" }}.")
        appendLine("Vai trò: ${persona.trim()}")
        appendLine()
        appendLine("QUY TẮC BẮT BUỘC:")
        appendLine("1. Chỉ dùng thông tin trong phần THÔNG TIN CỬA HÀNG bên dưới.")
        appendLine("2. Nếu câu hỏi cần dữ liệu không có ở đó (giá, phí ship, tồn kho, bảo hành, thời gian giao, khuyến mãi), KHÔNG được đoán.")
        appendLine("   Trả lời ngắn rằng em sẽ kiểm tra và báo lại, rồi kết thúc bằng đúng ký hiệu $ESCALATE_TOKEN.")
        appendLine("3. Không hứa giảm giá, hoàn tiền, đền bù hay bất kỳ cam kết nào không có trong thông tin cửa hàng.")
        appendLine("4. Viết tiếng Việt tự nhiên, tối đa 3 câu, như đang nhắn tin. Không markdown, không gạch đầu dòng.")
        appendLine("5. Chỉ xuất nội dung tin nhắn trả lời, không giải thích gì thêm.")
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

        appendLine("TIN NHẮN ĐẾN (${channel.label}, từ \"$sender\"):")
        appendLine(message.trim())
        appendLine()
        append("TRẢ LỜI:")
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
