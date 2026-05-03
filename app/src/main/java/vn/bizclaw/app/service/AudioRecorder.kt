package vn.bizclaw.app.service

import android.content.Context
import android.media.MediaRecorder
import android.os.Build
import android.util.Log
import kotlinx.coroutines.*
import java.io.File
import java.text.SimpleDateFormat
import java.util.*

/**
 * AudioRecorder — VivaDicta-inspired voice recording for BizClaw.
 *
 * Records audio on-device, saves to local storage, then the file can be:
 * 1. Transcribed locally (Whisper on-device) or via cloud API
 * 2. Sent to gateway for AI-powered recap/summary
 *
 * Recording flow:
 * [User taps Record] → MediaRecorder captures → M4A file saved
 * → User taps Stop → File path available for transcription
 * → Send to gateway: POST /api/v1/voice/transcribe (multipart upload)
 * → Gateway calls voice_transcribe tool → returns text + recap
 */
class AudioRecorder(private val context: Context) {

    companion object {
        const val TAG = "AudioRecorder"
        private const val RECORDINGS_DIR = "recordings"
    }

    private var recorder: MediaRecorder? = null
    private var currentFile: File? = null
    private var startTime: Long = 0
    private var _isRecording = false

    val isRecording: Boolean get() = _isRecording

    /** Get the directory where recordings are saved. */
    private fun recordingsDir(): File {
        val dir = File(context.filesDir, RECORDINGS_DIR)
        if (!dir.exists()) dir.mkdirs()
        return dir
    }

    /**
     * Start recording audio.
     *
     * @return true if recording started successfully
     */
    fun startRecording(): Boolean {
        if (_isRecording) {
            Log.w(TAG, "Already recording")
            return false
        }

        try {
            // Generate filename with timestamp
            val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.getDefault()).format(Date())
            currentFile = File(recordingsDir(), "bizclaw_${timestamp}.m4a")

            recorder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                MediaRecorder(context)
            } else {
                @Suppress("DEPRECATION")
                MediaRecorder()
            }

            recorder?.apply {
                setAudioSource(MediaRecorder.AudioSource.MIC)
                setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
                setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
                setAudioChannels(1)          // Mono — smaller file, better for speech
                setAudioSamplingRate(16000)   // 16kHz — optimal for Whisper
                setAudioEncodingBitRate(64000) // 64kbps — good quality speech
                setOutputFile(currentFile?.absolutePath)
                prepare()
                start()
            }

            _isRecording = true
            startTime = System.currentTimeMillis()
            Log.i(TAG, "🎙️ Recording started: ${currentFile?.name}")
            return true
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to start recording: ${e.message}")
            cleanup()
            return false
        }
    }

    /**
     * Stop recording and return the saved file path.
     *
     * @return RecordingResult with file path and duration, or null if failed
     */
    fun stopRecording(): RecordingResult? {
        if (!_isRecording) {
            Log.w(TAG, "Not recording")
            return null
        }

        try {
            recorder?.apply {
                stop()
                release()
            }

            val duration = System.currentTimeMillis() - startTime
            val file = currentFile

            cleanup()

            if (file != null && file.exists() && file.length() > 0) {
                val sizeKB = file.length() / 1024
                Log.i(TAG, "✅ Recording saved: ${file.name} (${sizeKB}KB, ${duration/1000}s)")

                return RecordingResult(
                    filePath = file.absolutePath,
                    fileName = file.name,
                    durationMs = duration,
                    sizeBytes = file.length(),
                )
            } else {
                Log.e(TAG, "❌ Recording file missing or empty")
                return null
            }
        } catch (e: Exception) {
            Log.e(TAG, "❌ Failed to stop recording: ${e.message}")
            cleanup()
            return null
        }
    }

    /** Cancel recording and delete the file. */
    fun cancelRecording() {
        try {
            recorder?.apply {
                stop()
                release()
            }
        } catch (_: Exception) {}

        currentFile?.delete()
        cleanup()
        Log.i(TAG, "🗑️ Recording cancelled")
    }

    /** Get recording duration so far (in milliseconds). */
    fun getElapsedMs(): Long {
        return if (_isRecording) System.currentTimeMillis() - startTime else 0
    }

    /** List all saved recordings. */
    fun listRecordings(): List<RecordingResult> {
        return recordingsDir().listFiles()
            ?.filter { it.extension == "m4a" }
            ?.sortedByDescending { it.lastModified() }
            ?.map { file ->
                RecordingResult(
                    filePath = file.absolutePath,
                    fileName = file.name,
                    durationMs = 0, // Unknown for saved files
                    sizeBytes = file.length(),
                )
            } ?: emptyList()
    }

    /** Delete a specific recording. */
    fun deleteRecording(filePath: String): Boolean {
        val file = File(filePath)
        return if (file.exists() && file.parentFile == recordingsDir()) {
            file.delete()
        } else false
    }

    private fun cleanup() {
        recorder = null
        currentFile = null
        _isRecording = false
        startTime = 0
    }

    data class RecordingResult(
        val filePath: String,
        val fileName: String,
        val durationMs: Long,
        val sizeBytes: Long,
    ) {
        val durationFormatted: String
            get() {
                val secs = durationMs / 1000
                val mins = secs / 60
                val remainSecs = secs % 60
                return "${mins}:${String.format("%02d", remainSecs)}"
            }

        val sizeFormatted: String
            get() {
                return when {
                    sizeBytes < 1024 -> "${sizeBytes}B"
                    sizeBytes < 1024 * 1024 -> "${sizeBytes / 1024}KB"
                    else -> "${sizeBytes / (1024 * 1024)}MB"
                }
            }
    }
}
