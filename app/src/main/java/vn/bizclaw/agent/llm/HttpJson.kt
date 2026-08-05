package vn.bizclaw.agent.llm

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import okhttp3.Headers
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.util.concurrent.TimeUnit

/** Thrown when a provider answers with a non-2xx status. Carries the body for display. */
class HttpApiException(val status: Int, val body: String) :
    Exception("HTTP $status: ${body.take(400)}")

/** Shared JSON + HTTP plumbing for the cloud providers. */
internal object HttpJson {

    val json = Json { ignoreUnknownKeys = true; isLenient = true; encodeDefaults = true }

    private val MEDIA_TYPE = "application/json; charset=utf-8".toMediaType()

    /**
     * Generous read timeout: a tool round-trip on a large model can take tens of
     * seconds, and a timeout here loses the whole turn plus everything already billed.
     */
    private val client = OkHttpClient.Builder()
        .connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(180, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    suspend fun post(url: String, headers: Headers, body: JsonObject): JsonObject =
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url(url)
                .headers(headers)
                .post(body.toString().toRequestBody(MEDIA_TYPE))
                .build()

            client.newCall(request).execute().use { response ->
                val text = response.body?.string().orEmpty()
                if (!response.isSuccessful) throw HttpApiException(response.code, text)
                json.parseToJsonElement(text) as JsonObject
            }
        }
}
