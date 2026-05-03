package vn.bizclaw.app.engine

import android.app.DownloadManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Environment
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Model Download Manager — download GGUF models from HuggingFace for on-device inference.
 *
 * Features:
 * - Download from HuggingFace URLs via Android DownloadManager
 * - Progress tracking with real-time updates
 * - Resume downloads after interruption
 * - Manage downloaded models (list, delete)
 * - Storage management (check available space)
 *
 * Models are stored in: app_data/models/{filename}.gguf
 */
class ModelDownloadManager(private val context: Context) {
    companion object {
        private const val TAG = "ModelDownload"
        private const val MODELS_DIR = "models"
    }

    // ── Download state ──
    private val _downloadState = MutableStateFlow<DownloadState>(DownloadState.Idle)
    val downloadState: StateFlow<DownloadState> = _downloadState.asStateFlow()

    private val _downloadedModels = MutableStateFlow<List<LocalModel>>(emptyList())
    val downloadedModels: StateFlow<List<LocalModel>> = _downloadedModels.asStateFlow()

    private var currentDownloadId: Long = -1
    private val downloadManager = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager

    init {
        refreshModelList()
    }

    // ═══════════════════════════════════════════════════════════
    // Download
    // ═══════════════════════════════════════════════════════════

    /**
     * Start downloading a GGUF model from URL.
     * Uses Android DownloadManager for background download with progress.
     */
    fun downloadModel(model: DownloadableModel): Long {
        val modelsDir = getModelsDir()
        val fileName = model.url.substringAfterLast("/")
        val destFile = File(modelsDir, fileName)

        // Skip if already downloaded
        if (destFile.exists() && destFile.length() > 0) {
            Log.i(TAG, "Model already exists: $fileName (${destFile.length()} bytes)")
            _downloadState.value = DownloadState.Completed(destFile.absolutePath)
            return -1
        }

        // Check available storage
        val freeSpace = modelsDir.usableSpace
        if (freeSpace < model.sizeBytes * 1.1) { // 10% buffer
            _downloadState.value = DownloadState.Error(
                "Not enough storage. Need ${model.sizeDisplay}, available: ${freeSpace / 1_000_000_000}GB"
            )
            return -1
        }

        Log.i(TAG, "Starting download: ${model.name} → $fileName (dest: ${destFile.absolutePath})")

        return try {
            val request = DownloadManager.Request(Uri.parse(model.url)).apply {
                setTitle("BizClaw: ${model.name}")
                setDescription("Downloading ${model.sizeDisplay} model for local AI")
                setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
                setDestinationUri(Uri.fromFile(destFile))
                setAllowedOverMetered(true)
                setAllowedOverRoaming(false)
            }

            currentDownloadId = downloadManager.enqueue(request)
            _downloadState.value = DownloadState.Downloading(0, model.sizeBytes)

            // Register completion receiver
            registerDownloadReceiver()

            // Start progress polling
            pollDownloadProgress(currentDownloadId, model.sizeBytes)

            currentDownloadId
        } catch (e: SecurityException) {
            Log.e(TAG, "SecurityException during download: ${e.message}", e)
            _downloadState.value = DownloadState.Error(
                "Download permission denied: ${e.message}"
            )
            -1
        } catch (e: Exception) {
            Log.e(TAG, "Download failed: ${e.message}", e)
            _downloadState.value = DownloadState.Error(
                "Download error: ${e.message}"
            )
            -1
        }
    }

    /**
     * Cancel ongoing download.
     */
    fun cancelDownload() {
        if (currentDownloadId != -1L) {
            downloadManager.remove(currentDownloadId)
            currentDownloadId = -1
            _downloadState.value = DownloadState.Idle
            Log.i(TAG, "Download cancelled")
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Model Management
    // ═══════════════════════════════════════════════════════════

    /**
     * Get list of downloaded models on device.
     */
    fun refreshModelList() {
        val modelsDir = getModelsDir()
        val models = modelsDir.listFiles { file -> file.extension == "gguf" }
            ?.map { file ->
                LocalModel(
                    name = file.nameWithoutExtension
                        .replace("-", " ")
                        .replace(".", " ")
                        .replaceFirstChar { it.uppercase() },
                    fileName = file.name,
                    path = file.absolutePath,
                    sizeBytes = file.length(),
                    lastModified = file.lastModified(),
                )
            }
            ?.sortedByDescending { it.lastModified }
            ?: emptyList()

        _downloadedModels.value = models
        Log.i(TAG, "Found ${models.size} downloaded models")
    }

    /**
     * Delete a downloaded model from device storage.
     */
    fun deleteModel(model: LocalModel): Boolean {
        val file = File(model.path)
        val deleted = file.delete()
        if (deleted) {
            Log.i(TAG, "Deleted model: ${model.fileName}")
            refreshModelList()
        }
        return deleted
    }

    /**
     * Get available storage space in bytes.
     */
    fun getAvailableSpace(): Long {
        return getModelsDir().usableSpace
    }

    /**
     * Get total models storage used in bytes.
     */
    fun getUsedSpace(): Long {
        return getModelsDir().listFiles()
            ?.filter { it.extension == "gguf" }
            ?.sumOf { it.length() }
            ?: 0L
    }

    // ═══════════════════════════════════════════════════════════
    // Internal
    // ═══════════════════════════════════════════════════════════

    private fun getModelsDir(): File {
        // Use external files dir — DownloadManager cannot write to internal filesDir
        val externalDir = context.getExternalFilesDir(null)
        val dir = if (externalDir != null) {
            File(externalDir, MODELS_DIR)
        } else {
            // Fallback to internal (won't work with DownloadManager but at least list works)
            File(context.filesDir, MODELS_DIR)
        }
        if (!dir.exists()) dir.mkdirs()
        return dir
    }

    private fun pollDownloadProgress(downloadId: Long, totalBytes: Long) {
        Thread {
            while (currentDownloadId == downloadId) {
                val query = DownloadManager.Query().setFilterById(downloadId)
                val cursor = downloadManager.query(query)

                if (cursor.moveToFirst()) {
                    val statusIdx = cursor.getColumnIndex(DownloadManager.COLUMN_STATUS)
                    val bytesIdx = cursor.getColumnIndex(DownloadManager.COLUMN_BYTES_DOWNLOADED_SO_FAR)

                    val status = cursor.getInt(statusIdx)
                    val bytesDownloaded = cursor.getLong(bytesIdx)

                    when (status) {
                        DownloadManager.STATUS_RUNNING -> {
                            _downloadState.value = DownloadState.Downloading(bytesDownloaded, totalBytes)
                        }
                        DownloadManager.STATUS_SUCCESSFUL -> {
                            val uri = cursor.getString(
                                cursor.getColumnIndex(DownloadManager.COLUMN_LOCAL_URI)
                            )
                            val path = Uri.parse(uri).path ?: ""
                            _downloadState.value = DownloadState.Completed(path)
                            refreshModelList()
                            cursor.close()
                            return@Thread
                        }
                        DownloadManager.STATUS_FAILED -> {
                            val reasonIdx = cursor.getColumnIndex(DownloadManager.COLUMN_REASON)
                            val reason = cursor.getInt(reasonIdx)
                            _downloadState.value = DownloadState.Error("Download failed (reason: $reason)")
                            cursor.close()
                            return@Thread
                        }
                    }
                }
                cursor.close()

                try { Thread.sleep(500) } catch (_: InterruptedException) { return@Thread }
            }
        }.start()
    }

    private fun registerDownloadReceiver() {
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context?, intent: Intent?) {
                val id = intent?.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1) ?: return
                if (id == currentDownloadId) {
                    currentDownloadId = -1
                    refreshModelList()
                    try { context.unregisterReceiver(this) } catch (_: Exception) {}
                }
            }
        }
        context.registerReceiver(
            receiver,
            IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE),
            Context.RECEIVER_NOT_EXPORTED,
        )
    }
}

// ═══════════════════════════════════════════════════════════════
// State types
// ═══════════════════════════════════════════════════════════════

sealed class DownloadState {
    data object Idle : DownloadState()
    data class Downloading(val bytesDownloaded: Long, val totalBytes: Long) : DownloadState() {
        val progress: Float get() = if (totalBytes > 0) bytesDownloaded.toFloat() / totalBytes else 0f
        val percentDisplay: String get() = "${(progress * 100).toInt()}%"
        val downloadedDisplay: String get() {
            val mb = bytesDownloaded / 1_000_000
            val totalMb = totalBytes / 1_000_000
            return "${mb}MB / ${totalMb}MB"
        }
    }
    data class Completed(val modelPath: String) : DownloadState()
    data class Error(val message: String) : DownloadState()
}

data class LocalModel(
    val name: String,
    val fileName: String,
    val path: String,
    val sizeBytes: Long,
    val lastModified: Long,
) {
    val sizeDisplay: String
        get() {
            val gb = sizeBytes / 1_000_000_000.0
            return if (gb >= 1.0) "%.1f GB".format(gb) else "${sizeBytes / 1_000_000} MB"
        }
}
