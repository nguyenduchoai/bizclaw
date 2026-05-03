package vn.bizclaw.app.ui.agents

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
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
import vn.bizclaw.app.engine.AIProvider
import vn.bizclaw.app.engine.LocalAgent
import vn.bizclaw.app.engine.LocalAgentManager
import vn.bizclaw.app.engine.LocalRAG
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.engine.ProviderType

// ═══════════════════════════════════════════════════════════════
// Emoji options for agent creation
// ═══════════════════════════════════════════════════════════════
private val EMOJI_OPTIONS = listOf(
    "🤖", "📝", "📊", "🎯", "💬", "🧠", "🔥", "⚡", "🎨", "📱",
    "💼", "🏪", "📦", "🛒", "📣", "🎤", "📞", "✈️", "🏥", "🎓",
)

// ═══════════════════════════════════════════════════════════════
// Main Agents Screen
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AgentsScreen(
    onSelectAgent: (LocalAgent) -> Unit,
    onOpenKB: () -> Unit,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val manager = remember { LocalAgentManager(context) }
    var agents by remember { mutableStateOf(manager.loadAgents()) }
    var showCreateDialog by remember { mutableStateOf(false) }
    var editingAgent by remember { mutableStateOf<LocalAgent?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("Trợ Lý AI", fontWeight = FontWeight.Bold)
                        Text(
                            "${agents.size} agent",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Quay lại")
                    }
                },
                actions = {
                    IconButton(onClick = onOpenKB) {
                        Icon(Icons.Default.MenuBook, "Kho kiến thức")
                    }
                    IconButton(onClick = { showCreateDialog = true }) {
                        Icon(Icons.Default.Add, "Tạo agent mới")
                    }
                },
            )
        },
        // FAB removed — use + in TopAppBar instead (FAB was blocking edit buttons)
    ) { padding ->
        if (agents.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("🤖", fontSize = 64.sp)
                    Spacer(Modifier.height(16.dp))
                    Text("Chưa có agent nào", style = MaterialTheme.typography.titleMedium)
                    Spacer(Modifier.height(8.dp))
                    Button(onClick = { showCreateDialog = true }) {
                        Icon(Icons.Default.Add, null)
                        Spacer(Modifier.width(8.dp))
                        Text("Tạo agent đầu tiên")
                    }
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.padding(padding),
                contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 80.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(agents) { agent ->
                    LocalAgentCard(
                        agent = agent,
                        onSelect = { onSelectAgent(agent) },
                        onEdit = { editingAgent = agent },
                        onRag = onOpenKB,
                        onDelete = {
                            manager.deleteAgent(agent.id)
                            agents = manager.loadAgents()
                        },
                    )
                }
            }
        }
    }

    // Create dialog
    if (showCreateDialog) {
        AgentFormDialog(
            title = "Tạo Agent Mới",
            agent = null,
            knowledgeBases = LocalRAG.listKnowledgeBases(context),
            onDismiss = { showCreateDialog = false },
            onSave = { agent ->
                manager.addAgent(agent)
                agents = manager.loadAgents()
                showCreateDialog = false
            },
        )
    }

    // Edit dialog
    editingAgent?.let { agent ->
        AgentFormDialog(
            title = "Sửa Agent",
            agent = agent,
            knowledgeBases = LocalRAG.listKnowledgeBases(context),
            onDismiss = { editingAgent = null },
            onSave = { updated ->
                manager.updateAgent(updated)
                agents = manager.loadAgents()
                editingAgent = null
            },
        )
    }
}

// ═══════════════════════════════════════════════════════════════
// Agent Card
// ═══════════════════════════════════════════════════════════════

@Composable
private fun LocalAgentCard(
    agent: LocalAgent,
    onSelect: () -> Unit,
    onEdit: () -> Unit,
    onRag: () -> Unit,
    onDelete: () -> Unit,
) {
    var showMenu by remember { mutableStateOf(false) }

    Card(
        onClick = onSelect,
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Emoji avatar
            Surface(
                shape = CircleShape,
                color = MaterialTheme.colorScheme.primaryContainer,
                modifier = Modifier.size(48.dp),
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Text(agent.emoji, fontSize = 24.sp)
                }
            }

            Spacer(Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    agent.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    agent.role.ifEmpty { "Agent" },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                // RAG badges
                if (agent.knowledgeBaseIds.isNotEmpty()) {
                    Spacer(Modifier.height(4.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        agent.knowledgeBaseIds.forEach { kbId ->
                            Surface(
                                shape = RoundedCornerShape(12.dp),
                                color = MaterialTheme.colorScheme.tertiary.copy(alpha = 0.15f),
                            ) {
                                Text(
                                    "📚 $kbId",
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.tertiary,
                                )
                            }
                        }
                    }
                }
                // Auto-reply
                if (agent.autoReply) {
                    Text(
                        "🔔 Tự động trả lời",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.secondary,
                    )
                }
            }

            // Menu
            Box {
                IconButton(onClick = { showMenu = true }) {
                    Icon(Icons.Default.MoreVert, "Menu")
                }
                DropdownMenu(
                    expanded = showMenu,
                    onDismissRequest = { showMenu = false },
                ) {
                    DropdownMenuItem(
                        text = { Text("✏️ Sửa") },
                        onClick = { showMenu = false; onEdit() },
                    )
                    DropdownMenuItem(
                        text = { Text("📚 Quản lý kiến thức (RAG)") },
                        onClick = { showMenu = false; onRag() },
                    )
                    HorizontalDivider()
                    DropdownMenuItem(
                        text = { Text("🗑 Xoá", color = MaterialTheme.colorScheme.error) },
                        onClick = { showMenu = false; onDelete() },
                    )
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Agent Create/Edit Dialog
// ═══════════════════════════════════════════════════════════════

@Composable
private fun AgentFormDialog(
    title: String,
    agent: LocalAgent?,
    knowledgeBases: List<String>,
    onDismiss: () -> Unit,
    onSave: (LocalAgent) -> Unit,
) {
    var name by remember { mutableStateOf(agent?.name ?: "") }
    var emoji by remember { mutableStateOf(agent?.emoji ?: "🤖") }
    var role by remember { mutableStateOf(agent?.role ?: "") }
    var systemPrompt by remember { mutableStateOf(agent?.systemPrompt ?: "") }
    var selectedKBs by remember { mutableStateOf(agent?.knowledgeBaseIds?.toSet() ?: emptySet()) }
    var autoReply by remember { mutableStateOf(agent?.autoReply ?: false) }
    var selectedProvider by remember { mutableStateOf(agent?.providerId ?: "local_gguf") }

    val context = LocalContext.current
    val providers = remember { ProviderManager(context).loadProviders() }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title, fontWeight = FontWeight.Bold) },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                // Emoji picker
                Text("Biểu tượng", style = MaterialTheme.typography.labelMedium)
                LazyRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    items(EMOJI_OPTIONS) { e ->
                        FilterChip(
                            selected = emoji == e,
                            onClick = { emoji = e },
                            label = { Text(e, fontSize = 20.sp) },
                        )
                    }
                }

                // Name
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Tên agent") },
                    placeholder = { Text("VD: CSKH Zalo") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )

                // Role
                OutlinedTextField(
                    value = role,
                    onValueChange = { role = it },
                    label = { Text("Vai trò") },
                    placeholder = { Text("VD: Trả lời tin nhắn khách hàng") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )

                // System prompt
                OutlinedTextField(
                    value = systemPrompt,
                    onValueChange = { systemPrompt = it },
                    label = { Text("System Prompt") },
                    placeholder = { Text("Hướng dẫn cho AI: cách trả lời, phong cách, quy tắc...") },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(150.dp),
                    maxLines = 8,
                )

                // Knowledge base selection
                if (knowledgeBases.isNotEmpty()) {
                    Text("📚 Kiến thức (RAG)", style = MaterialTheme.typography.labelMedium)
                    knowledgeBases.forEach { kbId ->
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    selectedKBs = if (kbId in selectedKBs) selectedKBs - kbId
                                    else selectedKBs + kbId
                                },
                        ) {
                            Checkbox(
                                checked = kbId in selectedKBs,
                                onCheckedChange = {
                                    selectedKBs = if (it) selectedKBs + kbId else selectedKBs - kbId
                                },
                            )
                            Text("📚 $kbId", style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                }

                // Auto-reply toggle
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("🔔 Tự động trả lời", style = MaterialTheme.typography.labelMedium)
                        Text(
                            "Tự trả lời khi nhận tin nhắn mới",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Switch(checked = autoReply, onCheckedChange = { autoReply = it })
                }

                // Provider selector — show only enabled ones or currently selected
                Text("⚡ Nguồn AI", style = MaterialTheme.typography.labelMedium)

                val availableProviders = providers.filter { it.enabled || it.id == selectedProvider }

                availableProviders.forEach { provider ->
                    val typeTag = when {
                        provider.type == ProviderType.LOCAL_GGUF -> "🧠 Local"
                        provider.type == ProviderType.OLLAMA -> "🦙 Local"
                        else -> "🌐 API"
                    }
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { selectedProvider = provider.id },
                    ) {
                        RadioButton(
                            selected = selectedProvider == provider.id,
                            onClick = { selectedProvider = provider.id },
                        )
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                "${provider.emoji} ${provider.name}",
                                style = MaterialTheme.typography.bodyMedium,
                            )
                            Text(
                                typeTag,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        if (!provider.enabled && provider.apiKey.isBlank() && provider.type != ProviderType.OLLAMA) {
                            Text(
                                "chưa kn",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.error,
                            )
                        }
                    }
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    if (name.isNotBlank() && systemPrompt.isNotBlank()) {
                        val newAgent = LocalAgent(
                            id = agent?.id ?: "agent_${System.currentTimeMillis()}",
                            emoji = emoji,
                            name = name,
                            role = role,
                            systemPrompt = systemPrompt,
                            knowledgeBaseIds = selectedKBs.toList(),
                            autoReply = autoReply,
                            triggerApps = agent?.triggerApps ?: listOf("com.zing.zalo", "com.facebook.orca", "org.telegram.messenger"),
                            providerId = selectedProvider,
                            createdAt = agent?.createdAt ?: System.currentTimeMillis(),
                        )
                        onSave(newAgent)
                    }
                },
                enabled = name.isNotBlank() && systemPrompt.isNotBlank(),
            ) {
                Text("Lưu")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Huỷ") }
        },
    )
}


