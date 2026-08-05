package vn.bizclaw.agent.agent

import android.content.Context
import android.util.Log
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.data.Channel
import vn.bizclaw.agent.data.Exchange
import vn.bizclaw.agent.data.Order
import vn.bizclaw.agent.data.ReplyState
import vn.bizclaw.agent.llm.LlmProvider

private const val TAG = "ReplyAgent"

/** Outcome of handling one inbound message. */
data class ReplyOutcome(
    val exchange: Exchange,
    /** True when the reply may be delivered without the owner reading it first. */
    val safeToAutoSend: Boolean,
    /** Orders captured from this message. Always start as drafts. */
    val orders: List<Order> = emptyList(),
)

/**
 * Generates a customer reply, and captures an order when the customer is buying.
 *
 * Two strategies, chosen by what the active provider can do. A tool-capable model gets
 * the catalog and policies behind function calls and decides what to look up; the
 * on-device model gets the facts inlined and a separate constrained-JSON pass for order
 * extraction, because free-form tool calls are unreliable at 4B.
 */
class ReplyAgent(private val context: Context) {

    suspend fun handle(
        id: String,
        channel: Channel,
        sender: String,
        message: String,
        receivedAt: Long,
    ): ReplyOutcome {
        val app = BizClawApp.from(context)
        val provider = app.providers.resolve(app.settings.provider)
        val base = Exchange(
            id = id,
            channel = channel,
            sender = sender,
            incoming = message,
            receivedAt = receivedAt,
        )

        if (!provider.isReady()) {
            return ReplyOutcome(
                base.copy(state = ReplyState.FAILED, error = "${provider.kind.label} chưa sẵn sàng"),
                safeToAutoSend = false,
            )
        }

        val startedAt = System.currentTimeMillis()
        val outcome = if (provider.supportsTools) {
            withTools(app, provider, base, channel, sender, message)
        } else {
            withInlinedFacts(app, provider, base, channel, sender, message)
        }
        val latency = System.currentTimeMillis() - startedAt
        return outcome.copy(exchange = outcome.exchange.copy(latencyMs = latency))
    }

    /** Cloud path: the model calls tools for every fact it needs. */
    private suspend fun withTools(
        app: BizClawApp,
        provider: LlmProvider,
        base: Exchange,
        channel: Channel,
        sender: String,
        message: String,
    ): ReplyOutcome {
        val tools = AgentTools(app, channel, sender, base.id)
        val system = PromptBuilder.toolSystem(
            businessName = app.settings.businessName,
            persona = app.settings.persona,
            channel = channel,
        )

        return provider.chat(
            system = system,
            userMessage = "Tin nhắn từ \"$sender\":\n${message.trim()}",
            tools = tools.specs,
            onToolCall = tools::dispatch,
        ).fold(
            onSuccess = { reply ->
                val (text, needsOwner) = PromptBuilder.parse(reply.text)
                if (text.isBlank()) {
                    return@fold ReplyOutcome(
                        base.copy(state = ReplyState.FAILED, error = "Model trả về rỗng"),
                        safeToAutoSend = false,
                        orders = tools.createdOrders,
                    )
                }
                ReplyOutcome(
                    exchange = base.copy(reply = text, state = ReplyState.DRAFT),
                    // An order means money is moving; that always waits for a human,
                    // and so does any answer the model had to escalate.
                    safeToAutoSend = app.settings.autoSend &&
                        !needsOwner &&
                        tools.createdOrders.isEmpty() &&
                        reply.toolsUsed.isNotEmpty(),
                    orders = tools.createdOrders,
                )
            },
            onFailure = { error ->
                Log.e(TAG, "Tool-capable turn failed", error)
                ReplyOutcome(
                    base.copy(state = ReplyState.FAILED, error = error.message ?: "Lỗi gọi model"),
                    safeToAutoSend = false,
                    orders = tools.createdOrders,
                )
            },
        )
    }

    /** On-device path: facts inlined, order extraction as a separate constrained pass. */
    private suspend fun withInlinedFacts(
        app: BizClawApp,
        provider: LlmProvider,
        base: Exchange,
        channel: Channel,
        sender: String,
        message: String,
    ): ReplyOutcome {
        val catalog = app.products.forPrompt()
        val order = if (OrderExtractor.looksLikeOrder(message)) {
            extractOrder(app, provider, base.id, channel, sender, message, catalog)
        } else {
            null
        }

        val knowledge = app.knowledge.relevantTo(message)
        val prompt = PromptBuilder.build(
            businessName = app.settings.businessName,
            persona = app.settings.persona,
            channel = channel,
            sender = sender,
            message = message,
            knowledge = knowledge,
            catalog = catalog,
            orderHint = order?.let {
                PromptBuilder.orderHint(it.missing, it.productName, it.totalLabel)
            },
        )

        return provider.chat(system = "", userMessage = prompt).fold(
            onSuccess = { reply ->
                val (text, needsOwner) = PromptBuilder.parse(reply.text)
                if (text.isBlank()) {
                    return@fold ReplyOutcome(
                        base.copy(state = ReplyState.FAILED, error = "Model trả về rỗng"),
                        safeToAutoSend = false,
                        orders = listOfNotNull(order),
                    )
                }
                ReplyOutcome(
                    exchange = base.copy(reply = text, state = ReplyState.DRAFT),
                    safeToAutoSend = app.settings.autoSend &&
                        !needsOwner &&
                        knowledge.isNotEmpty() &&
                        order?.isComplete != true,
                    orders = listOfNotNull(order),
                )
            },
            onFailure = { error ->
                Log.e(TAG, "On-device turn failed", error)
                ReplyOutcome(
                    base.copy(state = ReplyState.FAILED, error = error.message ?: "Lỗi sinh câu trả lời"),
                    safeToAutoSend = false,
                    orders = listOfNotNull(order),
                )
            },
        )
    }

    private suspend fun extractOrder(
        app: BizClawApp,
        provider: LlmProvider,
        exchangeId: String,
        channel: Channel,
        sender: String,
        message: String,
        catalog: String,
    ): Order? {
        val raw = provider
            .json(OrderExtractor.buildPrompt(catalog, sender, message), OrderExtractor.SCHEMA)
            .onFailure { Log.w(TAG, "Order extraction failed", it) }
            .getOrNull() ?: return null

        val extracted = OrderExtractor.parse(raw) ?: return null
        if (!extracted.hasOrder) return null

        // The model names a product; the app decides it exists and what it costs.
        val product = app.products.match(extracted.product)
        val missing = buildList {
            addAll(extracted.missing)
            if (product == null && extracted.product.isNotBlank()) {
                add("sản phẩm (không có trong bảng giá)")
            }
        }

        return Order(
            id = "dh-${System.currentTimeMillis()}",
            customerName = extracted.customerName.ifBlank { sender },
            phone = extracted.phone,
            address = extracted.address,
            productName = product?.name ?: extracted.product,
            productId = product?.id,
            quantity = extracted.quantity,
            unitPrice = product?.price ?: 0L,
            channel = channel,
            sourceExchangeId = exchangeId,
            missing = missing,
            createdAt = System.currentTimeMillis(),
        )
    }
}
