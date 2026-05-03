package vn.bizclaw.app.ui.chat

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.launch
import vn.bizclaw.app.engine.*

// ═══════════════════════════════════════════════════════════
// GROUP CHAT SENDER — extracted business logic
// ═══════════════════════════════════════════════════════════

/**
 * Send group chat message — handles parallel remote + sequential local agents
 */
suspend fun sendGroupMessage(
    msg: String,
    group: AgentGroup,
    agents: List<LocalAgent>,
    providers: List<AIProvider>,
    agentManager: LocalAgentManager,
    viewModel: ChatViewModel,
) {
    val groupAgents = group.agentIds.mapNotNull { aid ->
        agents.find { it.id == aid }
    }

    // Separate local vs remote
    val localAgentList = mutableListOf<LocalAgent>()
    val remoteAgentList = mutableListOf<LocalAgent>()

    groupAgents.forEach { agent ->
        val provider = providers.find { it.id == agent.providerId }
        if (provider == null || provider.type == ProviderType.LOCAL_GGUF) {
            localAgentList.add(agent)
        } else {
            remoteAgentList.add(agent)
        }
    }

    // Remote agents: parallel
    val remoteResults = remoteAgentList.map { agent ->
        kotlinx.coroutines.coroutineScope {
            async {
                val provider = providers.find { it.id == agent.providerId }
                    ?: return@async Triple(agent, "⚠️ Không tìm thấy nguồn AI", "Unknown")
                val prompt = agentManager.buildPromptForAgent(agent, msg)
                val response = try {
                    ProviderChat.chat(provider, prompt, msg)
                } catch (e: Exception) {
                    "❌ Lỗi: ${e.message?.take(60)}"
                }
                Triple(agent, response, provider.name)
            }
        }
    }.map { it.await() }

    // Add remote results immediately
    remoteResults.forEach { (agent, response, providerName) ->
        viewModel.addGroupResponse(
            agentEmoji = agent.emoji,
            agentName = agent.name,
            providerName = providerName,
            content = response,
        )
    }

    // Local agents: sequential
    localAgentList.forEach { agent ->
        val provider = providers.find { it.id == agent.providerId }
            ?: providers.find { it.id == "local_gguf" }
        val prompt = agentManager.buildPromptForAgent(agent, msg)
        val response = try {
            ProviderChat.chat(
                provider ?: AIProvider(
                    id = "local_gguf",
                    name = "AI Cục Bộ",
                    type = ProviderType.LOCAL_GGUF,
                    emoji = "🧠",
                ),
                prompt,
                msg,
            )
        } catch (e: Exception) {
            "❌ Lỗi cục bộ: ${e.message?.take(60)}"
        }
        viewModel.addGroupResponse(
            agentEmoji = agent.emoji,
            agentName = agent.name,
            providerName = provider?.name ?: "AI Cục Bộ",
            content = response,
        )
    }
}

// ═══════════════════════════════════════════════════════════
// GROUP CREATION DIALOG
// ═══════════════════════════════════════════════════════════

@Composable
fun GroupEditDialog(
    agents: List<LocalAgent>,
    editingGroup: AgentGroup? = null,
    onDismiss: () -> Unit,
    onSave: (AgentGroup) -> Unit,
    onDelete: ((AgentGroup) -> Unit)? = null,
) {
    var groupName by remember { mutableStateOf(editingGroup?.name ?: "") }
    var groupEmoji by remember { mutableStateOf(editingGroup?.emoji ?: "👥") }
    var selectedMembers by remember { mutableStateOf<Set<String>>(editingGroup?.agentIds?.toSet() ?: emptySet()) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (editingGroup == null) "Tạo nhóm Agent" else "Sửa nhóm Agent", fontWeight = FontWeight.Bold) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = groupName,
                    onValueChange = { groupName = it },
                    label = { Text("Tên nhóm") },
                    placeholder = { Text("VD: Nhóm CSKH") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Text("Chọn agent vào nhóm:", style = MaterialTheme.typography.labelMedium)
                agents.forEach { agent ->
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Checkbox(
                            checked = agent.id in selectedMembers,
                            onCheckedChange = {
                                selectedMembers = if (it)
                                    selectedMembers + agent.id
                                else
                                    selectedMembers - agent.id
                            },
                        )
                        Text("${agent.emoji} ${agent.name}")
                    }
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    if (groupName.isNotBlank() && selectedMembers.isNotEmpty()) {
                        onSave(
                            AgentGroup(
                                id = editingGroup?.id ?: "group_${System.currentTimeMillis()}",
                                name = groupName,
                                emoji = groupEmoji,
                                agentIds = selectedMembers.toList(),
                            )
                        )
                    }
                },
                enabled = groupName.isNotBlank() && selectedMembers.isNotEmpty(),
            ) { Text(if (editingGroup == null) "Tạo nhóm" else "Lưu nhóm") }
        },
        dismissButton = {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (editingGroup != null && onDelete != null) {
                    TextButton(
                        onClick = { onDelete(editingGroup) },
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error)
                    ) { Text("Xoá Nhóm") }
                }
                TextButton(onClick = onDismiss) { Text("Huỷ") }
            }
        },
    )
}

// ═══════════════════════════════════════════════════════════
// DELETE CONFIRMATION DIALOG
// ═══════════════════════════════════════════════════════════

@Composable
fun DeleteConfirmDialog(
    itemName: String,
    onDismiss: () -> Unit,
    onConfirm: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("⚠️ Xác nhận xoá", fontWeight = FontWeight.Bold) },
        text = {
            Text("Bạn có chắc muốn xoá \"$itemName\"? Hành động này không thể hoàn tác.")
        },
        confirmButton = {
            Button(
                onClick = onConfirm,
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error,
                ),
            ) { Text("🗑 Xoá") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Huỷ") }
        },
    )
}
