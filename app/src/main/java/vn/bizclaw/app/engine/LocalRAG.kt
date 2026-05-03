package vn.bizclaw.app.engine

import android.content.Context
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import kotlin.math.ln

/**
 * On-Device RAG (Retrieval-Augmented Generation) engine.
 *
 * Uses BM25 keyword search over a local JSON knowledge base.
 * Lightweight, no ML model needed — pure text matching.
 *
 * Usage:
 *   val rag = LocalRAG(context, "my_knowledge")
 *   rag.addDocument("Giá sản phẩm A là 500k")
 *   val context = rag.search("giá sản phẩm A", topK = 3)
 */
class LocalRAG(context: Context, private val knowledgeId: String) {

    @Serializable
    data class Document(
        val id: String,
        val content: String,
        val metadata: String = "", // tags, source, etc.
    )

    @Serializable
    data class KnowledgeBase(
        val id: String,
        val name: String,
        val description: String = "",
        val documents: MutableList<Document> = mutableListOf(),
    )

    private val json = Json { prettyPrint = true; ignoreUnknownKeys = true }
    private val storageDir = File(context.filesDir, "rag").also { it.mkdirs() }
    private val kbFile = File(storageDir, "$knowledgeId.json")
    private var kb: KnowledgeBase

    init {
        kb = if (kbFile.exists()) {
            try {
                json.decodeFromString(kbFile.readText())
            } catch (e: Exception) {
                KnowledgeBase(id = knowledgeId, name = knowledgeId)
            }
        } else {
            KnowledgeBase(id = knowledgeId, name = knowledgeId)
        }
    }

    /** Add a document to the knowledge base */
    fun addDocument(content: String, metadata: String = ""): String {
        val id = "doc_${System.currentTimeMillis()}"
        kb.documents.add(Document(id = id, content = content, metadata = metadata))
        save()
        return id
    }

    /** Add multiple documents at once */
    fun addDocuments(contents: List<String>) {
        contents.forEach { addDocument(it) }
    }

    /** Remove a document by ID */
    fun removeDocument(docId: String) {
        kb.documents.removeAll { it.id == docId }
        save()
    }

    /** Get all documents */
    fun getDocuments(): List<Document> = kb.documents.toList()

    /** Get document count */
    fun size(): Int = kb.documents.size

    /** Update knowledge base name/description */
    fun updateInfo(name: String, description: String = "") {
        kb = kb.copy(name = name, description = description)
        save()
    }

    /** Search for relevant documents using BM25 scoring */
    fun search(query: String, topK: Int = 3): List<Document> {
        if (kb.documents.isEmpty() || query.isBlank()) return emptyList()

        val queryTerms = tokenize(query)
        if (queryTerms.isEmpty()) return emptyList()

        // BM25 parameters
        val k1 = 1.5
        val b = 0.75
        val avgDl = kb.documents.map { tokenize(it.content).size }.average()
        val n = kb.documents.size.toDouble()

        // Score each document
        val scored = kb.documents.map { doc ->
            val docTerms = tokenize(doc.content)
            val dl = docTerms.size.toDouble()

            var score = 0.0
            for (term in queryTerms) {
                val tf = docTerms.count { it == term }.toDouble()
                val df = kb.documents.count { d -> tokenize(d.content).contains(term) }.toDouble()
                val idf = ln((n - df + 0.5) / (df + 0.5) + 1.0)
                val tfNorm = (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * dl / avgDl))
                score += idf * tfNorm
            }

            doc to score
        }

        return scored
            .filter { it.second > 0 }
            .sortedByDescending { it.second }
            .take(topK)
            .map { it.first }
    }

    /** Build context string for LLM prompt injection */
    fun buildContext(query: String, topK: Int = 3): String {
        val results = search(query, topK)
        if (results.isEmpty()) return ""

        val contextStr = results.joinToString("\n---\n") { it.content }
        return "Thông tin tham khảo:\n$contextStr\n\nDựa vào thông tin trên, hãy trả lời:"
    }

    /** Clear all documents */
    fun clear() {
        kb.documents.clear()
        save()
    }

    private fun save() {
        kbFile.writeText(json.encodeToString(kb))
    }

    private fun tokenize(text: String): List<String> {
        return text.lowercase()
            .replace(Regex("[^\\p{L}\\p{N}\\s]"), " ")
            .split(Regex("\\s+"))
            .filter { it.length > 1 }
    }

    companion object {
        /** List all knowledge bases on device */
        fun listKnowledgeBases(context: Context): List<String> {
            val dir = File(context.filesDir, "rag")
            if (!dir.exists()) return emptyList()
            return dir.listFiles()
                ?.filter { it.extension == "json" }
                ?.map { it.nameWithoutExtension }
                ?: emptyList()
        }

        /** Delete a knowledge base */
        fun deleteKnowledgeBase(context: Context, id: String) {
            File(context.filesDir, "rag/$id.json").delete()
        }
    }
}
