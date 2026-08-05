package vn.bizclaw.agent.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import vn.bizclaw.agent.llm.ModelVariant

private const val PREFS = "bizclaw_settings"

/**
 * User-tunable agent behaviour, persisted on every write.
 *
 * [autoSend] is off by default: a wrong answer sent to a real customer cannot be
 * recalled, so replies are held as drafts until the owner has seen the agent behave.
 */
class Settings(context: Context) {
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private var autoSendState by mutableStateOf(prefs.getBoolean(KEY_AUTO_SEND, false))
    private var agentEnabledState by mutableStateOf(prefs.getBoolean(KEY_ENABLED, false))
    private var businessNameState by mutableStateOf(prefs.getString(KEY_BUSINESS, "").orEmpty())
    private var personaState by mutableStateOf(
        prefs.getString(KEY_PERSONA, DEFAULT_PERSONA) ?: DEFAULT_PERSONA,
    )
    // Default to the largest variant this phone can actually hold rather than a fixed
    // one: a 16 GB flagship should not be handed the small model, and a 4 GB phone
    // would be killed by the large one.
    private var modelVariantState by mutableStateOf(
        prefs.getString(KEY_MODEL, null)
            ?.let(ModelVariant::from)
            ?: ModelVariant.recommendedFor(context)
            ?: ModelVariant.E2B,
    )

    var autoSend: Boolean
        get() = autoSendState
        set(value) {
            autoSendState = value
            prefs.edit().putBoolean(KEY_AUTO_SEND, value).apply()
        }

    var agentEnabled: Boolean
        get() = agentEnabledState
        set(value) {
            agentEnabledState = value
            prefs.edit().putBoolean(KEY_ENABLED, value).apply()
        }

    var businessName: String
        get() = businessNameState
        set(value) {
            businessNameState = value
            prefs.edit().putString(KEY_BUSINESS, value).apply()
        }

    var persona: String
        get() = personaState
        set(value) {
            personaState = value
            prefs.edit().putString(KEY_PERSONA, value).apply()
        }

    var modelVariant: ModelVariant
        get() = modelVariantState
        set(value) {
            modelVariantState = value
            prefs.edit().putString(KEY_MODEL, value.name).apply()
        }

    private companion object {
        const val KEY_AUTO_SEND = "auto_send"
        const val KEY_ENABLED = "agent_enabled"
        const val KEY_BUSINESS = "business_name"
        const val KEY_PERSONA = "persona"
        const val KEY_MODEL = "model_variant"

        const val DEFAULT_PERSONA =
            "Nhân viên chăm sóc khách hàng, xưng \"em\", gọi khách là \"anh/chị\". " +
                "Lịch sự, ngắn gọn, thân thiện."
    }
}
