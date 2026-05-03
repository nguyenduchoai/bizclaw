package vn.bizclaw.app.service

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiManager
import android.os.BatteryManager
import android.os.Build
import android.os.Environment
import android.os.StatFs
import android.provider.Settings
import androidx.core.content.ContextCompat
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Device Capabilities — what agents can do on THIS phone.
 *
 * This is the key differentiator from a thin client:
 * Agents can query device state, access sensors, send notifications,
 * and control phone features.
 */
class DeviceCapabilities(private val context: Context) {

    private val json = Json { prettyPrint = true }

    // ─── Battery ──────────────────────────────────────────────────

    fun getBatteryInfo(): BatteryInfo {
        val batteryIntent = context.registerReceiver(
            null,
            IntentFilter(Intent.ACTION_BATTERY_CHANGED)
        )
        val level = batteryIntent?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
        val scale = batteryIntent?.getIntExtra(BatteryManager.EXTRA_SCALE, 100) ?: 100
        val status = batteryIntent?.getIntExtra(BatteryManager.EXTRA_STATUS, -1) ?: -1
        val isCharging = status == BatteryManager.BATTERY_STATUS_CHARGING
                || status == BatteryManager.BATTERY_STATUS_FULL
        val temperature = (batteryIntent?.getIntExtra(
            BatteryManager.EXTRA_TEMPERATURE, 0
        ) ?: 0) / 10.0

        return BatteryInfo(
            level = (level * 100) / scale,
            isCharging = isCharging,
            temperatureCelsius = temperature,
        )
    }

    // ─── Storage ──────────────────────────────────────────────────

    fun getStorageInfo(): StorageInfo {
        val stat = StatFs(Environment.getDataDirectory().path)
        val totalBytes = stat.totalBytes
        val freeBytes = stat.freeBytes
        return StorageInfo(
            totalGb = totalBytes / (1024.0 * 1024 * 1024),
            freeGb = freeBytes / (1024.0 * 1024 * 1024),
            usedPercent = ((totalBytes - freeBytes) * 100.0 / totalBytes).toInt(),
        )
    }

    // ─── Network ──────────────────────────────────────────────────

    fun getNetworkInfo(): NetworkInfo {
        val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = cm.activeNetwork
        val caps = cm.getNetworkCapabilities(network)

        val type = when {
            caps == null -> "none"
            caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
            caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
            caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> "ethernet"
            else -> "unknown"
        }

        val wifiSsid = if (type == "wifi") {
            try {
                val wm = context.applicationContext.getSystemService(
                    Context.WIFI_SERVICE
                ) as WifiManager
                @Suppress("DEPRECATION")
                wm.connectionInfo?.ssid?.removeSurrounding("\"") ?: "unknown"
            } catch (_: Exception) {
                "unknown"
            }
        } else null

        return NetworkInfo(
            type = type,
            isConnected = network != null,
            wifiSsid = wifiSsid,
        )
    }

    // ─── Location ─────────────────────────────────────────────────

    fun isLocationAvailable(): Boolean {
        val hasPermission = ContextCompat.checkSelfPermission(
            context, Manifest.permission.ACCESS_FINE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED

        val lm = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
        val isEnabled = lm.isProviderEnabled(LocationManager.GPS_PROVIDER)
                || lm.isProviderEnabled(LocationManager.NETWORK_PROVIDER)

        return hasPermission && isEnabled
    }

    // ─── Device Info ──────────────────────────────────────────────

    fun getDeviceInfo(): DeviceInfo {
        val runtime = Runtime.getRuntime()
        return DeviceInfo(
            manufacturer = Build.MANUFACTURER,
            model = Build.MODEL,
            androidVersion = Build.VERSION.RELEASE,
            sdkVersion = Build.VERSION.SDK_INT,
            cpuCores = runtime.availableProcessors(),
            totalRamMb = runtime.maxMemory() / (1024 * 1024),
            freeRamMb = runtime.freeMemory() / (1024 * 1024),
            deviceId = Settings.Secure.getString(
                context.contentResolver,
                Settings.Secure.ANDROID_ID
            ) ?: "unknown",
        )
    }

    // ─── Full Status (for agent context) ──────────────────────────

    fun getFullStatus(): String {
        val status = DeviceStatus(
            device = getDeviceInfo(),
            battery = getBatteryInfo(),
            storage = getStorageInfo(),
            network = getNetworkInfo(),
            locationAvailable = isLocationAvailable(),
            daemonRunning = BizClawDaemonService.isRunning(),
        )
        return json.encodeToString(status)
    }

    // ─── OEM Battery Killer Detection ─────────────────────────────

    fun getOemBatteryKillerWarning(): String? {
        val manufacturer = Build.MANUFACTURER.lowercase()
        return when {
            manufacturer.contains("xiaomi") || manufacturer.contains("redmi") ->
                "Xiaomi/Redmi: Bật AutoStart + tắt Battery Optimization cho BizClaw"
            manufacturer.contains("samsung") ->
                "Samsung: Thêm BizClaw vào 'Unmonitored apps' trong Device Care"
            manufacturer.contains("huawei") || manufacturer.contains("honor") ->
                "Huawei/Honor: Tắt 'Manage automatically' cho BizClaw trong Battery"
            manufacturer.contains("oppo") || manufacturer.contains("realme") ->
                "OPPO/Realme: Bật AutoStart + 'Allow background activity' cho BizClaw"
            manufacturer.contains("vivo") ->
                "Vivo: Bật 'Allow AutoStart' + 'High background power' cho BizClaw"
            manufacturer.contains("oneplus") ->
                "OnePlus: Tắt Battery Optimization cho BizClaw"
            else -> null
        }
    }
}

// ─── Data Classes ─────────────────────────────────────────────────────

@Serializable
data class BatteryInfo(
    val level: Int,
    val isCharging: Boolean,
    val temperatureCelsius: Double,
)

@Serializable
data class StorageInfo(
    val totalGb: Double,
    val freeGb: Double,
    val usedPercent: Int,
)

@Serializable
data class NetworkInfo(
    val type: String,
    val isConnected: Boolean,
    val wifiSsid: String? = null,
)

@Serializable
data class DeviceInfo(
    val manufacturer: String,
    val model: String,
    val androidVersion: String,
    val sdkVersion: Int,
    val cpuCores: Int,
    val totalRamMb: Long,
    val freeRamMb: Long,
    val deviceId: String,
)

@Serializable
data class DeviceStatus(
    val device: DeviceInfo,
    val battery: BatteryInfo,
    val storage: StorageInfo,
    val network: NetworkInfo,
    val locationAvailable: Boolean,
    val daemonRunning: Boolean,
)
