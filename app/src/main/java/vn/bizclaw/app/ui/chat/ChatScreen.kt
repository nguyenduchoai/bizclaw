package vn.bizclaw.app.ui.chat

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import androidx.compose.ui.platform.LocalContext
import vn.bizclaw.app.engine.GlobalLLM
import vn.bizclaw.app.engine.LocalAgentManager
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.engine.ProviderChat

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatScreen(
    viewModel: ChatViewModel,
    onOpenAgents: () -> Unit = {},
    onOpenSettings: () -> Unit = {},
    onOpenAutomation: () -> Unit = {},
    onOpenLocalLLM: () -> Unit = {},
    onOpenMeeting: () -> Unit = {},
) {
    val messages = viewModel.messages
    val isLoading by viewModel.isLoading
    val currentAgent by viewModel.currentAgent
    val isConnected by viewModel.isConnected
    val isLocalMode by viewModel.isLocalMode
    val localModelName by viewModel.localModelName
    val localGenSpeed by viewModel.localGenSpeed
    val localContextUsed by viewModel.localContextUsed
    val error by viewModel.error
    var inputText by remember { mutableStateOf("") }
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    // Agent selector state
    val agentManager = remember { LocalAgentManager(context) }
    val providerManager = remember { ProviderManager(context) }
    var localAgents by remember { mutableStateOf(agentManager.loadAgents()) }
    var selectedAgentId by remember { mutableStateOf<String?>(null) }
    var groups by remember { mutableStateOf(providerManager.loadGroups()) }
    var showGroupDialog by remember { mutableStateOf(false) }
    var editingGroup by remember { mutableStateOf<vn.bizclaw.app.engine.AgentGroup?>(null) }
    var selectedGroupId by remember { mutableStateOf<String?>(null) }
    var isGroupChatting by remember { mutableStateOf(false) }

    // Initialize chat history persistence
    LaunchedEffect(Unit) {
        viewModel.initHistoryDir(context)
    }

    // Auto-scroll to bottom on new messages + auto-save
    LaunchedEffect(messages.size, messages.lastOrNull()?.content) {
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(messages.size - 1)
            viewModel.autoSave()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("BizClaw", fontWeight = FontWeight.Bold)
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            // Determine effective status
                            val hasLocalModel = isLocalMode && viewModel.localLLM.isLoaded
                            val hasGlobalModel = GlobalLLM.instance.isLoaded
                            val effectiveConnected = isConnected || hasLocalModel || hasGlobalModel

                            // Status dot
                            Box(
                                modifier = Modifier
                                    .size(8.dp)
                                    .clip(CircleShape)
                                    .background(
                                        when {
                                            hasLocalModel -> Color(0xFF00E676)
                                            isLocalMode -> Color(0xFFFF9800)
                                            hasGlobalModel -> Color(0xFF00E676)
                                            isConnected -> MaterialTheme.colorScheme.secondary
                                            else -> MaterialTheme.colorScheme.error
                                        }
                                    )
                            )
                            Spacer(Modifier.width(6.dp))
                            Text(
                                text = when {
                                    hasLocalModel ->
                                        "🧠 $currentAgent — ${localModelName ?: "cục bộ"}"
                                    isLocalMode -> "🧠 Cục bộ (chưa tải mô hình)"
                                    hasGlobalModel ->
                                        "🧠 AI Cục Bộ sẵn sàng — bấm 🧠 để dùng"
                                    isConnected -> "🤖 $currentAgent"
                                    else -> "Bấm 🧠 để dùng AI cục bộ"
                                },
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                },
                actions = {
                    if (selectedGroupId != null) {
                        IconButton(onClick = {
                            editingGroup = groups.find { it.id == selectedGroupId }
                            showGroupDialog = true
                        }) {
                            Icon(Icons.Default.Edit, contentDescription = "Sửa nhóm", tint = MaterialTheme.colorScheme.primary)
                        }
                    }
                    // Toggle local/cloud mode
                    IconButton(onClick = {
                        if (isLocalMode) {
                            viewModel.isLocalMode.value = false
                            viewModel.currentAgent.value = "default"
                        } else if (viewModel.localLLM.isLoaded || GlobalLLM.instance.isLoaded) {
                            onOpenLocalLLM() // Go to LocalLLM chat (model already loaded)
                        } else {
                            onOpenLocalLLM()
                        }
                    }) {
                        Icon(
                            Icons.Default.Memory,
                            contentDescription = "AI Cục Bộ",
                            tint = if (isLocalMode) Color(0xFF00E676) else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(onClick = onOpenMeeting) {
                        Icon(
                            Icons.Default.Mic,
                            contentDescription = "Ghi Âm",
                            tint = Color(0xFFFF1744),
                        )
                    }
                    IconButton(onClick = onOpenAutomation) {
                        Icon(
                            Icons.Default.FlashOn,
                            contentDescription = "Tự Động Hoá",
                            tint = Color(0xFFFF9800),
                        )
                    }
                    IconButton(onClick = onOpenAgents) {
                        Icon(Icons.Default.Group, contentDescription = "Trợ lý")
                    }
                    IconButton(onClick = onOpenSettings) {
                        Icon(Icons.Default.Settings, contentDescription = "Cài đặt")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Local mode performance stats bar
            AnimatedVisibility(visible = isLocalMode && localGenSpeed > 0) {
                Surface(
                    color = Color(0xFF1B5E20).copy(alpha = 0.15f),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp, 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text("🧠", fontSize = 16.sp)
                        Text(
                            "⚡ %.1f tok/s".format(localGenSpeed),
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF00E676),
                        )
                        Text(
                            "📊 Ngữ cảnh: $localContextUsed",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            "💰 $0",
                            style = MaterialTheme.typography.labelSmall,
                            color = Color(0xFF00E676),
                        )
                    }
                }
            }

            // Error/info banner
            AnimatedVisibility(visible = error != null) {
                Surface(
                    color = if (error?.contains("auto-switch") == true)
                        Color(0xFF1B5E20).copy(alpha = 0.2f)
                    else
                        MaterialTheme.colorScheme.errorContainer,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = error ?: "",
                            modifier = Modifier.weight(1f),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.bodySmall,
                        )
                        IconButton(
                            onClick = { viewModel.error.value = null },
                            modifier = Modifier.size(20.dp),
                        ) {
                            Icon(Icons.Default.Close, "Đóng", Modifier.size(14.dp))
                        }
                    }
                }
            }

            // ─── Agent Selector Bar ───────────────────────────
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                modifier = Modifier.fillMaxWidth(),
            ) {
                LazyRow(
                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 6.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    // Group button
                    item {
                        FilterChip(
                            selected = false,
                            onClick = {
                                editingGroup = null
                                showGroupDialog = true
                            },
                            label = { Text("👥 Nhóm", maxLines = 1) },
                            leadingIcon = { Text("➕", fontSize = 12.sp) },
                        )
                    }

                    // Agent chips
                    items(localAgents) { agent ->
                        val isSelected = selectedAgentId == agent.id
                        FilterChip(
                            selected = isSelected,
                            onClick = {
                                if (isSelected) {
                                    selectedAgentId = null
                                    selectedGroupId = null
                                    viewModel.currentAgent.value = "BizClaw"
                                    viewModel.switchToConversation("default")
                                } else {
                                    // Don't switch agent while generating
                                    if (isLoading || isGroupChatting) return@FilterChip
                                    selectedAgentId = agent.id
                                    selectedGroupId = null
                                    viewModel.currentAgent.value = agent.name
                                    viewModel.switchToConversation("agent_${agent.id}")
                                    // Safe system prompt update
                                    scope.launch {
                                        try {
                                            if (GlobalLLM.instance.isLoaded) {
                                                val prompt = agentManager.buildPromptForAgent(agent, "")
                                                GlobalLLM.instance.addSystemPrompt(prompt)
                                            }
                                        } catch (e: Exception) {
                                            // Ignore — model might be busy
                                        }
                                    }
                                }
                            },
                            label = {
                                Text(
                                    "${agent.emoji} ${agent.name}",
                                    maxLines = 1,
                                    style = MaterialTheme.typography.labelMedium,
                                )
                            },
                            colors = FilterChipDefaults.filterChipColors(
                                selectedContainerColor = MaterialTheme.colorScheme.primaryContainer,
                            ),
                        )
                    }

                    // Existing groups
                    items(groups) { group ->
                        val isGroupSelected = selectedGroupId == group.id
                        FilterChip(
                            selected = isGroupSelected,
                            onClick = {
                                if (isGroupSelected) {
                                    selectedGroupId = null
                                    selectedAgentId = null
                                    viewModel.currentAgent.value = "BizClaw"
                                    viewModel.switchToConversation("default")
                                } else {
                                    if (isLoading || isGroupChatting) return@FilterChip
                                    selectedGroupId = group.id
                                    selectedAgentId = null
                                    viewModel.switchToConversation("group_${group.id}")
                                    val memberNames = group.agentIds.mapNotNull { id ->
                                        localAgents.find { it.id == id }?.name
                                    }.joinToString(", ")
                                    viewModel.currentAgent.value = "${group.emoji} ${group.name}"
                                }
                            },
                            label = {
                                val memberCount = group.agentIds.size
                                Text(
                                    "${group.emoji} ${group.name} ($memberCount)",
                                    maxLines = 1,
                                    style = MaterialTheme.typography.labelMedium,
                                )
                            },
                            colors = FilterChipDefaults.filterChipColors(
                                selectedContainerColor = MaterialTheme.colorScheme.tertiaryContainer,
                                containerColor = MaterialTheme.colorScheme.tertiaryContainer.copy(alpha = 0.5f),
                            ),
                        )
                    }
                }
            }

            // Messages
            LazyColumn(
                state = listState,
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                // Welcome message
                if (messages.isEmpty()) {
                    item {
                        WelcomeCard(
                            isLocalMode = isLocalMode,
                            localModelName = localModelName,
                            onSuggestionClick = { suggestion ->
                                inputText = suggestion
                                viewModel.sendMessage(suggestion)
                                inputText = ""
                            },
                        )
                    }
                }

                items(messages) { message ->
                    ChatBubble(message)
                }
            }

            // Input bar
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier
                        .padding(horizontal = 12.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = inputText,
                        onValueChange = { inputText = it },
                        modifier = Modifier.weight(1f),
                        placeholder = {
                            Text(
                                if (isLocalMode) "Hỏi BizClaw cục bộ..."
                                else "Nhắn tin cho $currentAgent..."
                            )
                        },
                        shape = RoundedCornerShape(24.dp),
                        maxLines = 4,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedBorderColor = if (isLocalMode) Color(0xFF00E676)
                            else MaterialTheme.colorScheme.primary,
                            unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f),
                        ),
                    )
                    Spacer(Modifier.width(8.dp))
                    FilledIconButton(
                        onClick = {
                            val msg = inputText.trim()
                            if (msg.isBlank()) return@FilledIconButton
                            inputText = ""

                            val activeGroup = selectedGroupId?.let { gid ->
                                groups.find { it.id == gid }
                            }

                            if (activeGroup != null) {
                                // ─── GROUP CHAT: tất cả agent trả lời ───
                                val providers = providerManager.loadProviders()
                                val groupAgents = activeGroup.agentIds.mapNotNull { aid ->
                                    localAgents.find { it.id == aid }
                                }

                                // Add user message
                                viewModel.addUserMessage(msg)
                                isGroupChatting = true

                                scope.launch {
                                    try {
                                        sendGroupMessage(
                                            msg = msg,
                                            group = activeGroup,
                                            agents = localAgents,
                                            providers = providers,
                                            agentManager = agentManager,
                                            viewModel = viewModel,
                                        )
                                    } catch (e: Exception) {
                                        viewModel.addGroupResponse(
                                            agentEmoji = "⚠️",
                                            agentName = "Hệ thống",
                                            providerName = "Error",
                                            content = "❌ Lỗi nhóm chat: ${e.message?.take(80)}",
                                        )
                                    }
                                    isGroupChatting = false
                                }
                            } else {
                                // ─── SINGLE AGENT CHAT ───
                                viewModel.sendMessage(msg)
                            }
                        },
                        enabled = inputText.isNotBlank() && !isLoading && !isGroupChatting,
                        modifier = Modifier.size(48.dp),
                        colors = IconButtonDefaults.filledIconButtonColors(
                            containerColor = if (isLocalMode) Color(0xFF00E676)
                            else MaterialTheme.colorScheme.primary,
                        ),
                    ) {
                        if (isLoading) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                strokeWidth = 2.dp,
                            )
                        } else {
                            Icon(
                                Icons.AutoMirrored.Filled.Send,
                                contentDescription = "Gửi",
                            )
                        }
                    }
                }
            }
        }
    }

    if (showGroupDialog) {
        GroupEditDialog(
            agents = localAgents,
            editingGroup = editingGroup,
            onDismiss = {
                showGroupDialog = false
                editingGroup = null
            },
            onSave = { group ->
                if (editingGroup == null) {
                    providerManager.addGroup(group)
                } else {
                    providerManager.updateGroup(group)
                }
                groups = providerManager.loadGroups()
                // Auto switch to newly created group or updated group
                selectedGroupId = group.id
                selectedAgentId = null
                viewModel.switchToConversation("group_${group.id}")
                viewModel.currentAgent.value = "${group.emoji} ${group.name}"
                showGroupDialog = false
                editingGroup = null
            },
            onDelete = { group ->
                providerManager.deleteGroup(group.id)
                groups = providerManager.loadGroups()
                if (selectedGroupId == group.id) {
                    selectedGroupId = null
                    selectedAgentId = null
                    viewModel.currentAgent.value = "BizClaw"
                    viewModel.switchToConversation("default")
                }
                showGroupDialog = false
                editingGroup = null
            }
        )
    }
}

@Composable
fun ChatBubble(message: UiMessage) {
    val isUser = message.role == "user"

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
    ) {
        if (!isUser) {
            // Agent avatar
            Surface(
                shape = CircleShape,
                color = if (message.isLocal) Color(0xFF1B5E20) else MaterialTheme.colorScheme.primary,
                modifier = Modifier
                    .size(32.dp)
                    .padding(top = 4.dp),
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Text(if (message.isLocal) "🧠" else "🤖", fontSize = 16.sp)
                }
            }
            Spacer(Modifier.width(8.dp))
        }

        Surface(
            shape = RoundedCornerShape(
                topStart = 16.dp,
                topEnd = 16.dp,
                bottomStart = if (isUser) 16.dp else 4.dp,
                bottomEnd = if (isUser) 4.dp else 16.dp,
            ),
            color = if (isUser)
                MaterialTheme.colorScheme.primary
            else
                MaterialTheme.colorScheme.surfaceVariant,
            modifier = Modifier.widthIn(max = 300.dp),
        ) {
            Column(modifier = Modifier.padding(12.dp)) {
                // Agent name for assistant messages
                if (!isUser && message.agentName.isNotEmpty()) {
                    Text(
                        text = message.agentName,
                        style = MaterialTheme.typography.labelSmall,
                        color = if (message.isLocal) Color(0xFF00E676) else MaterialTheme.colorScheme.primary,
                    )
                    Spacer(Modifier.height(2.dp))
                }

                Text(
                    text = message.content.ifEmpty { "..." },
                    color = if (isUser)
                        MaterialTheme.colorScheme.onPrimary
                    else
                        MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodyMedium,
                )

                // Tool actions log (for agent mode)
                if (message.toolActions.isNotBlank()) {
                    Spacer(Modifier.height(8.dp))
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = Color(0xFF1B5E20).copy(alpha = 0.1f),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Column(modifier = Modifier.padding(8.dp)) {
                            Text(
                                "🔧 Hành động Agent",
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFF00E676),
                            )
                            Spacer(Modifier.height(4.dp))
                            Text(
                                text = message.toolActions.trim(),
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.8f),
                                fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                                lineHeight = 16.sp,
                            )
                        }
                    }
                }

                // Streaming indicator
                if (message.isStreaming) {
                    Spacer(Modifier.height(4.dp))
                    LinearProgressIndicator(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(2.dp),
                        color = if (message.isLocal) Color(0xFF00E676) else MaterialTheme.colorScheme.secondary,
                    )
                }

                // Stats row (tokens or tok/s)
                if (message.tokensUsed > 0 || message.tokPerSec > 0) {
                    Spacer(Modifier.height(4.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        if (message.tokPerSec > 0) {
                            Text(
                                text = "⚡ %.1f tok/s".format(message.tokPerSec),
                                style = MaterialTheme.typography.labelSmall,
                                color = Color(0xFF00E676),
                            )
                        }
                        if (message.tokensUsed > 0) {
                            Text(
                                text = "${message.tokensUsed} tokens",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                            )
                        }
                        if (message.isLocal) {
                            Text(
                                text = "📱 cục bộ",
                                style = MaterialTheme.typography.labelSmall,
                                color = Color(0xFF00E676).copy(alpha = 0.7f),
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun WelcomeCard(
    isLocalMode: Boolean = false,
    localModelName: String? = null,
    onSuggestionClick: (String) -> Unit = {},
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 24.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
        shape = RoundedCornerShape(20.dp),
    ) {
        Column(
            modifier = Modifier
                .padding(24.dp)
                .fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            // Logo
            Surface(
                shape = RoundedCornerShape(16.dp),
                color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.5f),
                modifier = Modifier.size(64.dp),
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Text(if (isLocalMode) "🧠" else "🤖", fontSize = 36.sp)
                }
            }

            Spacer(Modifier.height(16.dp))

            Text(
                "BizClaw",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
            )

            // Status badge
            Surface(
                shape = RoundedCornerShape(12.dp),
                color = if (isLocalMode && localModelName != null)
                    Color(0xFF00E676).copy(alpha = 0.15f)
                else
                    MaterialTheme.colorScheme.secondaryContainer.copy(alpha = 0.5f),
            ) {
                Text(
                    if (isLocalMode && localModelName != null)
                        "🧠 $localModelName • Offline • Miễn phí"
                    else if (isLocalMode)
                        "📦 Chưa tải mô hình — Bấm 🧠"
                    else
                        "🌐 AI Agent Platform",
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
                    style = MaterialTheme.typography.labelMedium,
                    color = if (isLocalMode && localModelName != null)
                        Color(0xFF00E676)
                    else
                        MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            Spacer(Modifier.height(12.dp))

            Text(
                "Chọn Agent ở thanh cuộn bên trên, hoặc bấm gợi ý:",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Spacer(Modifier.height(12.dp))

            // Quick action chips — NOW FUNCTIONAL
            val suggestions = listOf(
                "Viết caption FB bán hàng cho sản phẩm mới",
                "Phân tích báo cáo doanh thu tháng này",
                "Soạn email gửi khách hàng",
                "Tư vấn CSKH cho khách hỏi giá",
            )
            val labels = listOf(
                "💡 Caption FB",
                "📊 Báo cáo",
                "📧 Email",
                "💬 CSKH",
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                suggestions.take(2).forEachIndexed { idx, text ->
                    SuggestionChip(
                        onClick = { onSuggestionClick(text) },
                        label = { Text(labels[idx], style = MaterialTheme.typography.labelSmall) },
                        modifier = Modifier.weight(1f),
                    )
                }
            }
            Spacer(Modifier.height(4.dp))
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                suggestions.drop(2).forEachIndexed { idx, text ->
                    SuggestionChip(
                        onClick = { onSuggestionClick(text) },
                        label = { Text(labels[idx + 2], style = MaterialTheme.typography.labelSmall) },
                        modifier = Modifier.weight(1f),
                    )
                }
            }
        }
    }
}
