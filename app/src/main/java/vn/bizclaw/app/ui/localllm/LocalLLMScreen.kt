package vn.bizclaw.app.ui.localllm

import android.app.Application
import android.content.Context
import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.launch
import vn.bizclaw.app.engine.BizClawLLM
import vn.bizclaw.app.engine.DownloadableModel
import vn.bizclaw.app.engine.GlobalLLM
import vn.bizclaw.app.engine.ModelDownloadManager
import vn.bizclaw.app.engine.RECOMMENDED_MODELS
import java.io.File

// ═══════════════════════════════════════════════════════════════
// Tab identifiers
// ═══════════════════════════════════════════════════════════════
private enum class LLMTab { Models, Benchmark }

// ═══════════════════════════════════════════════════════════════
// Demo Agent Personas
// ═══════════════════════════════════════════════════════════════
data class AgentPersona(
    val emoji: String,
    val name: String,
    val role: String,
    val systemPrompt: String,
)

val DEMO_AGENTS = listOf(
    AgentPersona(
        emoji = "🤖",
        name = "BizClaw",
        role = "Trợ lý tổng hợp",
        systemPrompt = "Bạn tên là BizClaw, trợ lý AI thân thiện chạy trên điện thoại. " +
            "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
            "Trả lời ngắn gọn, tự nhiên. Khi được chào, chào lại thân thiện. " +
            "Không bịa thông tin.",
    ),
    AgentPersona(
        emoji = "📝",
        name = "Copywriter",
        role = "Viết nội dung & quảng cáo",
        systemPrompt = "Bạn là chuyên gia viết nội dung marketing. " +
            "CHỈ viết bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
            "Viết caption Facebook, mô tả sản phẩm, email marketing, bài blog. " +
            "Phong cách sáng tạo, thu hút, phù hợp thị trường Việt Nam.",
    ),
    AgentPersona(
        emoji = "📊",
        name = "Phân tích",
        role = "Phân tích dữ liệu & báo cáo",
        systemPrompt = "Bạn là chuyên gia phân tích kinh doanh. " +
            "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
            "Giúp phân tích số liệu, đưa ra nhận xét và đề xuất. " +
            "Trả lời có cấu trúc, dùng gạch đầu dòng.",
    ),
    AgentPersona(
        emoji = "🎯",
        name = "Chiến lược",
        role = "Tư vấn chiến lược kinh doanh",
        systemPrompt = "Bạn là cố vấn chiến lược kinh doanh cho doanh nghiệp Việt Nam. " +
            "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
            "Tư vấn marketing, bán hàng, vận hành, nhân sự. " +
            "Đưa lời khuyên thực tế, áp dụng được ngay.",
    ),
    AgentPersona(
        emoji = "💬",
        name = "CSKH",
        role = "Chăm sóc khách hàng",
        systemPrompt = "Bạn là nhân viên chăm sóc khách hàng chuyên nghiệp. " +
            "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
            "Trả lời lịch sự, kiên nhẫn, giải quyết vấn đề nhanh chóng. " +
            "Xin lỗi khi cần, cảm ơn khách hàng, đề xuất giải pháp.",
    ),
)

// ═══════════════════════════════════════════════════════════════
// Main Screen
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LocalLLMScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    // State
    var selectedTab by remember { mutableStateOf(LLMTab.Models) }
    val llm = GlobalLLM.instance
    val downloadManager = remember { ModelDownloadManager(context) }

    // Model state — restore from GlobalLLM if model was already loaded
    var loadedModelName by remember { mutableStateOf(GlobalLLM.loadedModelName) }
    var isLoading by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf(
        if (GlobalLLM.loadedModelName != null) "✅ ${GlobalLLM.loadedModelName} đã sẵn sàng!" else ""
    ) }
    var downloadProgress by remember { mutableStateOf<Map<String, Float>>(emptyMap()) }

    // Chat state
    val chatMessages = remember { mutableStateListOf<ChatMsg>() }
    var chatInput by remember { mutableStateOf("") }
    var isGenerating by remember { mutableStateOf(false) }
    var generationSpeed by remember { mutableStateOf(0f) }
    var contextUsed by remember { mutableStateOf(0) }
    var selectedAgent by remember { mutableStateOf(DEMO_AGENTS[0]) }

    // Benchmark state
    var benchResult by remember { mutableStateOf("") }
    var isBenching by remember { mutableStateOf(false) }

    // Get local model files
    val downloadedModels by downloadManager.downloadedModels.collectAsState(initial = emptyList())
    val localModels = remember(downloadedModels) {
        downloadedModels.map { it.name to it.path }
    }

    // Auto-stay on Models tab
    LaunchedEffect(Unit) {
        if (llm.isLoaded && loadedModelName != null) {
            selectedTab = LLMTab.Models
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("🧠 AI Cục Bộ", fontWeight = FontWeight.Bold)
                        if (loadedModelName != null) {
                            Text(
                                "✅ $loadedModelName",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.primary,
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Quay lại")
                    }
                },
                actions = {
                    if (llm.isLoaded) {
                        IconButton(onClick = {
                            llm.close()
                            loadedModelName = null
                            GlobalLLM.setModelName(null)
                            statusMessage = "Đã gỡ mô hình"
                        }) {
                            Icon(Icons.Default.PowerSettingsNew, "Gỡ bỏ")
                        }
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
                .padding(padding),
        ) {
            // Tab bar
            TabRow(selectedTabIndex = selectedTab.ordinal) {
                LLMTab.entries.forEach { tab ->
                    Tab(
                        selected = selectedTab == tab,
                        onClick = { selectedTab = tab },
                        text = {
                            Text(
                                when (tab) {
                                    LLMTab.Models -> "📦 Mô hình"
                                    LLMTab.Benchmark -> "⚡ Hiệu năng"
                                }
                            )
                        },
                    )
                }
            }

            // Status bar
            AnimatedVisibility(visible = statusMessage.isNotBlank()) {
                Surface(
                    color = MaterialTheme.colorScheme.secondaryContainer,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        text = statusMessage,
                        modifier = Modifier.padding(12.dp),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }

            // Tab content
            when (selectedTab) {
                LLMTab.Models -> ModelsTab(
                    downloadManager = downloadManager,
                    localModels = localModels,
                    downloadProgress = downloadProgress,
                    loadedModelName = loadedModelName,
                    isLoading = isLoading,
                    onDownload = { model ->
                        scope.launch {
                            statusMessage = "Đang tải ${model.name}..."
                            downloadManager.downloadModel(model)
                        }
                    },
                    onLoad = { name, path ->
                        scope.launch {
                            isLoading = true
                            statusMessage = "Đang nạp $name..."
                            try {
                                llm.close() // Unload any previous model
                                llm.load(
                                    modelPath = path,
                                    params = BizClawLLM.InferenceParams(
                                        numThreads = Runtime.getRuntime().availableProcessors()
                                            .coerceAtMost(8),
                                    ),
                                )
                                // Set agent system prompt
                                llm.addSystemPrompt(selectedAgent.systemPrompt)
                                loadedModelName = name
                                GlobalLLM.setModelName(name)
                                statusMessage = "✅ $name đã sẵn sàng! Quay về Home để chat."
                                chatMessages.clear()
                            } catch (e: Exception) {
                                statusMessage = "❌ Nạp thất bại: ${e.message}"
                            }
                            isLoading = false
                        }
                    },
                    onDelete = { name, path ->
                        File(path).delete()
                        statusMessage = "Đã xóa $name"
                    },
                )

                LLMTab.Benchmark -> BenchmarkTab(
                    llm = llm,
                    result = benchResult,
                    isBenching = isBenching,
                    loadedModelName = loadedModelName,
                    onRunBench = {
                        if (llm.isLoaded) {
                            isBenching = true
                            scope.launch(Dispatchers.IO) {
                                benchResult = llm.benchmark()
                                isBenching = false
                            }
                        }
                    },
                )
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Chat message data
// ═══════════════════════════════════════════════════════════════
data class ChatMsg(
    val role: String,
    val content: String,
    val thinking: String = "",
    val isThinking: Boolean = false,
)

/** Parse <think>...</think> tags from model output */
private data class ThinkParsed(
    val thinking: String,
    val response: String,
    val isStillThinking: Boolean,
)

private fun parseThinking(raw: String): ThinkParsed {
    val thinkStart = raw.indexOf("<think>")
    val thinkEnd = raw.indexOf("</think>")

    return when {
        // No think tags at all
        thinkStart < 0 -> ThinkParsed("", raw.trim(), false)
        // Still inside <think>...</think> — hasn't closed yet
        thinkEnd < 0 -> {
            val thinking = raw.substring(thinkStart + 7).trim()
            ThinkParsed(thinking, "", true)
        }
        // Think tag is closed — split thinking and response
        else -> {
            val thinking = raw.substring(thinkStart + 7, thinkEnd).trim()
            val response = raw.substring(thinkEnd + 8).trim()
            ThinkParsed(thinking, response, false)
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Models Tab
// ═══════════════════════════════════════════════════════════════
@Composable
private fun ModelsTab(
    downloadManager: ModelDownloadManager,
    localModels: List<Pair<String, String>>, // name, path
    downloadProgress: Map<String, Float>,
    loadedModelName: String?,
    isLoading: Boolean,
    onDownload: (DownloadableModel) -> Unit,
    onLoad: (String, String) -> Unit,
    onDelete: (String, String) -> Unit,
) {
    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        // Local models section
        if (localModels.isNotEmpty()) {
            item {
                Text(
                    "📱 Mô hình đã tải",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )
            }
            items(localModels) { (name, path) ->
                LocalModelCard(
                    name = name,
                    path = path,
                    isLoaded = name == loadedModelName,
                    isLoading = isLoading,
                    onLoad = { onLoad(name, path) },
                    onDelete = { onDelete(name, path) },
                )
            }
        }

        // Available models section
        item {
            Spacer(Modifier.height(8.dp))
            Text(
                "🌐 Mô hình có sẵn (HuggingFace)",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
        }
        items(RECOMMENDED_MODELS) { model ->
            val isDownloaded = localModels.any { it.first.contains(model.paramCount) }
            val progress = downloadProgress[model.name]
            DownloadableModelCard(
                model = model,
                isDownloaded = isDownloaded,
                downloadProgress = progress,
                onDownload = { onDownload(model) },
            )
        }
    }
}

@Composable
private fun LocalModelCard(
    name: String,
    path: String,
    isLoaded: Boolean,
    isLoading: Boolean,
    onLoad: () -> Unit,
    onDelete: () -> Unit,
) {
    val file = File(path)
    val sizeMB = file.length() / 1_000_000

    Card(
        colors = CardDefaults.cardColors(
            containerColor = if (isLoaded) MaterialTheme.colorScheme.primaryContainer
            else MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            // Icon
            Box(
                modifier = Modifier
                    .size(48.dp)
                    .clip(CircleShape)
                    .background(
                        if (isLoaded) MaterialTheme.colorScheme.primary
                        else MaterialTheme.colorScheme.outline.copy(alpha = 0.2f)
                    ),
                contentAlignment = Alignment.Center,
            ) {
                Text(if (isLoaded) "✅" else "🧠", fontSize = 20.sp)
            }

            Spacer(Modifier.width(12.dp))

            // Info
            Column(modifier = Modifier.weight(1f)) {
                Text(name, fontWeight = FontWeight.Bold)
                Text(
                    "${sizeMB} MB",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            // Actions
            if (!isLoaded) {
                IconButton(onClick = onLoad, enabled = !isLoading) {
                    if (isLoading) {
                        CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                    } else {
                        Icon(Icons.Default.PlayArrow, "Nạp", tint = MaterialTheme.colorScheme.primary)
                    }
                }
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Default.Delete, "Xóa", tint = MaterialTheme.colorScheme.error)
            }
        }
    }
}

@Composable
private fun DownloadableModelCard(
    model: DownloadableModel,
    isDownloaded: Boolean,
    downloadProgress: Float?,
    onDownload: () -> Unit,
) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(model.name, fontWeight = FontWeight.Bold)
                    Text(
                        model.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Spacer(Modifier.width(8.dp))
                Column(horizontalAlignment = Alignment.End) {
                    Text(
                        model.sizeDisplay,
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        model.quantization,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }

            Spacer(Modifier.height(8.dp))

            if (downloadProgress != null) {
                LinearProgressIndicator(
                    progress = { downloadProgress },
                    modifier = Modifier.fillMaxWidth(),
                )
                Text(
                    "${(downloadProgress * 100).toInt()}%",
                    style = MaterialTheme.typography.labelSmall,
                )
            } else if (isDownloaded) {
                Text(
                    "✅ Đã tải",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary,
                )
            } else {
                Button(
                    onClick = onDownload,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Icon(Icons.Default.Download, null)
                    Spacer(Modifier.width(8.dp))
                    Text("Tải xuống ${model.sizeDisplay}")
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Chat Tab
// ═══════════════════════════════════════════════════════════════
@Composable
private fun ChatTab(
    llm: BizClawLLM,
    messages: List<ChatMsg>,
    input: String,
    isGenerating: Boolean,
    generationSpeed: Float,
    contextUsed: Int,
    loadedModelName: String?,
    selectedAgent: AgentPersona,
    onAgentChange: (AgentPersona) -> Unit,
    onInputChange: (String) -> Unit,
    onSend: (String) -> Unit,
) {
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()

    // Auto-scroll
    LaunchedEffect(messages.size, messages.lastOrNull()?.content) {
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(messages.size - 1)
        }
    }

    Column(modifier = Modifier.fillMaxSize()) {
        if (loadedModelName == null) {
            // No model loaded
            Box(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("🧠", fontSize = 64.sp)
                    Spacer(Modifier.height(16.dp))
                    Text("Hãy nạp mô hình trước", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Vào tab Mô hình → bấm ▶️ trên mô hình đã tải",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        } else {
            // Agent selector chips
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier
                        .padding(horizontal = 12.dp, vertical = 6.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    DEMO_AGENTS.forEach { agent ->
                        val isSelected = agent.name == selectedAgent.name
                        FilterChip(
                            selected = isSelected,
                            onClick = { onAgentChange(agent) },
                            label = {
                                Text(
                                    "${agent.emoji} ${agent.name}",
                                    style = MaterialTheme.typography.labelSmall,
                                )
                            },
                            modifier = Modifier.height(32.dp),
                        )
                    }
                }
            }

            // Performance stats
            if (generationSpeed > 0) {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        modifier = Modifier.padding(8.dp, 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        Text(
                            "⚡ %.1f tok/s".format(generationSpeed),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary,
                        )
                        Text(
                            "📊 Ngữ cảnh: $contextUsed",
                            style = MaterialTheme.typography.labelSmall,
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
                contentPadding = PaddingValues(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (messages.isEmpty()) {
                    item {
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 24.dp),
                            colors = CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(
                                    alpha = 0.5f
                                ),
                            ),
                        ) {
                            Column(
                                modifier = Modifier
                                    .padding(24.dp)
                                    .fillMaxWidth(),
                                horizontalAlignment = Alignment.CenterHorizontally,
                            ) {
                                Text(selectedAgent.emoji, fontSize = 48.sp)
                                Spacer(Modifier.height(8.dp))
                                Text(
                                    "${selectedAgent.name} — $loadedModelName",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold,
                                )
                                Text(
                                    selectedAgent.role,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.primary,
                                )
                                Spacer(Modifier.height(4.dp))
                                Text(
                                    "100% ngoại tuyến • Miễn phí",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }

                items(messages) { msg ->
                    LLMChatBubble(msg = msg, isGenerating = isGenerating && msg == messages.last())
                }
            }

            // Input bar
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier.padding(12.dp, 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedTextField(
                        value = input,
                        onValueChange = onInputChange,
                        modifier = Modifier.weight(1f),
                        placeholder = { Text("Hỏi BizClaw cục bộ...") },
                        shape = RoundedCornerShape(24.dp),
                        maxLines = 4,
                    )
                    Spacer(Modifier.width(8.dp))
                    FilledIconButton(
                        onClick = { onSend(input) },
                        enabled = input.isNotBlank() && !isGenerating && llm.isLoaded,
                        modifier = Modifier.size(48.dp),
                    ) {
                        if (isGenerating) {
                            CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                        } else {
                            Icon(Icons.Default.Send, "Gửi")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LLMChatBubble(msg: ChatMsg, isGenerating: Boolean) {
    val isUser = msg.role == "user"
    var showThinking by remember { mutableStateOf(false) }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
    ) {
        if (!isUser) {
            Box(
                modifier = Modifier
                    .size(32.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primary),
                contentAlignment = Alignment.Center,
            ) {
                Text("🧠", fontSize = 16.sp)
            }
            Spacer(Modifier.width(8.dp))
        }

        Surface(
            shape = RoundedCornerShape(
                topStart = 16.dp, topEnd = 16.dp,
                bottomStart = if (isUser) 16.dp else 4.dp,
                bottomEnd = if (isUser) 4.dp else 16.dp,
            ),
            color = if (isUser) MaterialTheme.colorScheme.primary
            else MaterialTheme.colorScheme.surfaceVariant,
            modifier = Modifier.widthIn(max = 300.dp),
        ) {
            Column(modifier = Modifier.padding(12.dp)) {
                if (!isUser) {
                    Text(
                        "BizClaw (Cục bộ)",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Spacer(Modifier.height(2.dp))
                }

                // Thinking indicator — while model is still in <think> mode
                if (msg.isThinking) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("🤔", fontSize = 16.sp)
                        Spacer(Modifier.width(8.dp))
                        Text(
                            "Đang suy nghĩ...",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                            fontStyle = androidx.compose.ui.text.font.FontStyle.Italic,
                        )
                    }
                    Spacer(Modifier.height(4.dp))
                    LinearProgressIndicator(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(2.dp),
                        color = MaterialTheme.colorScheme.tertiary,
                    )
                }

                // Collapsible thinking block — after thinking is done
                if (!msg.isThinking && msg.thinking.isNotBlank()) {
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.tertiary.copy(alpha = 0.1f),
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { showThinking = !showThinking },
                    ) {
                        Column(modifier = Modifier.padding(8.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Text("💭", fontSize = 14.sp)
                                Spacer(Modifier.width(6.dp))
                                Text(
                                    if (showThinking) "Ẩn suy nghĩ" else "Xem suy nghĩ",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.tertiary,
                                    fontWeight = FontWeight.Medium,
                                )
                                Spacer(Modifier.weight(1f))
                                Text(
                                    if (showThinking) "▲" else "▼",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.tertiary,
                                )
                            }
                            AnimatedVisibility(visible = showThinking) {
                                Column {
                                    Spacer(Modifier.height(6.dp))
                                    Text(
                                        msg.thinking,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                                        fontStyle = androidx.compose.ui.text.font.FontStyle.Italic,
                                        lineHeight = 16.sp,
                                    )
                                }
                            }
                        }
                    }
                    Spacer(Modifier.height(6.dp))
                }

                // Actual response content
                if (msg.content.isNotEmpty()) {
                    Text(
                        text = msg.content,
                        color = if (isUser) MaterialTheme.colorScheme.onPrimary
                        else MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                } else if (!msg.isThinking) {
                    Text(
                        text = "...",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }

                if (isGenerating && !isUser && !msg.isThinking) {
                    Spacer(Modifier.height(4.dp))
                    LinearProgressIndicator(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(2.dp),
                        color = MaterialTheme.colorScheme.secondary,
                    )
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Benchmark Tab
// ═══════════════════════════════════════════════════════════════
@Composable
private fun BenchmarkTab(
    llm: BizClawLLM,
    result: String,
    isBenching: Boolean,
    loadedModelName: String?,
    onRunBench: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        if (loadedModelName == null) {
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center,
            ) {
                Text("Hãy nạp mô hình để chạy đo hiệu năng")
            }
        } else {
            Text(
                "⚡ Đo Hiệu Năng",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "Mô hình: $loadedModelName",
                style = MaterialTheme.typography.bodySmall,
            )

            Button(
                onClick = onRunBench,
                enabled = !isBenching,
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (isBenching) {
                    CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                    Spacer(Modifier.width(8.dp))
                    Text("Đang đo hiệu năng...")
                } else {
                    Icon(Icons.Default.Speed, null)
                    Spacer(Modifier.width(8.dp))
                    Text("Chạy Đo Hiệu Năng (pp=512, tg=128)")
                }
            }

            if (result.isNotBlank()) {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant,
                    ),
                ) {
                    Text(
                        text = result,
                        modifier = Modifier.padding(16.dp),
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                    )
                }
            }
        }
    }
}
