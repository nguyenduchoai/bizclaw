package vn.bizclaw.app

import android.annotation.SuppressLint
import android.content.Context
import android.content.SharedPreferences
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Handler
import android.os.Looper
import android.util.Base64
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.max
import kotlin.math.sqrt

enum class TranslationProvider(
    val label: String,
    val sampleRate: Int,
) {
    Soniox("Soniox", 16_000),
    OpenAI("OpenAI", 24_000),
    Qwen("Qwen", 16_000);

    companion object {
        fun fromPref(value: String?): TranslationProvider =
            entries.firstOrNull { it.name.equals(value, ignoreCase = true) } ?: Soniox
    }
}

enum class TranslationStatus(val label: String) {
    Idle("Sẵn sàng"),
    Connecting("Đang kết nối"),
    Streaming("Đang nghe"),
    Error("Lỗi"),
}

data class TranslationLine(
    val id: String,
    val source: String,
    val translation: String,
    val provider: TranslationProvider,
    val createdAt: Long = System.currentTimeMillis(),
)

class TranslationController(context: Context) {
    private val prefs = encryptedPrefs(context.applicationContext)
    private val main = Handler(Looper.getMainLooper())

    val rows = mutableStateListOf<TranslationLine>()

    var provider by mutableStateOf(TranslationProvider.fromPref(prefs.getString("provider", "Soniox")))
    var targetLanguage by mutableStateOf(prefs.getString("target_language", "vi") ?: "vi")
    var sonioxKey by mutableStateOf(prefs.getString("soniox_key", "") ?: "")
    var openaiKey by mutableStateOf(prefs.getString("openai_key", "") ?: "")
    var qwenKey by mutableStateOf(prefs.getString("qwen_key", "") ?: "")
    var status by mutableStateOf(TranslationStatus.Idle)
    var errorMessage by mutableStateOf<String?>(null)
    var sourceProvisional by mutableStateOf("")
    var targetProvisional by mutableStateOf("")

    private var audioCapture: PcmAudioCapture? = null
    private var client: RealtimeTranslationClient? = null

    val isRunning: Boolean
        get() = status == TranslationStatus.Connecting || status == TranslationStatus.Streaming

    fun saveSettings() {
        prefs.edit()
            .putString("provider", provider.name)
            .putString("target_language", targetLanguage.trim().ifBlank { "vi" })
            .putString("soniox_key", sonioxKey.trim())
            .putString("openai_key", openaiKey.trim())
            .putString("qwen_key", qwenKey.trim())
            .apply()
    }

    fun updateActiveKey(value: String) {
        when (provider) {
            TranslationProvider.Soniox -> sonioxKey = value
            TranslationProvider.OpenAI -> openaiKey = value
            TranslationProvider.Qwen -> qwenKey = value
        }
    }

    fun activeKey(): String = when (provider) {
        TranslationProvider.Soniox -> sonioxKey
        TranslationProvider.OpenAI -> openaiKey
        TranslationProvider.Qwen -> qwenKey
    }

    fun start() {
        if (isRunning) return
        saveSettings()
        val key = activeKey().trim()
        if (key.isEmpty()) {
            fail("Thiếu API key cho ${provider.label}.")
            return
        }
        errorMessage = null
        sourceProvisional = ""
        targetProvisional = ""
        status = TranslationStatus.Connecting

        val callbacks = TranslationCallbacks(
            onStatus = { next -> onMain { status = next } },
            onSourceProvisional = { text -> onMain { sourceProvisional = text } },
            onTargetProvisional = { text -> onMain { targetProvisional = text } },
            onSegment = { source, translation ->
                onMain {
                    sourceProvisional = ""
                    targetProvisional = ""
                    if (source.isNotBlank() || translation.isNotBlank()) {
                        rows.add(
                            0,
                            TranslationLine(
                                id = "tr-${System.currentTimeMillis()}-${rows.size}",
                                source = source.trim(),
                                translation = translation.trim(),
                                provider = provider,
                            ),
                        )
                        while (rows.size > 200) rows.removeAt(rows.lastIndex)
                    }
                }
            },
            onError = { message -> onMain { fail(message) } },
        )

        val nextClient = when (provider) {
            TranslationProvider.Soniox -> SonioxRealtimeTranslator(key, targetLanguage, callbacks)
            TranslationProvider.OpenAI -> OpenAiRealtimeTranslator(key, targetLanguage, callbacks)
            TranslationProvider.Qwen -> QwenRealtimeTranslator(key, targetLanguage, callbacks)
        }
        client = nextClient

        try {
            nextClient.connect()
            audioCapture = PcmAudioCapture(provider.sampleRate).also { capture ->
                capture.start { pcm -> nextClient.sendAudio(pcm) }
            }
        } catch (error: Throwable) {
            stop()
            fail(error.message ?: "Không khởi động được mic.")
        }
    }

    fun stop() {
        try {
            client?.flushPending()
        } catch (_: Throwable) {
        }
        try {
            audioCapture?.stop()
        } catch (_: Throwable) {
        }
        try {
            client?.disconnect()
        } catch (_: Throwable) {
        }
        audioCapture = null
        client = null
        sourceProvisional = ""
        targetProvisional = ""
        if (status != TranslationStatus.Error) status = TranslationStatus.Idle
    }

    fun clear() {
        rows.clear()
        sourceProvisional = ""
        targetProvisional = ""
        errorMessage = null
    }

    fun permissionDenied() {
        fail("Android chưa cấp quyền microphone.")
    }

    fun transcriptMarkdown(): String {
        if (rows.isEmpty()) return ""
        val lines = rows.asReversed().mapIndexed { idx, row ->
            buildString {
                append("## Đoạn ${idx + 1}\n")
                if (row.source.isNotBlank()) append("- Gốc: ${row.source}\n")
                if (row.translation.isNotBlank()) append("- Dịch: ${row.translation}\n")
            }
        }
        return buildString {
            append("# Phiên dịch BizClaw\n\n")
            append("- Provider: ${provider.label}\n")
            append("- Target: ${targetLanguage}\n\n")
            append(lines.joinToString("\n"))
        }
    }

    private fun fail(message: String) {
        errorMessage = message
        status = TranslationStatus.Error
    }

    private fun onMain(block: () -> Unit) {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            block()
        } else {
            main.post(block)
        }
    }

    private fun encryptedPrefs(context: Context): SharedPreferences {
        return try {
            val key = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            EncryptedSharedPreferences.create(
                context,
                "bizclaw_translation",
                key,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            )
        } catch (_: Throwable) {
            context.getSharedPreferences("bizclaw_translation_fallback", Context.MODE_PRIVATE)
        }
    }
}

private data class TranslationCallbacks(
    val onStatus: (TranslationStatus) -> Unit,
    val onSourceProvisional: (String) -> Unit,
    val onTargetProvisional: (String) -> Unit,
    val onSegment: (String, String) -> Unit,
    val onError: (String) -> Unit,
)

private interface RealtimeTranslationClient {
    fun connect()
    fun sendAudio(pcm: ByteArray)
    fun flushPending() {}
    fun disconnect()
}

private class SonioxRealtimeTranslator(
    private val apiKey: String,
    private val targetLanguage: String,
    private val callbacks: TranslationCallbacks,
) : RealtimeTranslationClient {
    private val http = OkHttpClient()
    private var ws: WebSocket? = null
    private val recentTranslations = ArrayDeque<String>()

    override fun connect() {
        val request = Request.Builder()
            .url("wss://stt-rt.soniox.com/transcribe-websocket")
            .build()
        ws = http.newWebSocket(
            request,
            object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    val config = JSONObject()
                        .put("api_key", apiKey)
                        .put("model", "stt-rt-v4")
                        .put("audio_format", "pcm_s16le")
                        .put("sample_rate", 16_000)
                        .put("num_channels", 1)
                        .put("enable_endpoint_detection", true)
                        .put("max_endpoint_delay_ms", 3_000)
                        .put("enable_speaker_diarization", true)
                        .put("enable_language_identification", true)
                        .put("translation", JSONObject().put("type", "one_way").put("target_language", targetLanguage.ifBlank { "vi" }))
                    webSocket.send(config.toString())
                    callbacks.onStatus(TranslationStatus.Streaming)
                }

                override fun onMessage(webSocket: WebSocket, text: String) {
                    handleMessage(text)
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    callbacks.onError(t.message ?: "Soniox WebSocket lỗi.")
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    callbacks.onStatus(TranslationStatus.Idle)
                }
            },
        )
    }

    override fun sendAudio(pcm: ByteArray) {
        ws?.send(pcm.toByteString())
    }

    override fun disconnect() {
        ws?.send(ByteString.EMPTY)
        ws?.close(1000, "stop")
        ws = null
    }

    private fun handleMessage(text: String) {
        val json = JSONObject(text)
        if (json.has("error_code")) {
            callbacks.onError(json.optString("error_message", "Soniox error ${json.optString("error_code")}"))
            return
        }
        val tokens = json.optJSONArray("tokens") ?: return
        val source = StringBuilder()
        val translation = StringBuilder()
        val provisional = StringBuilder()
        for (idx in 0 until tokens.length()) {
            val token = tokens.optJSONObject(idx) ?: continue
            val tokenText = token.optString("text")
            if (tokenText == "<end>") continue
            when (token.optString("translation_status")) {
                "translation" -> if (token.optBoolean("is_final")) translation.append(tokenText)
                "original", "none" -> {
                    if (token.optBoolean("is_final")) source.append(tokenText) else provisional.append(tokenText)
                }
            }
        }
        val src = source.toString().trim()
        val tgt = translation.toString().trim()
        if (provisional.isNotBlank()) callbacks.onSourceProvisional(provisional.toString())
        if (src.isNotBlank() || tgt.isNotBlank()) {
            callbacks.onSegment(src, tgt)
            if (tgt.isNotBlank()) {
                recentTranslations.addLast(tgt)
                while (recentTranslations.sumOf { it.length } > 500 && recentTranslations.size > 1) {
                    recentTranslations.removeFirst()
                }
            }
        }
    }
}

private class OpenAiRealtimeTranslator(
    private val apiKey: String,
    private val targetLanguage: String,
    private val callbacks: TranslationCallbacks,
) : RealtimeTranslationClient {
    private val http = OkHttpClient()
    private var ws: WebSocket? = null
    private val pendingSources = ArrayDeque<String>()
    private var sourceBuffer = ""
    private var targetBuffer = ""

    override fun connect() {
        val request = Request.Builder()
            .url("wss://api.openai.com/v1/realtime/translations?model=gpt-realtime-translate")
            .header("Authorization", "Bearer $apiKey")
            .header("OpenAI-Beta", "realtime=v1")
            .build()
        ws = http.newWebSocket(
            request,
            object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    val update = JSONObject()
                        .put("type", "session.update")
                        .put(
                            "session",
                            JSONObject().put(
                                "audio",
                                JSONObject()
                                    .put(
                                        "input",
                                        JSONObject()
                                            .put("transcription", JSONObject().put("model", "gpt-realtime-whisper"))
                                            .put("noise_reduction", JSONObject().put("type", "near_field")),
                                    )
                                    .put("output", JSONObject().put("language", targetLanguage.ifBlank { "vi" })),
                            ),
                        )
                    webSocket.send(update.toString())
                    callbacks.onStatus(TranslationStatus.Streaming)
                }

                override fun onMessage(webSocket: WebSocket, text: String) {
                    handleEvent(JSONObject(text))
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    callbacks.onError(t.message ?: "OpenAI Realtime lỗi.")
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    callbacks.onStatus(TranslationStatus.Idle)
                }
            },
        )
    }

    override fun sendAudio(pcm: ByteArray) {
        val audio = Base64.encodeToString(pcm, Base64.NO_WRAP)
        ws?.send(JSONObject().put("type", "session.input_audio_buffer.append").put("audio", audio).toString())
    }

    override fun flushPending() {
        emitPending()
    }

    override fun disconnect() {
        ws?.close(1000, "stop")
        ws = null
    }

    private fun handleEvent(data: JSONObject) {
        val raw = data.optString("type")
        val type = raw.removePrefix("session.")
        when (type) {
            "input_transcript.delta",
            "conversation.item.input_audio_transcription.delta" -> {
                sourceBuffer += data.optString("delta")
                callbacks.onSourceProvisional(sourceBuffer)
            }
            "input_transcript.done",
            "input_audio_transcription.completed",
            "conversation.item.input_audio_transcription.completed" -> {
                val text = data.optString("transcript", data.optString("text"))
                if (text.isNotBlank()) {
                    pendingSources.addLast(text)
                    sourceBuffer = ""
                    callbacks.onSourceProvisional(text)
                }
            }
            "output_transcript.delta",
            "response.output_text.delta",
            "response.text.delta" -> {
                targetBuffer += data.optString("delta")
                callbacks.onTargetProvisional(targetBuffer)
            }
            "output_transcript.done",
            "response.output_text.done",
            "response.text.done" -> {
                val target = data.optString("transcript", data.optString("text", targetBuffer))
                val source = if (pendingSources.isNotEmpty()) pendingSources.removeFirst() else sourceBuffer
                sourceBuffer = ""
                targetBuffer = ""
                callbacks.onSegment(source, target)
            }
            "error" -> {
                val error = data.optJSONObject("error")
                callbacks.onError(error?.optString("message") ?: "OpenAI Realtime error.")
            }
        }
    }

    private fun emitPending() {
        val source = if (pendingSources.isNotEmpty()) pendingSources.removeFirst() else sourceBuffer
        if (source.isNotBlank() || targetBuffer.isNotBlank()) callbacks.onSegment(source, targetBuffer)
        sourceBuffer = ""
        targetBuffer = ""
    }
}

private class QwenRealtimeTranslator(
    private val apiKey: String,
    private val targetLanguage: String,
    private val callbacks: TranslationCallbacks,
) : RealtimeTranslationClient {
    private val http = OkHttpClient()
    private var ws: WebSocket? = null
    private val pendingSources = ArrayDeque<String>()
    private var sourceBuffer = ""
    private var targetBuffer = ""
    private var windowMs = 0.0
    private var silenceMs = 0.0
    private var hasSpeech = false

    override fun connect() {
        val request = Request.Builder()
            .url("wss://dashscope-intl.aliyuncs.com/api-ws/v1/realtime?model=qwen3.5-omni-plus-realtime")
            .header("Authorization", "Bearer $apiKey")
            .build()
        ws = http.newWebSocket(
            request,
            object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    val languageName = languageName(targetLanguage)
                    val session = JSONObject()
                        .put("modalities", JSONArray().put("text"))
                        .put("input_audio_format", "pcm")
                        .put("output_audio_format", "pcm")
                        .put("turn_detection", JSONObject.NULL)
                        .put(
                            "instructions",
                            "You are a professional interpreter. Translate every utterance into $languageName. Output only the translation.",
                        )
                    webSocket.send(JSONObject().put("type", "session.update").put("session", session).toString())
                    callbacks.onStatus(TranslationStatus.Streaming)
                }

                override fun onMessage(webSocket: WebSocket, text: String) {
                    handleEvent(JSONObject(text))
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    callbacks.onError(t.message ?: "Qwen Realtime lỗi.")
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    callbacks.onStatus(TranslationStatus.Idle)
                }
            },
        )
    }

    override fun sendAudio(pcm: ByteArray) {
        val socket = ws ?: return
        val audio = Base64.encodeToString(pcm, Base64.NO_WRAP)
        socket.send(JSONObject().put("type", "input_audio_buffer.append").put("audio", audio).toString())

        val chunkMs = pcm.size / (16_000.0 * 2.0) * 1000.0
        val rms = rmsInt16(pcm)
        windowMs += chunkMs
        if (rms >= 500.0) {
            silenceMs = 0.0
            hasSpeech = true
        } else {
            silenceMs += chunkMs
        }
        val hitPause = windowMs >= 2_000.0 && silenceMs >= 400.0
        val hitMax = windowMs >= 7_000.0
        if (hasSpeech && (hitPause || hitMax)) commitTurn()
        if (!hasSpeech && hitMax) resetWindow()
    }

    override fun flushPending() {
        if (hasSpeech) commitTurn()
        val source = if (pendingSources.isNotEmpty()) pendingSources.removeFirst() else sourceBuffer
        if (source.isNotBlank() || targetBuffer.isNotBlank()) callbacks.onSegment(source, targetBuffer)
        sourceBuffer = ""
        targetBuffer = ""
    }

    override fun disconnect() {
        ws?.close(1000, "stop")
        ws = null
    }

    private fun commitTurn() {
        ws?.send(JSONObject().put("type", "input_audio_buffer.commit").toString())
        ws?.send(JSONObject().put("type", "response.create").toString())
        resetWindow()
    }

    private fun resetWindow() {
        windowMs = 0.0
        silenceMs = 0.0
        hasSpeech = false
    }

    private fun handleEvent(data: JSONObject) {
        when (data.optString("type")) {
            "conversation.item.input_audio_transcription.delta" -> {
                sourceBuffer += data.optString("delta")
                callbacks.onSourceProvisional(sourceBuffer)
            }
            "conversation.item.input_audio_transcription.completed" -> {
                val text = data.optString("transcript", data.optString("text"))
                if (text.isNotBlank()) {
                    pendingSources.addLast(text)
                    sourceBuffer = ""
                    callbacks.onSourceProvisional(text)
                }
            }
            "response.text.delta", "response.audio_transcript.delta" -> {
                targetBuffer += data.optString("delta")
                callbacks.onTargetProvisional(targetBuffer)
            }
            "response.text.done", "response.audio_transcript.done" -> {
                val target = data.optString("text", data.optString("transcript", targetBuffer))
                val source = if (pendingSources.isNotEmpty()) pendingSources.removeFirst() else sourceBuffer
                sourceBuffer = ""
                targetBuffer = ""
                callbacks.onSegment(source, target)
            }
            "error" -> {
                val error = data.optJSONObject("error")
                callbacks.onError(error?.optString("message") ?: "Qwen Realtime error.")
            }
        }
    }
}

private class PcmAudioCapture(
    private val sampleRate: Int,
) {
    @Volatile private var running = false
    private var record: AudioRecord? = null
    private var thread: Thread? = null

    @SuppressLint("MissingPermission")
    fun start(onChunk: (ByteArray) -> Unit) {
        if (running) return
        val minBuffer = AudioRecord.getMinBufferSize(
            sampleRate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
        )
        val bufferSize = max(minBuffer, sampleRate / 5 * 2)
        val recorder = AudioRecord(
            MediaRecorder.AudioSource.VOICE_RECOGNITION,
            sampleRate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
            bufferSize,
        )
        if (recorder.state != AudioRecord.STATE_INITIALIZED) {
            recorder.release()
            throw IllegalStateException("Không mở được microphone.")
        }
        recorder.startRecording()
        record = recorder
        running = true
        thread = Thread({
            val buffer = ByteArray(bufferSize)
            while (running) {
                val read = recorder.read(buffer, 0, buffer.size)
                if (read > 0) onChunk(buffer.copyOf(read))
            }
        }, "bizclaw-translation-audio").also { it.start() }
    }

    fun stop() {
        running = false
        try {
            thread?.join(500)
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
        }
        thread = null
        try {
            record?.stop()
        } catch (_: Throwable) {
        }
        try {
            record?.release()
        } catch (_: Throwable) {
        }
        record = null
    }
}

private fun rmsInt16(pcm: ByteArray): Double {
    if (pcm.size < 2) return 0.0
    var sum = 0.0
    var count = 0
    var idx = 0
    while (idx + 1 < pcm.size) {
        val sample = (pcm[idx].toInt() and 0xff) or (pcm[idx + 1].toInt() shl 8)
        val signed = sample.toShort().toInt()
        sum += signed.toDouble() * signed.toDouble()
        count += 1
        idx += 2
    }
    return if (count == 0) 0.0 else sqrt(sum / count)
}

private fun languageName(code: String): String = when (code.lowercase()) {
    "vi" -> "Vietnamese"
    "en" -> "English"
    "zh", "zh-cn" -> "Chinese"
    "ja" -> "Japanese"
    "ko" -> "Korean"
    "fr" -> "French"
    "de" -> "German"
    "es" -> "Spanish"
    "th" -> "Thai"
    else -> code.ifBlank { "Vietnamese" }
}
