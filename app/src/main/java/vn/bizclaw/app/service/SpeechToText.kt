package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Speech-to-Text Service - Simplified version for compilation.
 */
class SpeechToText(private val context: Context) {
    
    companion object {
        private const val TAG = "SpeechToText"
    }

    data class TranscriptionResult(
        val text: String,
        val success: Boolean,
        val provider: String = "placeholder",
    )

    suspend fun transcribe(
        audioFile: File,
        language: String = "vi",
    ): TranscriptionResult = withContext(Dispatchers.IO) {
        Log.w(TAG, "STT placeholder - configure API keys in Settings")
        TranscriptionResult(
            text = "[Cần cấu hình OpenAI API Key để sử dụng transcription]",
            success = false,
            provider = "placeholder",
        )
    }
}
