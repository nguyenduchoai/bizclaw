package vn.bizclaw.agent.data

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.SnapshotStateList
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json

private const val PREFS = "bizclaw_catalog"
private const val KEY = "products"

/** One sellable item. Price in đồng, no decimals. */
@Serializable
data class Product(
    val id: String,
    val name: String,
    val price: Long,
    val stock: Int,
    val note: String = "",
) {
    val priceLabel: String get() = "%,d₫".format(price)
    val inStock: Boolean get() = stock > 0
}

/**
 * The product list the agent quotes from.
 *
 * Separate from [KnowledgeStore] because prices and stock are structured facts the app
 * needs to compute an order total, not prose the model paraphrases.
 */
class ProductStore(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }
    private val serializer = ListSerializer(Product.serializer())

    val products: SnapshotStateList<Product> = mutableStateListOf()

    init {
        runCatching {
            json.decodeFromString(serializer, prefs.getString(KEY, "[]") ?: "[]")
        }.getOrDefault(emptyList()).forEach(products::add)
    }

    fun save(name: String, price: Long, stock: Int, note: String) {
        if (name.isBlank()) return
        products.add(
            0,
            Product(
                id = "sp-${System.currentTimeMillis()}",
                name = name.trim(),
                price = price,
                stock = stock,
                note = note.trim(),
            ),
        )
        persist()
    }

    fun delete(id: String) {
        products.removeAll { it.id == id }
        persist()
    }

    fun adjustStock(id: String, delta: Int) {
        val index = products.indexOfFirst { it.id == id }
        if (index < 0) return
        val current = products[index]
        products[index] = current.copy(stock = (current.stock + delta).coerceAtLeast(0))
        persist()
    }

    /**
     * Best-effort match of a free-text product name the model echoed back.
     *
     * Customers write "váy lụa" for "Váy lụa cao cấp", so this scores on shared words
     * rather than requiring an exact string.
     */
    fun match(query: String): Product? {
        if (query.isBlank() || products.isEmpty()) return null
        val terms = fold(query).split(Regex("[^\\p{L}\\p{N}]+")).filter { it.length >= 2 }
        if (terms.isEmpty()) return null
        return products
            .map { product ->
                val name = fold(product.name)
                product to terms.count { name.contains(it) }
            }
            .filter { it.second > 0 }
            .maxByOrNull { it.second }
            ?.first
    }

    /** Compact catalog block for the prompt. Out-of-stock items are marked, not hidden. */
    fun forPrompt(limit: Int = 30): String {
        if (products.isEmpty()) return "(chưa có sản phẩm nào)"
        return products.take(limit).joinToString("\n") { product ->
            val stock = if (product.inStock) "còn ${product.stock}" else "HẾT HÀNG"
            val note = product.note.takeIf { it.isNotBlank() }?.let { " — $it" }.orEmpty()
            "- ${product.name}: ${product.priceLabel} ($stock)$note"
        }
    }

    private fun persist() {
        val payload = runCatching {
            json.encodeToString(serializer, products.toList())
        }.getOrNull() ?: return
        prefs.edit().putString(KEY, payload).apply()
    }
}
