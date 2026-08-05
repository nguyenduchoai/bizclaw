package vn.bizclaw.agent.llm

import android.app.DownloadManager
import android.content.Context
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.delay

/** Progress of the multi-gigabyte model fetch. */
sealed interface DownloadState {
    data object Idle : DownloadState
    data class Running(val downloadedBytes: Long, val totalBytes: Long) : DownloadState {
        val percent: Int get() = if (totalBytes > 0) ((downloadedBytes * 100) / totalBytes).toInt() else 0
    }

    data object Done : DownloadState
    data class Failed(val reason: String) : DownloadState
}

/**
 * Fetches a `.litertlm` model via the system DownloadManager.
 *
 * DownloadManager rather than an in-process HTTP client: a 2.6 GB transfer has to
 * survive the app being backgrounded, the screen locking, and Wi-Fi hiccups, and it
 * resumes on its own.
 */
class ModelDownloader(private val context: Context) {

    var state by mutableStateOf<DownloadState>(DownloadState.Idle)
        private set

    private val manager: DownloadManager
        get() = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager

    private var activeId: Long? = null

    fun start(variant: ModelVariant) {
        if (variant.isInstalled(context)) {
            state = DownloadState.Done
            return
        }
        if (state is DownloadState.Running) return

        // A partial file from an earlier failure would make the size check lie.
        variant.fileIn(context).takeIf { it.exists() }?.delete()

        val request = DownloadManager.Request(Uri.parse(variant.downloadUrl))
            .setTitle("BizClaw · ${variant.label}")
            .setDescription("Đang tải model AI (${variant.downloadGb}) về máy")
            .setAllowedOverMetered(false)
            .setAllowedOverRoaming(false)
            .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
            .setDestinationInExternalFilesDir(context, null, variant.fileName)

        activeId = runCatching { manager.enqueue(request) }
            .onFailure { state = DownloadState.Failed(it.message ?: "Không khởi động được tải về") }
            .getOrNull()
        if (activeId != null) state = DownloadState.Running(0, variant.downloadBytes)
    }

    fun cancel() {
        activeId?.let { manager.remove(it) }
        activeId = null
        state = DownloadState.Idle
    }

    /** Polls DownloadManager until the transfer settles. Safe to call repeatedly. */
    suspend fun track(variant: ModelVariant) {
        while (true) {
            val id = activeId ?: return
            val query = DownloadManager.Query().setFilterById(id)
            manager.query(query).use { cursor ->
                if (!cursor.moveToFirst()) {
                    state = DownloadState.Failed("Tải về bị huỷ")
                    activeId = null
                    return
                }
                val status = cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS))
                val soFar = cursor.getLong(
                    cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_BYTES_DOWNLOADED_SO_FAR),
                )
                val total = cursor.getLong(
                    cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_TOTAL_SIZE_BYTES),
                ).takeIf { it > 0 } ?: variant.downloadBytes

                when (status) {
                    DownloadManager.STATUS_SUCCESSFUL -> {
                        state = DownloadState.Done
                        activeId = null
                        return
                    }

                    DownloadManager.STATUS_FAILED -> {
                        val reason = cursor.getInt(
                            cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_REASON),
                        )
                        state = DownloadState.Failed("Lỗi tải về (mã $reason)")
                        activeId = null
                        return
                    }

                    else -> state = DownloadState.Running(soFar, total)
                }
            }
            delay(1_000)
        }
    }
}
