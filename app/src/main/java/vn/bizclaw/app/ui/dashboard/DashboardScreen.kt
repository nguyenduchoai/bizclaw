package vn.bizclaw.app.ui.dashboard

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import vn.bizclaw.app.service.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val capabilities = remember { DeviceCapabilities(context) }
    var isDaemonRunning by remember { mutableStateOf(BizClawDaemonService.isRunning()) }

    // Business Dashboard
    val dashboard = remember { UnifiedDashboard(context) }
    var dashboardData by remember { mutableStateOf<UnifiedDashboard.DashboardData?>(null) }
    var isRefreshing by remember { mutableStateOf(false) }

    // Initialize and refresh
    LaunchedEffect(Unit) {
        dashboard.initialize()
        dashboard.refresh()
        dashboardData = dashboard.dashboardData.value
    }

    // Auto-refresh
    LaunchedEffect(dashboard.dashboardData) {
        dashboardData = dashboard.dashboardData.value
    }

    val battery = remember { capabilities.getBatteryInfo() }
    val storage = remember { capabilities.getStorageInfo() }
    val network = remember { capabilities.getNetworkInfo() }
    val device = remember { capabilities.getDeviceInfo() }
    val oemWarning = remember { capabilities.getOemBatteryKillerWarning() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Bảng Điều Khiển", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Quay lại")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // ─── Daemon Control ───────────────────────────────────────

            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (isDaemonRunning)
                        MaterialTheme.colorScheme.primaryContainer
                    else
                        MaterialTheme.colorScheme.surfaceVariant,
                ),
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        if (isDaemonRunning) "🤖" else "😴",
                        fontSize = 48.sp,
                    )
                    Spacer(Modifier.height(8.dp))
                    Text(
                        if (isDaemonRunning) "Agent đang chạy" else "Agent đã dừng",
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(16.dp))

                    Button(
                        onClick = {
                            if (isDaemonRunning) {
                                BizClawDaemonService.stop(context)
                            } else {
                                BizClawDaemonService.start(context)
                            }
                            isDaemonRunning = !isDaemonRunning
                        },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = if (isDaemonRunning)
                                MaterialTheme.colorScheme.error
                            else
                                MaterialTheme.colorScheme.primary,
                        ),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Icon(
                            if (isDaemonRunning) Icons.Default.Stop else Icons.Default.PlayArrow,
                            null,
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(if (isDaemonRunning) "Dừng Agent" else "Khởi động Agent")
                    }
                }
            }

            // ─── OEM Warning ──────────────────────────────────────────

            if (oemWarning != null) {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                    ),
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(Icons.Default.Warning, null, tint = MaterialTheme.colorScheme.tertiary)
                        Spacer(Modifier.width(12.dp))
                        Text(oemWarning, style = MaterialTheme.typography.bodySmall)
                    }
                }
            }

            // ─── Device Stats Grid ────────────────────────────────────

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.BatteryChargingFull,
                    label = "Pin",
                    value = "${battery.level}%",
                    subtext = if (battery.isCharging) "Đang sạc" else "${battery.temperatureCelsius}°C",
                    color = when {
                        battery.level > 50 -> MaterialTheme.colorScheme.secondary
                        battery.level > 20 -> MaterialTheme.colorScheme.tertiary
                        else -> MaterialTheme.colorScheme.error
                    },
                )
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.Storage,
                    label = "Bộ nhớ",
                    value = "${storage.usedPercent}%",
                    subtext = "%.1f GB trống".format(storage.freeGb),
                    color = if (storage.usedPercent < 80) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.error,
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.Wifi,
                    label = "Mạng",
                    value = network.type.uppercase(),
                    subtext = network.wifiSsid ?: if (network.isConnected) "Đã kết nối" else "Ngoại tuyến",
                    color = if (network.isConnected) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.error,
                )
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.Memory,
                    label = "CPU",
                    value = "${device.cpuCores} nhân",
                    subtext = "${device.freeRamMb} MB RAM trống",
                    color = MaterialTheme.colorScheme.primary,
                )
            }

            // ─── Device Info ──────────────────────────────────────────

            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                ),
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Thiết bị", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                    Spacer(Modifier.height(8.dp))
                    InfoRow("Hãng", device.manufacturer)
                    InfoRow("Model", device.model)
                    InfoRow("Android", "${device.androidVersion} (SDK ${device.sdkVersion})")
                    InfoRow("BizClaw", "v0.6.2")
                }
            }

            // ─── Business Dashboard ───────────────────────────────

            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f),
                ),
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text("📊 Dashboard", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                        IconButton(
                            onClick = {
                                isRefreshing = true
                                scope.launch {
                                    dashboard.refresh()
                                    dashboardData = dashboard.dashboardData.value
                                    isRefreshing = false
                                }
                            },
                            enabled = !isRefreshing,
                        ) {
                            if (isRefreshing) {
                                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                            } else {
                                Icon(Icons.Default.Refresh, "Refresh")
                            }
                        }
                    }

                    Spacer(Modifier.height(12.dp))

                    dashboardData?.let { data ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceEvenly,
                        ) {
                            MetricItem("Health", "${data.healthScore.toInt()}%", Icons.Default.Favorite)
                            MetricItem("Unread", "${data.totalUnread}", Icons.Default.Email)
                            MetricItem("Pending", "${data.totalPending}", Icons.Default.Schedule)
                        }
                    }
                }
            }

            Spacer(Modifier.height(16.dp))
        }
    }
}

@Composable
fun StatCard(
    modifier: Modifier = Modifier,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    label: String,
    value: String,
    subtext: String,
    color: Color,
) {
    Card(modifier = modifier, shape = RoundedCornerShape(16.dp)) {
        Column(modifier = Modifier.padding(16.dp)) {
            Icon(icon, label, tint = color, modifier = Modifier.size(24.dp))
            Spacer(Modifier.height(8.dp))
            Text(value, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold, color = color)
            Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(subtext, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f))
        }
    }
}

@Composable
fun InfoRow(label: String, value: String) {
    Row(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Text(label, modifier = Modifier.width(80.dp), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.Medium)
    }
}

@Composable
fun MetricItem(label: String, value: String, icon: androidx.compose.ui.graphics.vector.ImageVector) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Icon(icon, label, modifier = Modifier.size(20.dp), tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.height(4.dp))
        Text(value, style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold)
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
