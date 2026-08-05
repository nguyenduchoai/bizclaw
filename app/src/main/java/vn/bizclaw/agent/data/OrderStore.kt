package vn.bizclaw.agent.data

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.SnapshotStateList
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json

private const val PREFS = "bizclaw_orders"
private const val KEY = "orders"

enum class OrderState(val label: String) {
    /** Agent captured it from chat; the owner has not confirmed yet. */
    DRAFT("chờ xác nhận"),
    CONFIRMED("đã xác nhận"),
    SHIPPED("đã giao"),
    DONE("hoàn tất"),
    CANCELLED("đã huỷ"),
}

@Serializable
data class Order(
    val id: String,
    val customerName: String,
    val phone: String,
    val address: String,
    val productName: String,
    val productId: String? = null,
    val quantity: Int,
    val unitPrice: Long,
    val state: OrderState = OrderState.DRAFT,
    val channel: Channel = Channel.UNKNOWN,
    val sourceExchangeId: String? = null,
    /** Fields the customer has not supplied yet; empty means ready to confirm. */
    val missing: List<String> = emptyList(),
    val createdAt: Long,
    /** When to nudge the owner to check in on this customer. Null = no follow-up. */
    val followUpAt: Long? = null,
    val followUpDone: Boolean = false,
) {
    val total: Long get() = unitPrice * quantity
    val totalLabel: String get() = "%,d₫".format(total)
    val isComplete: Boolean get() = missing.isEmpty() && phone.isNotBlank()
}

/** Orders the agent captured from chat, newest first. */
class OrderStore(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }
    private val serializer = ListSerializer(Order.serializer())

    val orders: SnapshotStateList<Order> = mutableStateListOf()

    init {
        runCatching {
            json.decodeFromString(serializer, prefs.getString(KEY, "[]") ?: "[]")
        }.getOrDefault(emptyList()).forEach(orders::add)
    }

    fun add(order: Order) {
        orders.add(0, order)
        persist()
    }

    fun update(id: String, transform: (Order) -> Order) {
        val index = orders.indexOfFirst { it.id == id }
        if (index < 0) return
        orders[index] = transform(orders[index])
        persist()
    }

    fun countBy(state: OrderState): Int = orders.count { it.state == state }

    /** Confirmed orders whose follow-up date has passed and that nobody has handled. */
    fun dueFollowUps(now: Long = System.currentTimeMillis()): List<Order> =
        orders.filter { order ->
            !order.followUpDone &&
                order.followUpAt != null &&
                order.followUpAt <= now &&
                order.state in setOf(OrderState.CONFIRMED, OrderState.SHIPPED, OrderState.DONE)
        }

    private fun persist() {
        val payload = runCatching {
            json.encodeToString(serializer, orders.toList())
        }.getOrNull() ?: return
        prefs.edit().putString(KEY, payload).apply()
    }
}
