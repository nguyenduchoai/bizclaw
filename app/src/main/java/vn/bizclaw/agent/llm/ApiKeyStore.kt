package vn.bizclaw.agent.llm

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/**
 * API keys and model names for the cloud providers.
 *
 * Encrypted at rest: an API key on a lost phone is a billable liability, unlike the
 * order data next to it. Falls back to plain preferences only if the keystore is
 * unavailable, which happens on a few OEM builds with a broken StrongBox.
 */
class ApiKeyStore(context: Context) {
    private val prefs = encryptedPrefs(context.applicationContext)

    private var claudeKeyState by mutableStateOf(prefs.getString(KEY_CLAUDE, "").orEmpty())
    private var openAiKeyState by mutableStateOf(prefs.getString(KEY_OPENAI, "").orEmpty())
    private var claudeModelState by mutableStateOf(
        prefs.getString(KEY_CLAUDE_MODEL, DEFAULT_CLAUDE_MODEL) ?: DEFAULT_CLAUDE_MODEL,
    )
    private var openAiModelState by mutableStateOf(
        prefs.getString(KEY_OPENAI_MODEL, DEFAULT_OPENAI_MODEL) ?: DEFAULT_OPENAI_MODEL,
    )

    var claudeKey: String
        get() = claudeKeyState
        set(value) {
            claudeKeyState = value.trim()
            prefs.edit().putString(KEY_CLAUDE, claudeKeyState).apply()
        }

    var openAiKey: String
        get() = openAiKeyState
        set(value) {
            openAiKeyState = value.trim()
            prefs.edit().putString(KEY_OPENAI, openAiKeyState).apply()
        }

    var claudeModel: String
        get() = claudeModelState
        set(value) {
            claudeModelState = value.trim().ifBlank { DEFAULT_CLAUDE_MODEL }
            prefs.edit().putString(KEY_CLAUDE_MODEL, claudeModelState).apply()
        }

    var openAiModel: String
        get() = openAiModelState
        set(value) {
            openAiModelState = value.trim().ifBlank { DEFAULT_OPENAI_MODEL }
            prefs.edit().putString(KEY_OPENAI_MODEL, openAiModelState).apply()
        }

    private fun encryptedPrefs(context: Context): SharedPreferences = runCatching {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        EncryptedSharedPreferences.create(
            context,
            PREFS,
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    }.getOrElse {
        context.getSharedPreferences("${PREFS}_fallback", Context.MODE_PRIVATE)
    }

    companion object {
        private const val PREFS = "bizclaw_keys"
        private const val KEY_CLAUDE = "claude_key"
        private const val KEY_OPENAI = "openai_key"
        private const val KEY_CLAUDE_MODEL = "claude_model"
        private const val KEY_OPENAI_MODEL = "openai_model"

        const val DEFAULT_CLAUDE_MODEL = "claude-opus-5"
        const val DEFAULT_OPENAI_MODEL = "gpt-5"
    }
}
