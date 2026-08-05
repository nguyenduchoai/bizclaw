package vn.bizclaw.agent.ui

import android.content.Intent
import android.provider.Settings
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.agent.AgentService
import vn.bizclaw.agent.data.ReplyState
import vn.bizclaw.agent.llm.DownloadState
import vn.bizclaw.agent.llm.GemmaEngine
import vn.bizclaw.agent.llm.ModelVariant
import vn.bizclaw.agent.messaging.MessageListenerService
import vn.bizclaw.agent.messaging.SupportedApps

@Composable
fun HomeScreen(app: BizClawApp) {
    val context = LocalContext.current
    val settings = app.settings
    val variant = settings.modelVariant
    val downloadState = app.downloader.state
    val modelReady = variant.isInstalled(context)
    val notifGranted = MessageListenerService.isGranted(context)
    val chatApps = SupportedApps.installed(context)

    LaunchedEffect(downloadState) {
        if (downloadState is DownloadState.Running) app.downloader.track(variant)
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                MetricCard("Chờ duyệt", app.exchanges.countBy(ReplyState.DRAFT).toString(), Modifier.weight(1f))
                MetricCard("Đã gửi", app.exchanges.countBy(ReplyState.SENT).toString(), Modifier.weight(1f))
            }
        }

        item {
            SectionCard("Checklist khởi động") {
                CheckRow(
                    "Model ${variant.label} (${variant.downloadGb})",
                    modelReady,
                    "Bấm Tải model bên dưới. Nên dùng Wi-Fi.",
                )
                CheckRow(
                    "Quyền đọc thông báo",
                    notifGranted,
                    "Cần bật để agent thấy tin nhắn Zalo/Messenger.",
                )
                CheckRow(
                    "App chat đã cài (${chatApps.size})",
                    chatApps.isNotEmpty(),
                    "Chưa thấy Zalo hoặc Messenger trên máy.",
                )
                CheckRow(
                    "Đã nạp thông tin cửa hàng (${app.knowledge.docs.size})",
                    app.knowledge.docs.isNotEmpty(),
                    "Không có tài liệu thì agent sẽ luôn chuyển việc cho anh.",
                )
            }
        }

        item {
            SectionCard("Model AI") {
                Text(
                    "RAM máy: ${ModelVariant.totalRamMb(context)} MB · " +
                        "${variant.label} cần ~${variant.peakRamMb} MB",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                when (val s = downloadState) {
                    is DownloadState.Running -> {
                        Text("Đang tải ${s.percent}% (${s.downloadedBytes / 1_000_000} MB)")
                        LinearProgressIndicator(
                            progress = { s.percent / 100f },
                            modifier = Modifier.fillMaxWidth(),
                        )
                        OutlinedButton(onClick = { app.downloader.cancel() }) { Text("Huỷ tải") }
                    }

                    is DownloadState.Failed ->
                        Text(s.reason, color = Bad, style = MaterialTheme.typography.bodySmall)

                    else -> Unit
                }
                if (!modelReady && downloadState !is DownloadState.Running) {
                    Button(
                        onClick = { app.downloader.start(variant) },
                        modifier = Modifier.fillMaxWidth(),
                    ) { Text("Tải model ${variant.label}") }
                }
                if (modelReady) {
                    StatusPill(
                        if (GemmaEngine.isReady) "Đã nạp · ${GemmaEngine.activeBackend}" else "Đã tải về máy",
                        Ok,
                    )
                }
                GemmaEngine.lastError?.let {
                    Text(it, color = Bad, style = MaterialTheme.typography.bodySmall)
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    ModelVariant.entries.forEach { option ->
                        OutlinedButton(
                            onClick = { settings.modelVariant = option },
                            enabled = option != variant,
                            modifier = Modifier.weight(1f),
                        ) { Text(option.label) }
                    }
                }
            }
        }

        item {
            SectionCard("Bật agent") {
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("Tự động đọc tin nhắn đến")
                    Switch(
                        checked = settings.agentEnabled,
                        enabled = modelReady && notifGranted,
                        onCheckedChange = { on ->
                            settings.agentEnabled = on
                            if (on) AgentService.start(context) else AgentService.stop(context)
                        },
                    )
                }
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("Tự gửi không cần duyệt")
                    Switch(
                        checked = settings.autoSend,
                        onCheckedChange = { settings.autoSend = it },
                    )
                }
                Text(
                    "Tắt = agent chỉ soạn sẵn, anh bấm Gửi ở tab Hộp thư. " +
                        "Câu hỏi thiếu dữ liệu luôn chờ duyệt kể cả khi bật.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (!notifGranted) {
                    Button(
                        onClick = {
                            context.startActivity(
                                Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS),
                            )
                        },
                        modifier = Modifier.fillMaxWidth(),
                    ) { Text("Cấp quyền đọc thông báo") }
                }
            }
        }

        item { BatteryHint() }
    }
}

/**
 * Xiaomi/OPPO/Vivo/Samsung skins kill background services on their own schedule, which
 * shows up as the agent silently answering nothing overnight. Nothing in the app can
 * override that, so the fix is a one-tap route to the right settings screen.
 */
@Composable
private fun BatteryHint() {
    val context = LocalContext.current
    val vendor = android.os.Build.MANUFACTURER.lowercase()
    val advice = when {
        vendor.contains("xiaomi") || vendor.contains("redmi") || vendor.contains("poco") ->
            "HyperOS/MIUI: bật Tự khởi động (Autostart) và đặt Tiết kiệm pin = Không giới hạn."
        vendor.contains("samsung") ->
            "Samsung: thêm app vào danh sách Ứng dụng không bị giám sát (Unmonitored apps)."
        vendor.contains("oppo") || vendor.contains("realme") || vendor.contains("oneplus") ->
            "ColorOS: bật Tự khởi động và cho phép hoạt động nền."
        vendor.contains("vivo") ->
            "Funtouch: cho phép Tự khởi động và mức tiêu thụ pin nền cao."
        vendor.contains("huawei") || vendor.contains("honor") ->
            "EMUI: tắt 'Quản lý tự động' cho app này."
        else ->
            "Đặt tiết kiệm pin cho app này về mức Không giới hạn."
    }

    SectionCard("Chống hệ thống tắt agent") {
        Text(advice, style = MaterialTheme.typography.bodySmall)
        Text(
            "Không làm bước này thì agent hay chết lặng lẽ sau vài giờ.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        OutlinedButton(
            onClick = {
                context.startActivity(
                    Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                        data = android.net.Uri.parse("package:${context.packageName}")
                    },
                )
            },
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Mở cài đặt app") }
    }
}
