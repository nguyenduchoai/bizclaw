package vn.bizclaw.agent.agent

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonPrimitive
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.data.Channel
import vn.bizclaw.agent.data.Order
import vn.bizclaw.agent.llm.ToolCall
import vn.bizclaw.agent.llm.ToolSpec

/**
 * The function schemas exposed to tool-capable models.
 *
 * Separate from [AgentTools] so they can be validated without an Android context: a
 * malformed schema is rejected by the provider at request time, which would break every
 * reply rather than one.
 */
object AgentToolSpecs {

    const val LOOKUP_PRODUCT = "tra_cuu_san_pham"
    const val LOOKUP_POLICY = "tra_cuu_chinh_sach"
    const val CREATE_ORDER = "tao_don_hang"

    val ALL: List<ToolSpec> = listOf(
        ToolSpec(
            name = LOOKUP_PRODUCT,
            description = "Tra gia va ton kho cua mot san pham. Dung bat cu khi nao khach hoi " +
                "gia, con hang khong, hoac nhac ten san pham. Khong duoc tu doan gia.",
            parameters = """
                {"type":"object",
                 "properties":{"ten_san_pham":{"type":"string","description":"Ten khach nhac toi"}},
                 "required":["ten_san_pham"],
                 "additionalProperties":false}
            """.trimIndent(),
        ),
        ToolSpec(
            name = LOOKUP_POLICY,
            description = "Tra thong tin cua hang: phi ship, bao hanh, doi tra, gio mo cua, " +
                "khuyen mai. Dung khi khach hoi bat ky dieu gi khong co trong bang gia.",
            parameters = """
                {"type":"object",
                 "properties":{"cau_hoi":{"type":"string","description":"Cau hoi cua khach"}},
                 "required":["cau_hoi"],
                 "additionalProperties":false}
            """.trimIndent(),
        ),
        ToolSpec(
            name = CREATE_ORDER,
            description = "Tao don hang nhap khi khach da cung cap du ten, so dien thoai, dia " +
                "chi va san pham. Thieu bat ky muc nao thi hoi khach truoc, dung goi tool.",
            parameters = """
                {"type":"object",
                 "properties":{
                   "ten_khach":{"type":"string"},
                   "so_dien_thoai":{"type":"string"},
                   "dia_chi":{"type":"string"},
                   "ten_san_pham":{"type":"string"},
                   "so_luong":{"type":"integer"}},
                 "required":["ten_khach","so_dien_thoai","dia_chi","ten_san_pham","so_luong"],
                 "additionalProperties":false}
            """.trimIndent(),
        ),
    )
}

/**
 * Executes the tools a model asks for.
 *
 * Everything the agent could otherwise invent - prices, stock, policy text, order
 * totals - is behind a tool, so the app supplies the number and the model only decides
 * when to ask for it. Creating an order is the one side effect, and it writes a draft
 * the owner still has to confirm.
 */
class AgentTools(
    private val app: BizClawApp,
    private val channel: Channel,
    private val senderName: String,
    private val exchangeId: String,
) {
    /** Orders this turn produced. The caller persists them; tools never do it directly. */
    val createdOrders = mutableListOf<Order>()

    val specs: List<ToolSpec> get() = AgentToolSpecs.ALL

    suspend fun dispatch(call: ToolCall): String = when (call.name) {
        AgentToolSpecs.LOOKUP_PRODUCT -> lookupProduct(call.arguments)
        AgentToolSpecs.LOOKUP_POLICY -> lookupPolicy(call.arguments)
        AgentToolSpecs.CREATE_ORDER -> createOrder(call.arguments)
        else -> "Không có tool tên ${call.name}."
    }

    private fun lookupProduct(args: JsonObject): String {
        val query = args.str("ten_san_pham")
        val product = app.products.match(query)
            ?: return "Không tìm thấy \"$query\" trong bảng giá. Nói với khách là em kiểm tra " +
                "lại rồi báo sau, tuyệt đối không đoán giá."
        val stock = if (product.inStock) "còn ${product.stock}" else "HẾT HÀNG"
        val note = product.note.takeIf { it.isNotBlank() }?.let { " Ghi chú: $it." }.orEmpty()
        return "${product.name}: ${product.priceLabel}, $stock.$note"
    }

    private fun lookupPolicy(args: JsonObject): String {
        val docs = app.knowledge.relevantTo(args.str("cau_hoi"))
        if (docs.isEmpty()) {
            return "Không có tài liệu nào trả lời được câu này. Nói với khách là em kiểm tra " +
                "lại rồi báo sau, không được tự bịa chính sách."
        }
        return docs.joinToString("\n\n") { "### ${it.title}\n${it.body}" }
    }

    private fun createOrder(args: JsonObject): String {
        val productName = args.str("ten_san_pham")
        val product = app.products.match(productName)
            ?: return "Không tạo được đơn: \"$productName\" không có trong bảng giá."
        if (!product.inStock) {
            return "Không tạo được đơn: ${product.name} đang hết hàng. Báo khách biết."
        }

        val quantity = args.int("so_luong").coerceAtLeast(1)
        val order = Order(
            id = "dh-${System.currentTimeMillis()}",
            customerName = args.str("ten_khach").ifBlank { senderName },
            phone = args.str("so_dien_thoai"),
            address = args.str("dia_chi"),
            productName = product.name,
            productId = product.id,
            quantity = quantity,
            unitPrice = product.price,
            channel = channel,
            sourceExchangeId = exchangeId,
            createdAt = System.currentTimeMillis(),
        )
        createdOrders += order
        return "Đã tạo đơn nháp ${order.id}: ${product.name} × $quantity = ${order.totalLabel}. " +
            "Xác nhận lại đơn với khách và nói shop sẽ liên hệ giao hàng."
    }

    private fun JsonObject.str(key: String): String =
        this[key]?.jsonPrimitive?.content?.trim().orEmpty()

    private fun JsonObject.int(key: String): Int =
        this[key]?.jsonPrimitive?.content?.toIntOrNull() ?: 1
}
