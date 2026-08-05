package vn.bizclaw.agent.messaging

import android.content.Context
import android.content.pm.PackageManager
import vn.bizclaw.agent.data.Channel

/** Chat apps the agent watches, and how their notifications map to a [Channel]. */
object SupportedApps {

    private val byPackage = mapOf(
        "com.zing.zalo" to Channel.ZALO,
        "com.facebook.orca" to Channel.MESSENGER,
        "com.facebook.mlite" to Channel.MESSENGER,
        "com.facebook.katana" to Channel.FACEBOOK,
    )

    val packages: Set<String> get() = byPackage.keys

    fun channelOf(packageName: String): Channel? = byPackage[packageName]

    fun installed(context: Context): List<Pair<String, Channel>> =
        byPackage.entries
            .filter { isInstalled(context, it.key) }
            .map { it.key to it.value }

    private fun isInstalled(context: Context, packageName: String): Boolean = runCatching {
        context.packageManager.getPackageInfo(packageName, 0)
        true
    }.getOrDefault(false)
}
