package vn.bizclaw.agent.llm

import android.app.ActivityManager
import android.content.Context
import java.io.File

/**
 * Gemma 4 builds that LiteRT-LM can run on a phone.
 *
 * Sizes and peak-RAM figures are the published on-device numbers; they matter because
 * exceeding available RAM does not degrade, it kills the process at load time.
 */
enum class ModelVariant(
    val label: String,
    val fileName: String,
    val downloadUrl: String,
    val downloadBytes: Long,
    val peakRamMb: Int,
) {
    E2B(
        label = "Gemma 4 E2B",
        fileName = "gemma-4-E2B-it.litertlm",
        downloadUrl = "https://huggingface.co/litert-community/gemma-4-E2B-it-litert-lm/" +
            "resolve/main/gemma-4-E2B-it.litertlm?download=true",
        downloadBytes = 2_580_000_000L,
        peakRamMb = 1_733,
    ),
    E4B(
        label = "Gemma 4 E4B",
        fileName = "gemma-4-E4B-it.litertlm",
        downloadUrl = "https://huggingface.co/litert-community/gemma-4-E4B-it-litert-lm/" +
            "resolve/main/gemma-4-E4B-it.litertlm?download=true",
        downloadBytes = 3_650_000_000L,
        peakRamMb = 3_283,
    );

    val downloadGb: String get() = String.format("%.1f GB", downloadBytes / 1_000_000_000.0)

    /** Where the finished model lives. Kept in app-specific storage so uninstall reclaims it. */
    fun fileIn(context: Context): File =
        File(context.getExternalFilesDir(null) ?: context.filesDir, fileName)

    fun isInstalled(context: Context): Boolean {
        val file = fileIn(context)
        // A DownloadManager failure can leave a short file behind; treat that as absent.
        return file.exists() && file.length() > downloadBytes / 2
    }

    companion object {
        fun from(name: String?): ModelVariant =
            entries.firstOrNull { it.name == name } ?: E2B

        /** Largest variant this device can realistically load, or null if none fit. */
        fun recommendedFor(context: Context): ModelVariant? {
            val totalMb = totalRamMb(context)
            // Headroom for the OS and the chat app we are replying into.
            return entries.sortedByDescending { it.peakRamMb }
                .firstOrNull { totalMb >= it.peakRamMb + 2_000 }
        }

        fun totalRamMb(context: Context): Int {
            val am = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
            val info = ActivityManager.MemoryInfo()
            am.getMemoryInfo(info)
            return (info.totalMem / (1024 * 1024)).toInt()
        }
    }
}
