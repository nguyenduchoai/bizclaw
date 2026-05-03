package vn.bizclaw.app.ui.automation

import android.content.ComponentName
import android.content.Intent
import android.provider.Settings
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import vn.bizclaw.app.engine.GlobalLLM
import vn.bizclaw.app.engine.LocalAgent
import vn.bizclaw.app.engine.LocalAgentManager
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.service.BizClawAccessibilityService
import vn.bizclaw.app.service.BizClawNotificationListener
import vn.bizclaw.app.service.MamaAgent
import vn.bizclaw.app.service.MamaConfig

// ═══════════════════════════════════════════════════════════════
// Social app definitions
// ═══════════════════════════════════════════════════════════════

data class SocialApp(
    val emoji: String,
    val name: String,
    val packageName: String,
    val color: Color,
)

val SOCIAL_APPS = listOf(
    SocialApp("💬", "Zalo", "com.zing.zalo", Color(0xFF0068FF)),
    SocialApp("💙", "Messenger", "com.facebook.orca", Color(0xFF0084FF)),
    SocialApp("📘", "Facebook", "com.facebook.katana", Color(0xFF1877F2)),
    SocialApp("📸", "Instagram", "com.instagram.android", Color(0xFFE4405F)),
    SocialApp("🧵", "Threads", "com.instagram.barcelona", Color(0xFF000000)),
    SocialApp("📧", "Gmail", "com.google.android.gm", Color(0xFFEA4335)),
    SocialApp("📧", "Outlook", "com.microsoft.office.outlook", Color(0xFF0078D4)),
    SocialApp("💬", "Lark", "com.larksuite.suite", Color(0xFF3370FF)),
    SocialApp("💬", "Lark CN", "com.ss.android.lark", Color(0xFF3370FF)),
    SocialApp("✈️", "Telegram", "org.telegram.messenger", Color(0xFF26A5E4)),
)

// ═══════════════════════════════════════════════════════════════
// Automation Screen
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AutomationScreen(
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val manager = remember { LocalAgentManager(context) }
    val providerManager = remember { ProviderManager(context) }
    val mama = remember { MamaAgent(context) }
    var agents by remember { mutableStateOf(manager.loadAgents()) }
    var showCreateFlow by remember { mutableStateOf(false) }
    var mamaConfig by remember { mutableStateOf(mama.loadConfig()) }

    // Agent picker for flow activation
    var pendingTemplate by remember { mutableStateOf<FlowTemplate?>(null) }
    var showAgentPicker by remember { mutableStateOf(false) }

    // Service status
    val notifListenerConnected = BizClawNotificationListener.instance != null
    val accessibilityRunning = BizClawAccessibilityService.isRunning()
    val modelLoaded = GlobalLLM.instance.isLoaded

    // Recent notifications
    var recentNotifs by remember {
        mutableStateOf(BizClawNotificationListener.recentNotifications.toList())
    }

    // Listen for new notifications
    DisposableEffect(Unit) {
        val callback: (BizClawNotificationListener.SocialNotification) -> Unit = {
            recentNotifs = BizClawNotificationListener.recentNotifications.toList()
        }
        BizClawNotificationListener.onNotificationReceived = callback
        onDispose {
            BizClawNotificationListener.onNotificationReceived = null
        }
    }

    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()
    val autoAgents = agents.filter { it.autoReply }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("⚡ Tự Động Hoá", fontWeight = FontWeight.Bold)
                        Text(
                            if (autoAgents.isNotEmpty())
                                "${autoAgents.size} flow đang chạy • Zalo • FB • Email"
                            else
                                "Zalo • Facebook • Messenger • Email",
                            style = MaterialTheme.typography.labelSmall,
                            color = if (autoAgents.isNotEmpty())
                                Color(0xFF00E676)
                            else
                                MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Quay lại")
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // ─── Status Card ──────────────────────────────
            item {
                StatusCard(
                    modelLoaded = modelLoaded,
                    notifListener = notifListenerConnected,
                    accessibility = accessibilityRunning,
                    onEnableNotifListener = {
                        try {
                            val intent = Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)
                            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                            context.startActivity(intent)
                        } catch (_: Exception) {}
                    },
                    onEnableAccessibility = {
                        try {
                            val intent = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
                            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                            context.startActivity(intent)
                        } catch (_: Exception) {}
                    },
                )
            }

            // ─── Mama Agent Setup ──────────────────────────────
            item {
                Spacer(Modifier.height(8.dp))
                Text(
                    "👑 Mama Tổng Quản — Nhận Lệnh Zalo/Telegram",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = Color(0xFFFF6D00),
                )
                Spacer(Modifier.height(4.dp))
            }
            item {
                MamaSetupCard(
                    config = mamaConfig,
                    agents = agents,
                    onUpdateConfig = { newConfig ->
                        mama.saveConfig(newConfig)
                        mamaConfig = newConfig
                    },
                )
            }

            // ─── Active Automation Agents (SHOW FIRST) ──────────
            if (autoAgents.isNotEmpty()) {
                item {
                    Text(
                        "🟢 Flow Đang Chạy (${autoAgents.size})",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = Color(0xFF00E676),
                    )
                }

                items(autoAgents) { agent ->
                    ActiveAgentCard(
                        agent = agent,
                        onToggle = { enabled ->
                            manager.updateAgent(agent.copy(autoReply = enabled))
                            agents = manager.loadAgents()
                            coroutineScope.launch {
                                snackbarHostState.showSnackbar(
                                    if (enabled) "✅ ${agent.name} đã BẬT"
                                    else "⏸ ${agent.name} đã TẮT"
                                )
                            }
                        },
                        onDelete = {
                            manager.deleteAgent(agent.id)
                            agents = manager.loadAgents()
                        },
                    )
                }
            }

            // ─── Flow Templates ──────────────────────────────
            item {
                Text(
                    "⚡ Tạo Flow Mới",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    "Bấm [Dùng] để kích hoạt flow. Flow sẽ tự chạy khi nhận tin nhắn/email.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            items(FLOW_TEMPLATES) { template ->
                FlowTemplateCard(
                    template = template,
                    onUse = {
                        // Show agent picker first
                        pendingTemplate = template
                        showAgentPicker = true
                    },
                )
            }

            // ─── Recent Notifications ──────────────────────────
            if (recentNotifs.isNotEmpty()) {
                item {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            "📩 Tin nhắn gần đây (${recentNotifs.size})",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                        )
                        TextButton(onClick = {
                            BizClawNotificationListener.recentNotifications.clear()
                            recentNotifs = emptyList()
                        }) {
                            Text("Xoá hết")
                        }
                    }
                }

                items(recentNotifs.take(20)) { notif ->
                    NotificationCard(notif)
                }
            }
        }
    }

    // ─── Agent Picker Dialog ───────────────────────────────
    if (showAgentPicker && pendingTemplate != null) {
        val template = pendingTemplate!!
        val existingAgents = agents.filter { !it.id.startsWith("flow_") }
        val providers = remember { providerManager.loadProviders() }

        AlertDialog(
            onDismissRequest = {
                showAgentPicker = false
                pendingTemplate = null
            },
            title = {
                Text("🤖 Chọn Agent cho Flow")
            },
            text = {
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.heightIn(max = 400.dp),
                ) {
                    // Option 1: Use flow's built-in prompt (no agent)
                    item {
                        Card(
                            onClick = {
                                val agent = LocalAgent(
                                    id = "flow_${System.currentTimeMillis()}",
                                    emoji = template.emoji,
                                    name = template.name,
                                    role = template.description,
                                    systemPrompt = template.systemPrompt,
                                    triggerApps = template.targetApps,
                                    autoReply = true,
                                    providerId = providers.firstOrNull { it.enabled }?.id ?: "local_gguf",
                                )
                                manager.addAgent(agent)
                                agents = manager.loadAgents()
                                showAgentPicker = false
                                pendingTemplate = null
                                coroutineScope.launch {
                                    snackbarHostState.showSnackbar(
                                        "✅ Flow \"${template.name}\" đã kích hoạt!"
                                    )
                                }
                            },
                            colors = CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f),
                            ),
                        ) {
                            Row(
                                modifier = Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(template.emoji, fontSize = 24.sp)
                                Spacer(Modifier.width(10.dp))
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(
                                        "Dùng prompt mặc định",
                                        fontWeight = FontWeight.Bold,
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                    Text(
                                        template.systemPrompt.take(60) + "...",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 2,
                                    )
                                }
                            }
                        }
                    }

                    // Divider
                    if (existingAgents.isNotEmpty()) {
                        item {
                            Row(
                                modifier = Modifier.padding(vertical = 4.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                HorizontalDivider(modifier = Modifier.weight(1f))
                                Text(
                                    " hoặc chọn agent ",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                HorizontalDivider(modifier = Modifier.weight(1f))
                            }
                        }
                    }

                    // Option 2: Use an existing agent
                    items(existingAgents) { existingAgent ->
                        val agentProvider = providers.find { it.id == existingAgent.providerId }

                        Card(
                            onClick = {
                                // Create flow agent with existing agent's prompt + provider
                                val flowAgent = LocalAgent(
                                    id = "flow_${System.currentTimeMillis()}",
                                    emoji = existingAgent.emoji,
                                    name = "${template.name} (${existingAgent.name})",
                                    role = template.description,
                                    systemPrompt = existingAgent.systemPrompt,
                                    knowledgeBaseIds = existingAgent.knowledgeBaseIds,
                                    triggerApps = template.targetApps,
                                    autoReply = true,
                                    providerId = existingAgent.providerId,
                                    groupId = existingAgent.groupId,
                                )
                                manager.addAgent(flowAgent)
                                agents = manager.loadAgents()
                                showAgentPicker = false
                                pendingTemplate = null
                                coroutineScope.launch {
                                    snackbarHostState.showSnackbar(
                                        "✅ Flow \"${template.name}\" dùng agent ${existingAgent.name}!"
                                    )
                                }
                            },
                        ) {
                            Row(
                                modifier = Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(existingAgent.emoji, fontSize = 24.sp)
                                Spacer(Modifier.width(10.dp))
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(
                                        existingAgent.name,
                                        fontWeight = FontWeight.SemiBold,
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                    Text(
                                        existingAgent.role.ifBlank { existingAgent.systemPrompt.take(50) + "..." },
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                    // Show provider badge
                                    if (agentProvider != null) {
                                        Surface(
                                            shape = RoundedCornerShape(6.dp),
                                            color = Color(0xFF3370FF).copy(alpha = 0.15f),
                                            modifier = Modifier.padding(top = 2.dp),
                                        ) {
                                            Text(
                                                "⚡ ${agentProvider.name}",
                                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 1.dp),
                                                style = MaterialTheme.typography.labelSmall,
                                                color = Color(0xFF3370FF),
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = {
                    showAgentPicker = false
                    pendingTemplate = null
                }) {
                    Text("Huỷ")
                }
            },
        )
    }
}

// ═══════════════════════════════════════════════════════════════
// Status Card
// ═══════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════
// Mama Agent Setup Card
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MamaSetupCard(
    config: MamaConfig,
    agents: List<LocalAgent>,
    onUpdateConfig: (MamaConfig) -> Unit,
) {
    var bossText by remember(config) {
        mutableStateOf(config.bossContacts.joinToString(", "))
    }
    var expanded by remember { mutableStateOf(false) }
    val selectedAgent = agents.find { it.id == config.mamaAgentId }

    Card(
        colors = CardDefaults.cardColors(
            containerColor = if (config.enabled)
                Color(0xFFE65100).copy(alpha = 0.12f) // Orange glow
            else
                MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Header
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("👑", fontSize = 22.sp)
                Spacer(Modifier.width(8.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        "Mama Tổng Quản",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        if (config.enabled) "🟢 Đang hoạt động — chờ lệnh từ Zalo"
                        else "⚪ Chưa bật",
                        style = MaterialTheme.typography.labelSmall,
                        color = if (config.enabled) Color(0xFF00E676) else Color.Gray,
                    )
                }
                Switch(
                    checked = config.enabled,
                    onCheckedChange = { enabled ->
                        onUpdateConfig(config.copy(enabled = enabled))
                    },
                )
            }

            Spacer(Modifier.height(12.dp))

            // Boss contacts input
            OutlinedTextField(
                value = bossText,
                onValueChange = { bossText = it },
                label = { Text("Tên Zalo Boss (cách nhau dấu phẩy)") },
                placeholder = { Text("VD: Nguyễn Đức Hoài, 0901234567") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                trailingIcon = {
                    IconButton(onClick = {
                        val contacts = bossText.split(",").map { it.trim() }.filter { it.isNotBlank() }
                        onUpdateConfig(config.copy(bossContacts = contacts))
                    }) {
                        Icon(Icons.Default.Check, "Lưu")
                    }
                },
            )

            Spacer(Modifier.height(8.dp))

            // Agent selector
            ExposedDropdownMenuBox(
                expanded = expanded,
                onExpandedChange = { expanded = !expanded },
            ) {
                OutlinedTextField(
                    value = if (selectedAgent != null) "${selectedAgent.emoji} ${selectedAgent.name}" else "Chọn Agent...",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Agent xử lý lệnh") },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
                    modifier = Modifier
                        .menuAnchor()
                        .fillMaxWidth(),
                )
                ExposedDropdownMenu(
                    expanded = expanded,
                    onDismissRequest = { expanded = false },
                ) {
                    agents.filter { !it.id.startsWith("flow_") }.forEach { agent ->
                        DropdownMenuItem(
                            text = {
                                Text("${agent.emoji} ${agent.name} — ${agent.role.take(30)}")
                            },
                            onClick = {
                                onUpdateConfig(config.copy(mamaAgentId = agent.id))
                                expanded = false
                            },
                        )
                    }
                }
            }

            // Info text
            Spacer(Modifier.height(8.dp))
            Text(
                "💡 Gửi tin nhắn Zalo từ tên Boss → Mama AI phân tích → "
                    + "sai Agent làm → báo cáo lại Zalo",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun StatusCard(
    modelLoaded: Boolean,
    notifListener: Boolean,
    accessibility: Boolean,
    onEnableNotifListener: () -> Unit,
    onEnableAccessibility: () -> Unit,
) {
    val allReady = modelLoaded && notifListener && accessibility

    Card(
        colors = CardDefaults.cardColors(
            containerColor = if (allReady)
                Color(0xFF1B5E20).copy(alpha = 0.15f)
            else
                MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.5f),
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                if (allReady) "✅ Sẵn sàng tự động hoá" else "⚠️ Cần bật thêm quyền",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.Bold,
            )
            Spacer(Modifier.height(8.dp))

            StatusRow("🧠 Mô hình AI", modelLoaded, "Vào 🧠 AI Cục Bộ để tải")
            StatusRow("🔔 Đọc thông báo", notifListener, "Bấm để bật", onEnableNotifListener)
            StatusRow("♿ Điều khiển apps", accessibility, "Bấm để bật", onEnableAccessibility)
        }
    }
}

@Composable
private fun StatusRow(
    label: String,
    enabled: Boolean,
    hint: String = "",
    onEnable: (() -> Unit)? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            if (enabled) "✅" else "❌",
            fontSize = 14.sp,
        )
        Spacer(Modifier.width(8.dp))
        Text(
            label,
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.weight(1f),
        )
        if (!enabled && onEnable != null) {
            TextButton(
                onClick = onEnable,
                contentPadding = PaddingValues(horizontal = 8.dp, vertical = 0.dp),
            ) {
                Text(
                    hint,
                    style = MaterialTheme.typography.labelSmall,
                )
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Flow Templates
// ═══════════════════════════════════════════════════════════════

data class FlowTemplate(
    val emoji: String,
    val name: String,
    val description: String,
    val targetApps: List<String>,
    val systemPrompt: String,
)

val FLOW_TEMPLATES = listOf(
    FlowTemplate(
        emoji = "💬",
        name = "Trả lời Zalo CSKH",
        description = "Tự động trả lời tin nhắn Zalo bằng AI, dựa trên kho kiến thức",
        targetApps = listOf("com.zing.zalo"),
        systemPrompt = "Bạn là nhân viên chăm sóc khách hàng chuyên nghiệp. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Lịch sự, tận tâm, giải quyết vấn đề nhanh. " +
            "Nếu không biết, nói: 'Em sẽ chuyển yêu cầu cho bộ phận phụ trách ạ.'",
    ),
    FlowTemplate(
        emoji = "💙",
        name = "Trả lời Messenger",
        description = "Tự động trả lời tin nhắn Facebook Messenger",
        targetApps = listOf("com.facebook.orca"),
        systemPrompt = "Bạn là trợ lý bán hàng trên Facebook. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Tư vấn sản phẩm, báo giá, hướng dẫn mua hàng. " +
            "Khi có đơn, hỏi: tên, SĐT, địa chỉ giao hàng.",
    ),
    FlowTemplate(
        emoji = "📘",
        name = "Trả lời comment FB",
        description = "Tự động trả lời bình luận trên Facebook",
        targetApps = listOf("com.facebook.katana"),
        systemPrompt = "Bạn quản lý fanpage Facebook. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Trả lời comment ngắn gọn, thân thiện. " +
            "Hướng khách inbox để tư vấn chi tiết.",
    ),
    FlowTemplate(
        emoji = "📸",
        name = "Trả lời DM Instagram",
        description = "Tự động trả lời tin nhắn Instagram",
        targetApps = listOf("com.instagram.android"),
        systemPrompt = "Bạn quản lý tài khoản Instagram shop. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Giới thiệu sản phẩm, báo giá, link đặt hàng. " +
            "Phong cách trẻ trung, gần gũi.",
    ),
    FlowTemplate(
        emoji = "🔥",
        name = "Đa kênh (Zalo + FB + Mess)",
        description = "Trả lời tự động trên tất cả các kênh",
        targetApps = listOf("com.zing.zalo", "com.facebook.orca", "com.facebook.katana"),
        systemPrompt = "Bạn là trợ lý kinh doanh đa kênh. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Trả lời tin nhắn từ Zalo, Facebook, Messenger. " +
            "Tư vấn sản phẩm, giải đáp thắc mắc, hỗ trợ đặt hàng. " +
            "Chuyên nghiệp nhưng thân thiện.",
    ),
    // ─── Email Flows ──────────────────────────────
    FlowTemplate(
        emoji = "📧",
        name = "Kiểm tra & Trả lời Email",
        description = "Tự động đọc email mới, phân loại và soạn trả lời",
        targetApps = listOf("com.google.android.gm", "com.microsoft.office.outlook"),
        systemPrompt = "Bạn là trợ lý email chuyên nghiệp. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Khi nhận email, hãy: " +
            "1) Tóm tắt nội dung chính, " +
            "2) Phân loại: Quan trọng/Bình thường/Spam, " +
            "3) Soạn reply lịch sự, chuyên nghiệp. " +
            "Nếu email spam/quảng cáo, ghi chú: 'Bỏ qua—spam'.",
    ),
    FlowTemplate(
        emoji = "📋",
        name = "Phân loại Email tự động",
        description = "Tự động phân loại email: Quan trọng, Công việc, Spam",
        targetApps = listOf("com.google.android.gm"),
        systemPrompt = "Bạn là hệ thống phân loại email. CHỈ trả lời bằng tiếng Việt. " +
            "Phân loại email thành các nhóm: " +
            "🔴 KHẨN CẤP — cần trả lời ngay, " +
            "🟡 QUAN TRỌNG — trả lời trong ngày, " +
            "🟢 BÌNH THƯỜNG — trả lời khi rảnh, " +
            "⚪ SPAM — bỏ qua. " +
            "Kèm tóm tắt 1 dòng cho mỗi email.",
    ),
    FlowTemplate(
        emoji = "📊",
        name = "Tóm tắt Email hàng ngày",
        description = "Cuối ngày tổng hợp tất cả email chưa đọc",
        targetApps = listOf("com.google.android.gm", "com.microsoft.office.outlook"),
        systemPrompt = "Bạn là trợ lý tổng hợp email hàng ngày. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Tổng hợp tất cả email thành báo cáo ngắn gọn: " +
            "- Bao nhiêu email mới, " +
            "- Bao nhiêu cần trả lời, " +
            "- Tóm tắt email quan trọng nhất, " +
            "- Đề xuất hành động tiếp theo.",
    ),
    // ─── Lark Flows ──────────────────────────────
    FlowTemplate(
        emoji = "💬",
        name = "Trả lời Lark Chat",
        description = "Tự động trả lời tin nhắn Lark/Feishu bằng AI",
        targetApps = listOf("com.larksuite.suite", "com.ss.android.lark"),
        systemPrompt = "Bạn là trợ lý chuyên nghiệp trên Lark. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Trả lời tin nhắn công việc nhanh, rõ ràng. " +
            "Nếu cần xác nhận, hỏi lại ngắn gọn.",
    ),
    FlowTemplate(
        emoji = "📧",
        name = "Quản lý Lark Mail",
        description = "Tự động đọc và trả lời Lark Mail",
        targetApps = listOf("com.larksuite.suite", "com.ss.android.lark"),
        systemPrompt = "Bạn quản lý Lark Mail chuyên nghiệp. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Phân loại email: Khẩn/Quan trọng/Bình thường. " +
            "Soạn reply ngắn gọn, đúng form công việc.",
    ),
    // ─── Telegram Flows ──────────────────────────────
    FlowTemplate(
        emoji = "✈️",
        name = "Trả lời Telegram",
        description = "Tự động trả lời tin nhắn Telegram",
        targetApps = listOf("org.telegram.messenger"),
        systemPrompt = "Bạn là trợ lý trên Telegram. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Trả lời tin nhắn thân thiện, nhanh gọn. " +
            "Nếu không biết, nói: 'Mình sẽ kiểm tra và trả lời sau nhé.'",
    ),
    // ─── Threads Flows ──────────────────────────────
    FlowTemplate(
        emoji = "🧵",
        name = "Đăng bài Threads tự động",
        description = "Tự động đăng bài lên Threads từ lệnh Mama",
        targetApps = listOf("com.instagram.barcelona"),
        systemPrompt = "Bạn là content creator cho Threads. " +
            "CHỈ viết bằng tiếng Việt. " +
            "Viết nội dung ngắn gọn, viral, hay. " +
            "Thêm emoji, hashtag phù hợp. " +
            "Phong cách trẻ trung, sáng tạo.",
    ),
    // ─── Đa kênh mở rộng ──────────────────────────────
    FlowTemplate(
        emoji = "🌐",
        name = "Đa kênh toàn diện",
        description = "FB + IG + Threads + Zalo + Mess + Lark + Telegram",
        targetApps = listOf(
            "com.zing.zalo",
            "com.facebook.orca",
            "com.facebook.katana",
            "com.instagram.android",
            "com.instagram.barcelona",
            "com.larksuite.suite",
            "org.telegram.messenger",
        ),
        systemPrompt = "Bạn là trợ lý đa kênh toàn diện. " +
            "CHỈ trả lời bằng tiếng Việt. " +
            "Trả lời tin nhắn từ MỌI kênh: Zalo, Facebook, Messenger, Instagram, Threads, Lark, Telegram. " +
            "Tư vấn sản phẩm, giải đáp thắc mắc, hỗ trợ đặt hàng. " +
            "Chuyên nghiệp nhưng thân thiện.",
    ),
)

@Composable
private fun FlowTemplateCard(
    template: FlowTemplate,
    onUse: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp)) {
            // Header: emoji + name
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(template.emoji, fontSize = 22.sp)
                Spacer(Modifier.width(8.dp))
                Text(
                    template.name,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.weight(1f),
                )
                Button(
                    onClick = onUse,
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp),
                ) {
                    Text("Dùng")
                }
            }

            // Description
            Text(
                template.description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 30.dp, top = 2.dp),
            )

            // App badges
            Row(
                modifier = Modifier.padding(start = 30.dp, top = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                template.targetApps.forEach { pkg ->
                    val app = SOCIAL_APPS.find { it.packageName == pkg }
                    if (app != null) {
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = app.color.copy(alpha = 0.15f),
                        ) {
                            Text(
                                "${app.emoji} ${app.name}",
                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                                style = MaterialTheme.typography.labelSmall,
                                color = app.color,
                            )
                        }
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Active Agent Card
// ═══════════════════════════════════════════════════════════════

@Composable
private fun ActiveAgentCard(
    agent: LocalAgent,
    onToggle: (Boolean) -> Unit,
    onDelete: () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = if (agent.autoReply)
                Color(0xFF1B5E20).copy(alpha = 0.1f)
            else
                MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(agent.emoji, fontSize = 24.sp)
            Spacer(Modifier.width(8.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(agent.name, fontWeight = FontWeight.SemiBold)
                Text(
                    agent.role,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                // Show target apps
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    agent.triggerApps.forEach { pkg ->
                        val app = SOCIAL_APPS.find { it.packageName == pkg }
                        if (app != null) {
                            Text(
                                app.emoji,
                                style = MaterialTheme.typography.labelSmall,
                            )
                        }
                    }
                }
            }
            Switch(
                checked = agent.autoReply,
                onCheckedChange = onToggle,
            )
            IconButton(onClick = onDelete) {
                Icon(
                    Icons.Default.Delete,
                    "Xoá",
                    tint = MaterialTheme.colorScheme.error,
                    modifier = Modifier.size(20.dp),
                )
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Notification Card
// ═══════════════════════════════════════════════════════════════

@Composable
private fun NotificationCard(notif: BizClawNotificationListener.SocialNotification) {
    val app = SOCIAL_APPS.find { it.packageName == notif.packageName }

    Card(
        colors = CardDefaults.cardColors(
            containerColor = if (notif.replied)
                Color(0xFF1B5E20).copy(alpha = 0.08f)
            else
                MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(app?.emoji ?: "📱", fontSize = 16.sp)
                Spacer(Modifier.width(6.dp))
                Text(
                    "${notif.app} • ${notif.sender}",
                    style = MaterialTheme.typography.labelMedium,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.weight(1f),
                )
                if (notif.replied) {
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = Color(0xFF00E676).copy(alpha = 0.2f),
                    ) {
                        Text(
                            "✅ Đã trả lời",
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                            style = MaterialTheme.typography.labelSmall,
                            color = Color(0xFF00E676),
                        )
                    }
                }
            }
            Text(
                notif.message,
                style = MaterialTheme.typography.bodySmall,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            if (notif.replied && notif.replyContent.isNotBlank()) {
                Spacer(Modifier.height(4.dp))
                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f),
                ) {
                    Text(
                        "🤖 ${notif.replyContent}",
                        modifier = Modifier.padding(8.dp),
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 3,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}
