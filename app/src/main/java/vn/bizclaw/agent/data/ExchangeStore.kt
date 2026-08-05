package vn.bizclaw.agent.data

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.SnapshotStateList
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json

private const val PREFS = "bizclaw_exchanges"
private const val KEY = "items"
private const val MAX_ITEMS = 200

/**
 * The agent's inbox, newest first.
 *
 * Backed by a Compose snapshot list so the notification listener can write from a
 * background coroutine and the UI recomposes without an observer wiring step.
 */
class ExchangeStore(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    val items: SnapshotStateList<Exchange> = mutableStateListOf()

    private val serializer = ListSerializer(Exchange.serializer())

    init {
        runCatching {
            json.decodeFromString(serializer, prefs.getString(KEY, "[]") ?: "[]")
        }.getOrDefault(emptyList()).forEach(items::add)
    }

    fun add(exchange: Exchange) {
        items.add(0, exchange)
        while (items.size > MAX_ITEMS) items.removeAt(items.lastIndex)
        persist()
    }

    fun update(id: String, transform: (Exchange) -> Exchange) {
        val index = items.indexOfFirst { it.id == id }
        if (index < 0) return
        items[index] = transform(items[index])
        persist()
    }

    fun clear() {
        items.clear()
        persist()
    }

    fun countBy(state: ReplyState): Int = items.count { it.state == state }

    private fun persist() {
        val payload = runCatching {
            json.encodeToString(serializer, items.toList())
        }.getOrNull() ?: return
        prefs.edit().putString(KEY, payload).apply()
    }
}
