package vn.bizclaw.app.engine

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Multi-Provider Manager — cho phép thêm nhiều nguồn AI
 *
 * Mỗi provider có thể là:
 * - Local GGUF (BizClawLLM)
 * - OpenAI API
 * - Gemini API
 * - Ollama (local server)
 * - BizClaw Cloud
 * - Custom API
 *
 * Mỗi agent chọn 1 provider để trả lời.
 *
 * SECURITY: API keys mã hoá bằng EncryptedSharedPreferences (Android Keystore).
 * File JSON KHÔNG chứa API key plaintext.
 */

@Serializable
data class AIProvider(
    val id: String,
    val name: String,
    val type: ProviderType,
    val emoji: String = "🤖",
    val apiKey: String = "",
    val baseUrl: String = "",
    val model: String = "",
    val enabled: Boolean = true,
    val createdAt: Long = System.currentTimeMillis(),
)

@Serializable
enum class ProviderType {
    LOCAL_GGUF,      // On-device via BizClawLLM
    OPENAI,          // OpenAI API (GPT-4, etc.)
    GEMINI,          // Google Gemini API
    OLLAMA,          // Local Ollama server
    BIZCLAW_CLOUD,   // BizClaw backend
    CUSTOM_API,      // Any OpenAI-compatible API
    APP_GEMINI,      // Use Gemini app on device (free, slower)
    APP_CHATGPT,     // Use ChatGPT app on device (free, slower)
    APP_GROK,        // Use Grok app on device (free, slower)
    APP_DEEPSEEK,    // Use DeepSeek app on device (free, slower)
    APP_NOTEBOOKLM,  // Use NotebookLM app as RAG (free, grounded answers)
}

/**
 * Agent Group — nhóm agent cùng làm việc
 */
@Serializable
data class AgentGroup(
    val id: String,
    val name: String,
    val emoji: String = "👥",
    val description: String = "",
    val agentIds: List<String> = emptyList(),
    val routerAgentId: String? = null,
    val createdAt: Long = System.currentTimeMillis(),
)

class ProviderManager(context: Context) {

    private val json = Json { prettyPrint = true; ignoreUnknownKeys = true }
    private val providersFile = File(context.filesDir, "ai_providers.json")
    private val groupsFile = File(context.filesDir, "agent_groups.json")

    // Encrypted storage for API keys
    private val securePrefs: SharedPreferences = try {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        EncryptedSharedPreferences.create(
            context,
            "bizclaw_secure_keys",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    } catch (e: Exception) {
        Log.e("ProviderManager", "EncryptedSharedPreferences failed, fallback", e)
        context.getSharedPreferences("bizclaw_keys_fallback", Context.MODE_PRIVATE)
    }

    // ─── Providers ─────────────────────────────────────

    fun loadProviders(): List<AIProvider> {
        val providers = if (!providersFile.exists()) {
            defaultProviders()
        } else {
            try {
                json.decodeFromString<List<AIProvider>>(providersFile.readText())
            } catch (e: Exception) {
                defaultProviders()
            }
        }
        // Merge API keys from secure storage
        val mergedList = providers.mapNotNull { p ->
            // Skip all App providers as they are deprecated
            if (p.type.name.startsWith("APP_")) {
                return@mapNotNull null
            }
            
            val storedKey = securePrefs.getString("key_${p.id}", null)
            if (storedKey != null && p.apiKey.isBlank()) {
                p.copy(apiKey = storedKey)
            } else {
                p
            }
        }
        
        return mergedList
    }

    fun saveProviders(providers: List<AIProvider>) {
        // Extract API keys to secure storage, save providers WITHOUT keys
        val editor = securePrefs.edit()
        providers.forEach { p ->
            if (p.apiKey.isNotBlank()) {
                editor.putString("key_${p.id}", p.apiKey)
            }
        }
        editor.apply()

        // Save to JSON file WITHOUT plaintext API keys
        val sanitized = providers.map { it.copy(apiKey = "") }
        providersFile.writeText(json.encodeToString(sanitized))
    }

    fun addProvider(provider: AIProvider) {
        val list = loadProviders().toMutableList()
        list.add(provider)
        saveProviders(list)
    }

    fun updateProvider(provider: AIProvider) {
        val list = loadProviders().toMutableList()
        val idx = list.indexOfFirst { it.id == provider.id }
        if (idx >= 0) {
            list[idx] = provider
            saveProviders(list)
        }
    }

    fun deleteProvider(id: String) {
        val list = loadProviders().toMutableList()
        list.removeAll { it.id == id }
        // Remove encrypted key too
        securePrefs.edit().remove("key_$id").apply()
        saveProviders(list)
    }

    // ─── Groups ─────────────────────────────────────

    fun loadGroups(): List<AgentGroup> {
        if (!groupsFile.exists()) return emptyList()
        return try {
            json.decodeFromString<List<AgentGroup>>(groupsFile.readText())
        } catch (e: Exception) {
            emptyList()
        }
    }

    fun saveGroups(groups: List<AgentGroup>) {
        groupsFile.writeText(json.encodeToString(groups))
    }

    fun addGroup(group: AgentGroup) {
        val list = loadGroups().toMutableList()
        list.add(group)
        saveGroups(list)
    }

    fun updateGroup(group: AgentGroup) {
        val list = loadGroups().toMutableList()
        val idx = list.indexOfFirst { it.id == group.id }
        if (idx >= 0) {
            list[idx] = group
            saveGroups(list)
        }
    }

    fun deleteGroup(id: String) {
        val list = loadGroups().toMutableList()
        list.removeAll { it.id == id }
        saveGroups(list)
    }

    // ─── Default Providers ─────────────────────────────

    private fun defaultProviders(): List<AIProvider> {
        val defaults = listOf(
            AIProvider(
                id = "local_gguf",
                name = "AI Cục Bộ (GGUF)",
                type = ProviderType.LOCAL_GGUF,
                emoji = "🧠",
                model = "auto",
            ),
            AIProvider(
                id = "openai",
                name = "OpenAI",
                type = ProviderType.OPENAI,
                emoji = "🌐",
                baseUrl = "https://api.openai.com/v1",
                model = "gpt-4o-mini",
                enabled = false, // Cần thêm API key
            ),
            AIProvider(
                id = "gemini",
                name = "Google Gemini",
                type = ProviderType.GEMINI,
                emoji = "✨",
                baseUrl = "https://generativelanguage.googleapis.com",
                model = "gemini-2.0-flash",
                enabled = false,
            ),
            AIProvider(
                id = "ollama",
                name = "Ollama (Máy tính)",
                type = ProviderType.OLLAMA,
                emoji = "🦙",
                baseUrl = "http://192.168.1.100:11434",
                model = "qwen2.5:7b",
                enabled = false,
            ),
        )
        saveProviders(defaults)
        return defaults
    }
}
