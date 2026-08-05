package vn.bizclaw.agent.data

import android.content.Context
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.snapshots.SnapshotStateList
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import java.util.Locale

private const val PREFS = "bizclaw_knowledge"
private const val KEY = "docs"

/**
 * Business facts the agent is allowed to quote.
 *
 * A 2B model will invent a shipping fee if you let it, so [relevantTo] pulls the
 * matching documents into the prompt and the agent is instructed to refuse when
 * nothing matches.
 */
class KnowledgeStore(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    val docs: SnapshotStateList<KnowledgeDoc> = mutableStateListOf()

    private val serializer = ListSerializer(KnowledgeDoc.serializer())

    init {
        runCatching {
            json.decodeFromString(serializer, prefs.getString(KEY, "[]") ?: "[]")
        }.getOrDefault(emptyList()).forEach(docs::add)
    }

    fun save(title: String, body: String) {
        if (body.isBlank()) return
        docs.add(
            0,
            KnowledgeDoc(
                id = "kb-${System.currentTimeMillis()}",
                title = title.trim().ifBlank { "Tài liệu ${docs.size + 1}" },
                body = body.trim(),
                updatedAt = System.currentTimeMillis(),
            ),
        )
        persist()
    }

    fun delete(id: String) {
        docs.removeAll { it.id == id }
        persist()
    }

    /**
     * Keyword overlap between the customer question and each document, accent-folded so
     * "phi ship" matches "phí ship". Cheap on purpose — embeddings would cost more RAM
     * than the reply itself is worth at this size.
     */
    fun relevantTo(question: String, limit: Int = 3): List<KnowledgeDoc> {
        val terms = fold(question).split(NON_WORD)
            .filter { it.length >= 3 }
            .toSet()
        if (terms.isEmpty()) return docs.take(limit)
        return docs
            .map { doc -> doc to score(doc, terms) }
            .filter { it.second > 0 }
            .sortedByDescending { it.second }
            .take(limit)
            .map { it.first }
    }

    private fun score(doc: KnowledgeDoc, terms: Set<String>): Int {
        val haystack = fold("${doc.title} ${doc.body}")
        return terms.count { haystack.contains(it) }
    }

    private fun persist() {
        val payload = runCatching {
            json.encodeToString(serializer, docs.toList())
        }.getOrNull() ?: return
        prefs.edit().putString(KEY, payload).apply()
    }

    private companion object {
        val NON_WORD = Regex("[^\\p{L}\\p{N}]+")
    }
}

/** Lowercase and strip Vietnamese diacritics so keyword matching is accent-insensitive. */
fun fold(value: String): String = value.lowercase(Locale.ROOT)
    .replace(Regex("[àáạảãâầấậẩẫăằắặẳẵ]"), "a")
    .replace(Regex("[èéẹẻẽêềếệểễ]"), "e")
    .replace(Regex("[ìíịỉĩ]"), "i")
    .replace(Regex("[òóọỏõôồốộổỗơờớợởỡ]"), "o")
    .replace(Regex("[ùúụủũưừứựửữ]"), "u")
    .replace(Regex("[ỳýỵỷỹ]"), "y")
    .replace("đ", "d")
