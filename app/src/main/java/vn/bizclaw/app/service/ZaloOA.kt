package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.*
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

/**
 * ZaloOA — Server-side Zalo Official Account API client.
 *
 * No AccessibilityService needed! Uses Zalo OpenAPI directly:
 * - Send messages to Zalo users (OA → User)
 * - Auto-refresh access token when expired
 * - Broadcast messages to followers
 *
 * Setup:
 * 1. Register at https://oa.zalo.me
 * 2. Get app_id, app_secret from https://developers.zalo.me
 * 3. Generate access_token via OAuth2
 * 4. Configure in app Settings → Zalo OA
 *
 * API Reference: https://developers.zalo.me/docs/official-account/
 */
class ZaloOA(private val context: Context) {

    companion object {
        private const val TAG = "ZaloOA"
        private const val CONFIG_FILE = "zalo_oa_config.json"
        private const val BASE_URL = "https://openapi.zalo.me"
        private const val OAUTH_URL = "https://oauth.zaloapp.com/v4/oa/access_token"
    }

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        prettyPrint = true
        encodeDefaults = true
    }

    // ═══════════════════════════════════════════════════════════════
    // Config Management
    // ═══════════════════════════════════════════════════════════════

    @Serializable
    data class OAConfig(
        val enabled: Boolean = false,
        val appId: String = "",
        val appSecret: String = "",
        val accessToken: String = "",
        val refreshToken: String = "",
        val oaId: String = "",
        val tokenExpiresAt: Long = 0,
    )

    fun loadConfig(): OAConfig {
        val file = File(context.filesDir, CONFIG_FILE)
        if (!file.exists()) return OAConfig()
        return try {
            json.decodeFromString<OAConfig>(file.readText())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load config: ${e.message}")
            OAConfig()
        }
    }

    fun saveConfig(config: OAConfig) {
        File(context.filesDir, CONFIG_FILE).writeText(json.encodeToString(config))
        Log.i(TAG, "💾 OA config saved (enabled=${config.enabled})")
    }

    // ═══════════════════════════════════════════════════════════════
    // Token Refresh
    // ═══════════════════════════════════════════════════════════════

    /**
     * Auto-refresh Zalo OA access token using refresh_token.
     * Zalo tokens expire after ~90 days. Refresh before that.
     *
     * Returns the new access_token, or null on failure.
     */
    suspend fun refreshAccessToken(): String? {
        val config = loadConfig()
        if (config.refreshToken.isBlank() || config.appSecret.isBlank()) {
            Log.w(TAG, "Cannot refresh: missing refresh_token or app_secret")
            return null
        }

        return withContext(Dispatchers.IO) {
            var conn: HttpURLConnection? = null
            try {
                val url = URL(OAUTH_URL)
                conn = url.openConnection() as HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/x-www-form-urlencoded")
                conn.setRequestProperty("secret_key", config.appSecret)
                conn.connectTimeout = 15_000
                conn.readTimeout = 15_000
                conn.doOutput = true

                val body = "refresh_token=${config.refreshToken}" +
                    "&app_id=${config.appId}" +
                    "&grant_type=refresh_token"

                conn.outputStream.use { os ->
                    os.write(body.toByteArray())
                }

                val code = conn.responseCode
                if (code != 200) {
                    val err = try {
                        conn.errorStream?.bufferedReader()?.readText()?.take(200)
                    } catch (_: Exception) { null }
                    Log.e(TAG, "Token refresh failed: $code — $err")
                    return@withContext null
                }

                val respBody = conn.inputStream.bufferedReader().readText()
                val respJson = json.parseToJsonElement(respBody).jsonObject

                val newAccessToken = respJson["access_token"]?.jsonPrimitive?.content
                val newRefreshToken = respJson["refresh_token"]?.jsonPrimitive?.content
                val expiresIn = respJson["expires_in"]?.jsonPrimitive?.content?.toLongOrNull() ?: 7776000 // ~90 days

                if (newAccessToken != null) {
                    val updated = config.copy(
                        accessToken = newAccessToken,
                        refreshToken = newRefreshToken ?: config.refreshToken,
                        tokenExpiresAt = System.currentTimeMillis() + (expiresIn * 1000),
                    )
                    saveConfig(updated)
                    Log.i(TAG, "🔄 Access token refreshed! Expires in ${expiresIn / 86400} days")
                    newAccessToken
                } else {
                    Log.e(TAG, "No access_token in refresh response")
                    null
                }
            } catch (e: Exception) {
                Log.e(TAG, "Token refresh error: ${e.message}")
                null
            } finally {
                conn?.disconnect()
            }
        }
    }

    /**
     * Get valid access token — auto-refresh if expired or expiring soon (within 7 days).
     */
    suspend fun getValidToken(): String? {
        val config = loadConfig()
        if (config.accessToken.isBlank()) return null

        val sevenDaysMs = 7 * 24 * 3600 * 1000L
        if (config.tokenExpiresAt > 0 && config.tokenExpiresAt - System.currentTimeMillis() < sevenDaysMs) {
            Log.i(TAG, "⚠️ Token expiring soon, refreshing...")
            return refreshAccessToken() ?: config.accessToken
        }

        return config.accessToken
    }

    // ═══════════════════════════════════════════════════════════════
    // Send Messages
    // ═══════════════════════════════════════════════════════════════

    /**
     * Send text message to a Zalo user via OA API.
     * No AccessibilityService needed!
     */
    suspend fun sendMessage(userId: String, text: String): OAResult {
        val token = getValidToken()
            ?: return OAResult(false, "❌ Chưa cấu hình Zalo OA token")

        return withContext(Dispatchers.IO) {
            var conn: HttpURLConnection? = null
            try {
                val url = URL("$BASE_URL/v3.0/oa/message/cs")
                conn = url.openConnection() as HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json")
                conn.setRequestProperty("access_token", token)
                conn.connectTimeout = 15_000
                conn.readTimeout = 15_000
                conn.doOutput = true

                val payload = buildJsonObject {
                    putJsonObject("recipient") {
                        put("user_id", userId)
                    }
                    putJsonObject("message") {
                        put("text", text)
                    }
                }

                conn.outputStream.use { os ->
                    os.write(json.encodeToString(payload).toByteArray())
                }

                val code = conn.responseCode
                val respBody = if (code == 200) {
                    conn.inputStream.bufferedReader().readText()
                } else {
                    conn.errorStream?.bufferedReader()?.readText() ?: "Error $code"
                }

                val respJson = try {
                    json.parseToJsonElement(respBody).jsonObject
                } catch (_: Exception) { null }

                val errorCode = respJson?.get("error")?.jsonPrimitive?.content?.toIntOrNull()
                    ?: respJson?.get("error")?.jsonPrimitive?.content?.toIntOrNull()

                if (code == 200 && (errorCode == null || errorCode == 0)) {
                    Log.i(TAG, "✅ OA message sent to $userId")
                    OAResult(true, "Đã gửi qua Zalo OA")
                } else {
                    val errMsg = respJson?.get("message")?.jsonPrimitive?.content ?: "Error $code"
                    Log.e(TAG, "OA send failed: $errMsg")

                    // Check if token expired
                    if (errorCode == -216 || errorCode == -124) {
                        Log.i(TAG, "Token expired, refreshing...")
                        val newToken = refreshAccessToken()
                        if (newToken != null) {
                            // Retry with new token
                            return@withContext sendMessage(userId, text)
                        }
                    }

                    OAResult(false, "❌ $errMsg")
                }
            } catch (e: Exception) {
                Log.e(TAG, "OA send error: ${e.message}")
                OAResult(false, "❌ ${e.message?.take(100)}")
            } finally {
                conn?.disconnect()
            }
        }
    }

    /**
     * Send image message via OA API.
     */
    suspend fun sendImage(userId: String, imageUrl: String, caption: String = ""): OAResult {
        val token = getValidToken()
            ?: return OAResult(false, "❌ Chưa cấu hình Zalo OA token")

        return withContext(Dispatchers.IO) {
            var conn: HttpURLConnection? = null
            try {
                val url = URL("$BASE_URL/v3.0/oa/message/cs")
                conn = url.openConnection() as HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json")
                conn.setRequestProperty("access_token", token)
                conn.connectTimeout = 15_000
                conn.readTimeout = 15_000
                conn.doOutput = true

                val payload = buildJsonObject {
                    putJsonObject("recipient") {
                        put("user_id", userId)
                    }
                    putJsonObject("message") {
                        putJsonObject("attachment") {
                            put("type", "template")
                            putJsonObject("payload") {
                                put("template_type", "media")
                                putJsonArray("elements") {
                                    addJsonObject {
                                        put("media_type", "image")
                                        put("url", imageUrl)
                                    }
                                }
                            }
                        }
                    }
                }

                conn.outputStream.use { os ->
                    os.write(json.encodeToString(payload).toByteArray())
                }

                val code = conn.responseCode
                if (code == 200) {
                    OAResult(true, "Đã gửi ảnh qua Zalo OA")
                } else {
                    val err = conn.errorStream?.bufferedReader()?.readText()?.take(200) ?: "Error"
                    OAResult(false, "❌ $err")
                }
            } catch (e: Exception) {
                OAResult(false, "❌ ${e.message?.take(100)}")
            } finally {
                conn?.disconnect()
            }
        }
    }

    /**
     * Get follower list from OA.
     */
    suspend fun getFollowers(offset: Int = 0, count: Int = 50): List<String> {
        val token = getValidToken() ?: return emptyList()

        return withContext(Dispatchers.IO) {
            var conn: HttpURLConnection? = null
            try {
                val url = URL("$BASE_URL/v2.0/oa/getfollowers?data={\"offset\":$offset,\"count\":$count}")
                conn = url.openConnection() as HttpURLConnection
                conn.requestMethod = "GET"
                conn.setRequestProperty("access_token", token)
                conn.connectTimeout = 15_000
                conn.readTimeout = 15_000

                val code = conn.responseCode
                if (code != 200) return@withContext emptyList()

                val body = conn.inputStream.bufferedReader().readText()
                val resp = json.parseToJsonElement(body).jsonObject
                val data = resp["data"]?.jsonObject
                val followers = data?.get("followers")?.jsonArray

                followers?.mapNotNull { it.jsonObject["user_id"]?.jsonPrimitive?.content }
                    ?: emptyList()
            } catch (e: Exception) {
                Log.e(TAG, "Get followers error: ${e.message}")
                emptyList()
            } finally {
                conn?.disconnect()
            }
        }
    }

    /**
     * Broadcast message to all followers.
     */
    suspend fun broadcast(text: String): OAResult {
        val followers = getFollowers()
        if (followers.isEmpty()) {
            return OAResult(false, "Không có follower nào")
        }

        var sent = 0
        var failed = 0
        for (userId in followers) {
            val result = sendMessage(userId, text)
            if (result.success) sent++ else failed++
            // Zalo rate limit: max 50/minute
            kotlinx.coroutines.delay(1200)
        }

        return OAResult(true, "📣 Broadcast: $sent/${ sent + failed} gửi thành công")
    }
}

// ═══════════════════════════════════════════════════════════════
// Result type
// ═══════════════════════════════════════════════════════════════

@Serializable
data class OAResult(
    val success: Boolean,
    val message: String,
)
