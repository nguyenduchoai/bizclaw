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
import vn.bizclaw.app.engine.ModelDownloadManager
import vn.bizclaw.app.engine.RECOMMENDED_MODELS
import java.io.File

// ═══════════════════════════════════════════════════════════════
// Tab identifiers
// ═══════════════════════════════════════════════════════════════
private enum class LLMTab { Models, Chat, Benchmark }

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
    val llm = remember { BizClawLLM() }
    val downloadManager = remember { ModelDownloadManager(context) }

    // Model state
    var loadedModelName by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf("") }
    var downloadProgress by remember { mutableStateOf<Map<String, Float>>(emptyMap()) }

    // Chat state
    val chatMessages = remember { mutableStateListOf<ChatMsg>() }
    var chatInput by remember { mutableStateOf("") }
    var isGenerating by remember { mutableStateOf(false) }
    var generationSpeed by remember { mutableStateOf(0f) }
    var contextUsed by remember { mutableStateOf(0) }

    // Benchmark state
    var benchResult by remember { mutableStateOf("") }
    var isBenching by remember { mutableStateOf(false) }

    // Get local model files
    val downloadedModels by downloadManager.downloadedModels.collectAsState(initial = emptyList())
    val localModels = remember(downloadedModels) {
        downloadedModels.map { it.name to it.path }
    }

    // Cleanup
    DisposableEffect(Unit) {
        onDispose { llm.close() }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("🧠 Local LLM", fontWeight = FontWeight.Bold)
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
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                    }
                },
                actions = {
                    if (llm.isLoaded) {
                        IconButton(onClick = {
                            llm.close()
                            loadedModelName = null
                            statusMessage = "Model unloaded"
                        }) {
                            Icon(Icons.Default.PowerSettingsNew, "Unload")
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
                                    LLMTab.Models -> "📦 Models"
                                    LLMTab.Chat -> "💬 Chat"
                                    LLMTab.Benchmark -> "⚡ Bench"
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
                            statusMessage = "Downloading ${model.name}..."
                            downloadManager.downloadModel(model)
                        }
                    },
                    onLoad = { name, path ->
                        scope.launch {
                            isLoading = true
                            statusMessage = "Loading $name..."
                            try {
                                llm.close() // Unload any previous model
                                llm.load(
                                    modelPath = path,
                                    params = BizClawLLM.InferenceParams(
                                        numThreads = Runtime.getRuntime().availableProcessors()
                                            .coerceAtMost(8),
                                    ),
                                )
                                // Set BizClaw system prompt
                                llm.addSystemPrompt(
                                    "You are BizClaw, a helpful AI assistant for business operations. " +
                                    "Respond concisely and helpfully in the user's language."
                                )
                                loadedModelName = name
                                statusMessage = "✅ $name loaded and ready!"
                                chatMessages.clear()
                                selectedTab = LLMTab.Chat
                            } catch (e: Exception) {
                                statusMessage = "❌ Load failed: ${e.message}"
                            }
                            isLoading = false
                        }
                    },
                    onDelete = { name, path ->
                        File(path).delete()
                        statusMessage = "$name deleted"
                    },
                )

                LLMTab.Chat -> ChatTab(
                    llm = llm,
                    messages = chatMessages,
                    input = chatInput,
                    isGenerating = isGenerating,
                    generationSpeed = generationSpeed,
                    contextUsed = contextUsed,
                    loadedModelName = loadedModelName,
                    onInputChange = { chatInput = it },
                    onSend = { query ->
                        if (llm.isLoaded && query.isNotBlank()) {
                            chatInput = ""
                            chatMessages.add(ChatMsg("user", query))
                            isGenerating = true
                            val assistantMsg = ChatMsg("assistant", "")
                            chatMessages.add(assistantMsg)

                            scope.launch {
                                try {
                                    val responseBuilder = StringBuilder()
                                    llm.getResponseAsFlow(query)
                                        .flowOn(Dispatchers.IO)
                                        .collect { token ->
                                            responseBuilder.append(token)
                                            // Update last message in-place
                                            chatMessages[chatMessages.size - 1] =
                                                assistantMsg.copy(content = responseBuilder.toString())
                                        }
                                    generationSpeed = llm.getGenerationSpeed()
                                    contextUsed = llm.getContextUsed()
                                } catch (e: Exception) {
                                    chatMessages[chatMessages.size - 1] =
                                        assistantMsg.copy(content = "⚠️ Error: ${e.message}")
                                }
                                isGenerating = false
                            }
                        }
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
data class ChatMsg(val role: String, val content: String)

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
                    "📱 Downloaded Models",
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
                "🌐 Available Models (HuggingFace)",
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
                        Icon(Icons.Default.PlayArrow, "Load", tint = MaterialTheme.colorScheme.primary)
                    }
                }
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Default.Delete, "Delete", tint = MaterialTheme.colorScheme.error)
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
                    "✅ Downloaded",
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
                    Text("Download ${model.sizeDisplay}")
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
                    Text("Load a model first", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Go to Models tab → tap ▶️ on a downloaded model",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        } else {
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
                            "📊 Context: $contextUsed",
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
                                .padding(vertical = 32.dp),
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
                                Text("🧠", fontSize = 48.sp)
                                Spacer(Modifier.height(8.dp))
                                Text(
                                    "On-Device AI — $loadedModelName",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold,
                                )
                                Text(
                                    "100% offline • No API keys • $0 cost",
                                    style = MaterialTheme.typography.bodySmall,
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
                        placeholder = { Text("Ask BizClaw locally...") },
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
                            Icon(Icons.Default.Send, "Send")
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
                        "BizClaw (Local)",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Spacer(Modifier.height(2.dp))
                }
                Text(
                    text = msg.content.ifEmpty { "..." },
                    color = if (isUser) MaterialTheme.colorScheme.onPrimary
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodyMedium,
                )
                if (isGenerating && !isUser) {
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
                Text("Load a model first to run benchmarks")
            }
        } else {
            Text(
                "⚡ Performance Benchmark",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
            Text(
                "Model: $loadedModelName",
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
                    Text("Running benchmark...")
                } else {
                    Icon(Icons.Default.Speed, null)
                    Spacer(Modifier.width(8.dp))
                    Text("Run Benchmark (pp=512, tg=128)")
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
