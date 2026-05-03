package vn.bizclaw.app.ui.settings

import androidx.compose.animation.*
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vn.bizclaw.app.engine.AIProvider
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.engine.ProviderType
import java.net.HttpURLConnection
import java.net.URL

// ═══════════════════════════════════════════════════════════════
// Provider Config Screen
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProviderScreen(
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val manager = remember { ProviderManager(context) }
    var providers by remember { mutableStateOf(manager.loadProviders()) }
    var showAddDialog by remember { mutableStateOf(false) }
    var editingProvider by remember { mutableStateOf<AIProvider?>(null) }
    var deletingProvider by remember { mutableStateOf<AIProvider?>(null) }
    var testingProviderId by remember { mutableStateOf<String?>(null) }
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("⚡ Nguồn AI", fontWeight = FontWeight.Bold)
                        Text(
                            "${providers.count { it.enabled }} đang hoạt động",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Quay lại")
                    }
                },
                actions = {
                    IconButton(onClick = { showAddDialog = true }) {
                        Icon(Icons.Default.Add, "Thêm nguồn AI")
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        // No FAB — use + in TopAppBar
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            // Help text
            item {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f),
                    ),
                ) {
                    Column(modifier = Modifier.padding(12.dp)) {
                        Text(
                            "💡 Mỗi Agent có thể chọn 1 nguồn AI riêng",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.SemiBold,
                        )
                        Text(
                            "Danh sách các Nguồn AI API hoặc Local GGUF",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            // ═══ API Providers — configurable ═══
            item {
                Spacer(Modifier.height(8.dp))
                Text(
                    "🌐 API Providers",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
            }

            items(providers) { provider ->
                ProviderCard(
                    provider = provider,
                    onEdit = { editingProvider = provider },
                    onToggle = { enabled ->
                        manager.updateProvider(provider.copy(enabled = enabled))
                        providers = manager.loadProviders()
                    },
                    onDelete = { deletingProvider = provider },
                    onTest = {
                        testingProviderId = provider.id
                        scope.launch {
                            val result = testProvider(provider)
                            testingProviderId = null
                            snackbarHostState.showSnackbar(result)
                        }
                    },
                    isTesting = testingProviderId == provider.id,
                )
            }

            // Quick add shortcuts (API only)
            item {
                Spacer(Modifier.height(8.dp))
                Text(
                    "➕ Thêm nhanh",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                )
            }

            val quickAddTypes = listOf(
                Triple("🌐 OpenAI", ProviderType.OPENAI, "Cần API key từ platform.openai.com"),
                Triple("✨ Google Gemini", ProviderType.GEMINI, "Cần API key từ aistudio.google.com"),
                Triple("🦙 Ollama", ProviderType.OLLAMA, "Chạy Ollama trên máy tính cùng WiFi"),
                Triple("🔗 API Tương Thích", ProviderType.CUSTOM_API, "Bất kỳ API tương thích OpenAI"),
            )

            items(quickAddTypes) { (label, type, hint) ->
                val alreadyExists = providers.any { it.type == type && it.type != ProviderType.CUSTOM_API }
                OutlinedCard(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable(enabled = !alreadyExists) {
                            editingProvider = AIProvider(
                                id = "${type.name.lowercase()}_${System.currentTimeMillis()}",
                                name = label.substringAfter(" "),
                                type = type,
                                emoji = label.substringBefore(" "),
                            )
                        },
                    colors = CardDefaults.outlinedCardColors(
                        containerColor = if (alreadyExists)
                            MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
                        else
                            MaterialTheme.colorScheme.surface,
                    ),
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(label, modifier = Modifier.weight(1f))
                        if (alreadyExists) {
                            Text(
                                "✅ Đã thêm",
                                style = MaterialTheme.typography.labelSmall,
                                color = Color(0xFF00E676),
                            )
                        } else {
                            Text(
                                hint,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            item { Spacer(Modifier.height(24.dp)) }
        }
    }

    // Add/Edit Dialog
    if (showAddDialog || editingProvider != null) {
        ProviderFormDialog(
            provider = editingProvider,
            onDismiss = {
                showAddDialog = false
                editingProvider = null
            },
            onSave = { saved ->
                if (editingProvider != null && providers.any { it.id == saved.id }) {
                    manager.updateProvider(saved)
                } else {
                    manager.addProvider(saved)
                }
                providers = manager.loadProviders()
                showAddDialog = false
                editingProvider = null
                scope.launch {
                    snackbarHostState.showSnackbar("✅ ${saved.name} đã lưu")
                }
            },
        )
    }
}

// ═══════════════════════════════════════════════════════════════
// Provider Card
// ═══════════════════════════════════════════════════════════════

@Composable
private fun ProviderCard(
    provider: AIProvider,
    onEdit: () -> Unit,
    onToggle: (Boolean) -> Unit,
    onDelete: () -> Unit,
    onTest: () -> Unit,
    isTesting: Boolean = false,
) {
    val typeLabel = when {
        provider.type == ProviderType.LOCAL_GGUF -> "Cục bộ (GGUF)"
        provider.type == ProviderType.OPENAI -> "OpenAI API"
        provider.type == ProviderType.GEMINI -> "Google Gemini"
        provider.type == ProviderType.OLLAMA -> "Ollama"
        provider.type == ProviderType.BIZCLAW_CLOUD -> "BizClaw Cloud"
        provider.type == ProviderType.CUSTOM_API -> "API Tương Thích"
        else -> provider.type.name
    }

    val statusColor = when {
        !provider.enabled -> MaterialTheme.colorScheme.surfaceVariant
        provider.type == ProviderType.LOCAL_GGUF -> Color(0xFF1B5E20).copy(alpha = 0.15f)
        provider.apiKey.isNotBlank() || provider.type == ProviderType.OLLAMA -> Color(0xFF0D47A1).copy(alpha = 0.12f)
        else -> MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.3f)
    }

    Card(
        colors = CardDefaults.cardColors(containerColor = statusColor),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(provider.emoji, fontSize = 24.sp)
                Spacer(Modifier.width(8.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        provider.name,
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        typeLabel + if (provider.model.isNotBlank()) " • ${provider.model}" else "",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = provider.enabled,
                    onCheckedChange = onToggle,
                )
            }

            // Status badges
            Row(
                modifier = Modifier.padding(top = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                if (provider.type != ProviderType.LOCAL_GGUF) {
                    val hasKey = provider.apiKey.isNotBlank()
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = if (hasKey) Color(0xFF00E676).copy(alpha = 0.15f)
                        else MaterialTheme.colorScheme.error.copy(alpha = 0.15f),
                    ) {
                        Text(
                            if (hasKey) "🔑 API Key ✓" else "🔑 Chưa có key",
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                            style = MaterialTheme.typography.labelSmall,
                        )
                    }
                }
                if (provider.baseUrl.isNotBlank()) {
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.secondaryContainer.copy(alpha = 0.5f),
                    ) {
                        Text(
                            "🌐 ${provider.baseUrl.take(30)}",
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                            style = MaterialTheme.typography.labelSmall,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }

            // Action buttons
            Row(
                modifier = Modifier.padding(top = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (provider.type != ProviderType.LOCAL_GGUF) {
                    OutlinedButton(
                        onClick = onTest,
                        enabled = !isTesting,
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                    ) {
                        if (isTesting) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(14.dp),
                                strokeWidth = 2.dp,
                            )
                            Spacer(Modifier.width(4.dp))
                            Text("Testing...", style = MaterialTheme.typography.labelSmall)
                        } else {
                            Text("🔍 Test", style = MaterialTheme.typography.labelSmall)
                        }
                    }

                    OutlinedButton(
                        onClick = onEdit,
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                    ) { Text("✏️ Sửa", style = MaterialTheme.typography.labelSmall) }
                }

                if (provider.type != ProviderType.LOCAL_GGUF) {
                    OutlinedButton(
                        onClick = onDelete,
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                        colors = ButtonDefaults.outlinedButtonColors(
                            contentColor = MaterialTheme.colorScheme.error,
                        ),
                    ) { Text("🗑 Xoá", style = MaterialTheme.typography.labelSmall) }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Provider Form Dialog
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProviderFormDialog(
    provider: AIProvider?,
    onDismiss: () -> Unit,
    onSave: (AIProvider) -> Unit,
) {
    val isEdit = provider != null && provider.apiKey.isNotBlank()
    var name by remember { mutableStateOf(provider?.name ?: "") }
    var emoji by remember { mutableStateOf(provider?.emoji ?: "🤖") }
    var type by remember { mutableStateOf(provider?.type ?: ProviderType.OPENAI) }
    var apiKey by remember { mutableStateOf(provider?.apiKey ?: "") }
    var baseUrl by remember { mutableStateOf(provider?.baseUrl ?: "") }
    var model by remember { mutableStateOf(provider?.model ?: "") }
    var showApiKey by remember { mutableStateOf(false) }
    var typeExpanded by remember { mutableStateOf(false) }

    // Auto-fill defaults based on type
    LaunchedEffect(type) {
        if (!isEdit) {
            when (type) {
                ProviderType.OPENAI -> {
                    if (name.isBlank() || name == "Google Gemini" || name == "Ollama" || name == "API Tương Thích") name = "OpenAI"
                    if (baseUrl.isBlank() || baseUrl.contains("generativelanguage") || baseUrl.contains("11434")) baseUrl = "https://api.openai.com/v1"
                    if (model.isBlank() || model.contains("gemini") || model.contains("qwen")) model = "gpt-4o-mini"
                    emoji = "🌐"
                }
                ProviderType.GEMINI -> {
                    if (name.isBlank() || name == "OpenAI" || name == "Ollama") name = "Google Gemini"
                    if (baseUrl.isBlank() || baseUrl.contains("openai") || baseUrl.contains("11434")) baseUrl = "https://generativelanguage.googleapis.com"
                    if (model.isBlank() || model.contains("gpt") || model.contains("qwen")) model = "gemini-2.0-flash"
                    emoji = "✨"
                }
                ProviderType.OLLAMA -> {
                    if (name.isBlank() || name == "OpenAI" || name == "Google Gemini") name = "Ollama (Máy tính)"
                    if (baseUrl.isBlank() || baseUrl.contains("openai") || baseUrl.contains("generativelanguage")) baseUrl = "http://192.168.1.100:11434"
                    if (model.isBlank() || model.contains("gpt") || model.contains("gemini")) model = "qwen2.5:7b"
                    emoji = "🦙"
                }
                ProviderType.CUSTOM_API -> {
                    if (name.isBlank()) name = "API Tương Thích"
                    emoji = "🔗"
                }
                else -> {}
            }
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                if (isEdit) "✏️ Sửa ${provider?.name}" else "➕ Thêm Nguồn AI",
                fontWeight = FontWeight.Bold,
            )
        },
        text = {
            Column(
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                // Provider type selector
                Text("Loại nguồn AI:", style = MaterialTheme.typography.labelMedium)
                ExposedDropdownMenuBox(
                    expanded = typeExpanded,
                    onExpandedChange = { typeExpanded = it },
                ) {
                    OutlinedTextField(
                        value = when (type) {
                            ProviderType.OPENAI -> "🌐 OpenAI"
                            ProviderType.GEMINI -> "✨ Google Gemini"
                            ProviderType.OLLAMA -> "🦙 Ollama"
                            ProviderType.CUSTOM_API -> "🔗 API Tương Thích"
                            ProviderType.BIZCLAW_CLOUD -> "☁️ BizClaw Cloud"
                            ProviderType.LOCAL_GGUF -> "🧠 Cục Bộ (GGUF)"
                            else -> "📱 App (quản lý ở mục bật/tắt)"
                        },
                        onValueChange = {},
                        readOnly = true,
                        trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = typeExpanded) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .menuAnchor(MenuAnchorType.PrimaryNotEditable, true),
                        singleLine = true,
                    )
                    ExposedDropdownMenu(
                        expanded = typeExpanded,
                        onDismissRequest = { typeExpanded = false },
                    ) {
                        listOf(
                            ProviderType.OPENAI to "🌐 OpenAI",
                            ProviderType.GEMINI to "✨ Google Gemini",
                            ProviderType.OLLAMA to "🦙 Ollama (Máy tính)",
                            ProviderType.CUSTOM_API to "🔗 API Tương Thích",
                        ).forEach { (t, label) ->
                            DropdownMenuItem(
                                text = { Text(label) },
                                onClick = {
                                    type = t
                                    typeExpanded = false
                                },
                            )
                        }
                    }
                }

                // Name
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Tên hiển thị") },
                    placeholder = { Text("VD: OpenAI GPT-4") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )

                // API Key (not for Ollama)
                if (type != ProviderType.OLLAMA) {
                    OutlinedTextField(
                        value = apiKey,
                        onValueChange = { apiKey = it },
                        label = { Text("API Key") },
                        placeholder = {
                            Text(
                                when (type) {
                                    ProviderType.OPENAI -> "sk-..."
                                    ProviderType.GEMINI -> "AIza..."
                                    else -> "API key"
                                }
                            )
                        },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        visualTransformation = if (showApiKey) VisualTransformation.None
                        else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showApiKey = !showApiKey }) {
                                Icon(
                                    if (showApiKey) Icons.Default.VisibilityOff
                                    else Icons.Default.Visibility,
                                    if (showApiKey) "Ẩn" else "Hiện",
                                )
                            }
                        },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    )
                }

                // Base URL
                OutlinedTextField(
                    value = baseUrl,
                    onValueChange = { baseUrl = it },
                    label = { Text("URL") },
                    placeholder = {
                        Text(
                            when (type) {
                                ProviderType.OPENAI -> "https://api.openai.com/v1"
                                ProviderType.GEMINI -> "https://generativelanguage.googleapis.com"
                                ProviderType.OLLAMA -> "http://192.168.1.x:11434"
                                else -> "https://api.example.com/v1"
                            }
                        )
                    },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                )

                // Model name
                OutlinedTextField(
                    value = model,
                    onValueChange = { model = it },
                    label = { Text("Tên model") },
                    placeholder = {
                        Text(
                            when (type) {
                                ProviderType.OPENAI -> "gpt-4o-mini, gpt-4o, gpt-3.5-turbo"
                                ProviderType.GEMINI -> "gemini-2.0-flash, gemini-1.5-pro"
                                ProviderType.OLLAMA -> "qwen2.5:7b, llama3.1:8b"
                                else -> "model-name"
                            }
                        )
                    },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )

                // Hint
                Surface(
                    color = MaterialTheme.colorScheme.secondaryContainer.copy(alpha = 0.3f),
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text(
                        when (type) {
                            ProviderType.OPENAI -> "💡 Lấy API key tại: platform.openai.com/api-keys"
                            ProviderType.GEMINI -> "💡 Lấy API key tại: aistudio.google.com/apikey"
                            ProviderType.OLLAMA -> "💡 Chạy 'ollama serve' trên máy tính. Đổi IP thành IP máy tính trong cùng mạng WiFi."
                            ProviderType.CUSTOM_API -> "💡 Nhập URL bất kỳ API tương thích OpenAI (vd: LiteLLM, vLLM, Together AI)"
                            else -> ""
                        },
                        modifier = Modifier.padding(8.dp),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    onSave(
                        AIProvider(
                            id = provider?.id ?: "${type.name.lowercase()}_${System.currentTimeMillis()}",
                            name = name.ifBlank { type.name },
                            type = type,
                            emoji = emoji,
                            apiKey = apiKey,
                            baseUrl = baseUrl,
                            model = model,
                            enabled = true,
                        )
                    )
                },
                enabled = name.isNotBlank(),
            ) { Text("Lưu") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Huỷ") }
        },
    )
}

// ═══════════════════════════════════════════════════════════════
// Test Provider Connection
// ═══════════════════════════════════════════════════════════════

private suspend fun testProvider(provider: AIProvider): String {
    return withContext(Dispatchers.IO) {
        try {
            when (provider.type) {
                ProviderType.OLLAMA -> {
                    val url = URL("${provider.baseUrl}/api/tags")
                    val conn = url.openConnection() as HttpURLConnection
                    conn.connectTimeout = 5000
                    conn.readTimeout = 5000
                    val code = conn.responseCode
                    conn.disconnect()
                    if (code == 200) "✅ Ollama kết nối thành công!"
                    else "❌ Ollama trả về mã $code"
                }
                ProviderType.OPENAI -> {
                    val url = URL("${provider.baseUrl}/models")
                    val conn = url.openConnection() as HttpURLConnection
                    conn.setRequestProperty("Authorization", "Bearer ${provider.apiKey}")
                    conn.connectTimeout = 5000
                    conn.readTimeout = 5000
                    val code = conn.responseCode
                    conn.disconnect()
                    when (code) {
                        200 -> "✅ OpenAI kết nối thành công!"
                        401 -> "❌ API key không hợp lệ"
                        429 -> "⚠️ Đã vượt rate limit"
                        else -> "❌ OpenAI trả về mã $code"
                    }
                }
                ProviderType.GEMINI -> {
                    val url = URL("${provider.baseUrl}/v1beta/models?key=${provider.apiKey}")
                    val conn = url.openConnection() as HttpURLConnection
                    conn.connectTimeout = 5000
                    conn.readTimeout = 5000
                    val code = conn.responseCode
                    conn.disconnect()
                    when (code) {
                        200 -> "✅ Gemini kết nối thành công!"
                        400, 403 -> "❌ API key không hợp lệ"
                        else -> "❌ Gemini trả về mã $code"
                    }
                }
                else -> "ℹ️ Không thể test loại provider này"
            }
        } catch (e: Exception) {
            "❌ Lỗi: ${e.message?.take(50)}"
        }
    }
}
