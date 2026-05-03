package vn.bizclaw.app.ui.agents

import androidx.compose.animation.AnimatedVisibility
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import vn.bizclaw.app.engine.LocalRAG

/**
 * Full-screen Knowledge Base Manager.
 *
 * Features:
 * - List all knowledge bases
 * - Create new KB
 * - Open KB → see all documents
 * - Add/edit/delete documents
 * - Search test
 */

// ═══════════════════════════════════════════════════════════════
// Knowledge Base List Screen
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KnowledgeBaseScreen(
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    var kbList by remember { mutableStateOf(LocalRAG.listKnowledgeBases(context)) }
    var showCreateDialog by remember { mutableStateOf(false) }
    var openedKB by remember { mutableStateOf<String?>(null) }

    // If a KB is opened, show its document editor
    openedKB?.let { kbId ->
        KBDocumentEditor(
            kbId = kbId,
            onBack = {
                openedKB = null
                kbList = LocalRAG.listKnowledgeBases(context) // Refresh
            },
        )
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("📚 Kho Kiến Thức", fontWeight = FontWeight.Bold)
                        Text(
                            "${kbList.size} kho",
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
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showCreateDialog = true },
                containerColor = MaterialTheme.colorScheme.primary,
            ) {
                Icon(Icons.Default.Add, "Tạo kho mới")
            }
        },
    ) { padding ->
        if (kbList.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("📚", fontSize = 64.sp)
                    Spacer(Modifier.height(16.dp))
                    Text(
                        "Chưa có kho kiến thức nào",
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text(
                        "Tạo kho để nhập dữ liệu FAQ, giá cả, sản phẩm...\nAgent sẽ dùng kiến thức này để trả lời chính xác hơn",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(16.dp))
                    Button(onClick = { showCreateDialog = true }) {
                        Icon(Icons.Default.Add, null)
                        Spacer(Modifier.width(8.dp))
                        Text("Tạo kho đầu tiên")
                    }
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier.padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(kbList) { kbId ->
                    val rag = remember(kbId) { LocalRAG(context, kbId) }
                    KBCard(
                        kbId = kbId,
                        docCount = rag.size(),
                        onOpen = { openedKB = kbId },
                        onDelete = {
                            LocalRAG.deleteKnowledgeBase(context, kbId)
                            kbList = LocalRAG.listKnowledgeBases(context)
                        },
                    )
                }
            }
        }
    }

    // Create KB dialog
    if (showCreateDialog) {
        var newName by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { showCreateDialog = false },
            title = { Text("Tạo Kho Kiến Thức", fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Đặt tên cho kho kiến thức (không dấu, dùng gạch dưới)",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    OutlinedTextField(
                        value = newName,
                        onValueChange = {
                            newName = it.lowercase()
                                .replace(" ", "_")
                                .replace(Regex("[^a-z0-9_]"), "")
                        },
                        label = { Text("Tên kho") },
                        placeholder = { Text("VD: san_pham, faq, gia_ca") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )

                    // Quick templates
                    Text("Mẫu gợi ý:", style = MaterialTheme.typography.labelSmall)
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("san_pham", "faq", "gia_ca", "chinh_sach").forEach { template ->
                            SuggestionChip(
                                onClick = { newName = template },
                                label = { Text(template, style = MaterialTheme.typography.labelSmall) },
                            )
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (newName.isNotBlank()) {
                            LocalRAG(context, newName) // Creates the file
                            kbList = LocalRAG.listKnowledgeBases(context)
                            showCreateDialog = false
                            openedKB = newName // Open it immediately to add docs
                        }
                    },
                    enabled = newName.isNotBlank(),
                ) {
                    Text("Tạo & Nhập liệu")
                }
            },
            dismissButton = {
                TextButton(onClick = { showCreateDialog = false }) { Text("Huỷ") }
            },
        )
    }
}

// ═══════════════════════════════════════════════════════════════
// KB Card
// ═══════════════════════════════════════════════════════════════

@Composable
private fun KBCard(
    kbId: String,
    docCount: Int,
    onOpen: () -> Unit,
    onDelete: () -> Unit,
) {
    var showDeleteConfirm by remember { mutableStateOf(false) }

    Card(
        onClick = onOpen,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                shape = RoundedCornerShape(12.dp),
                color = MaterialTheme.colorScheme.primaryContainer,
                modifier = Modifier.size(48.dp),
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Text("📚", fontSize = 24.sp)
                }
            }

            Spacer(Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    kbId,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    "$docCount tài liệu",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            IconButton(onClick = onOpen) {
                Icon(Icons.Default.Edit, "Mở")
            }
            IconButton(onClick = { showDeleteConfirm = true }) {
                Icon(
                    Icons.Default.Delete,
                    "Xoá",
                    tint = MaterialTheme.colorScheme.error,
                )
            }
        }
    }

    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text("Xoá kho \"$kbId\"?") },
            text = { Text("Tất cả $docCount tài liệu sẽ bị xoá vĩnh viễn.") },
            confirmButton = {
                Button(
                    onClick = { showDeleteConfirm = false; onDelete() },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error,
                    ),
                ) { Text("Xoá") }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) { Text("Huỷ") }
            },
        )
    }
}

// ═══════════════════════════════════════════════════════════════
// KB Document Editor — Full screen for one knowledge base
// ═══════════════════════════════════════════════════════════════

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun KBDocumentEditor(
    kbId: String,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val rag = remember { LocalRAG(context, kbId) }
    var documents by remember { mutableStateOf(rag.getDocuments()) }
    var newDocContent by remember { mutableStateOf("") }
    var searchQuery by remember { mutableStateOf("") }
    var searchResults by remember { mutableStateOf<List<LocalRAG.Document>?>(null) }
    var editingDocId by remember { mutableStateOf<String?>(null) }
    var editContent by remember { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("📚 $kbId", fontWeight = FontWeight.Bold)
                        Text(
                            "${documents.size} tài liệu",
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
                    IconButton(onClick = {
                        rag.clear()
                        documents = rag.getDocuments()
                    }) {
                        Icon(Icons.Default.DeleteSweep, "Xoá hết")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            // ─── Input area for new documents ──────────────
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(12.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f),
                ),
            ) {
                Column(modifier = Modifier.padding(12.dp)) {
                    Text(
                        "➕ Thêm kiến thức mới",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(4.dp))
                    Text(
                        "Nhập từng mục: giá sản phẩm, câu hỏi thường gặp, chính sách...\n" +
                        "Mỗi mục nên là 1-3 câu ngắn gọn để AI tìm chính xác hơn.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(8.dp))
                    OutlinedTextField(
                        value = newDocContent,
                        onValueChange = { newDocContent = it },
                        placeholder = {
                            Text("VD: Sản phẩm X có giá 500.000đ. Miễn phí vận chuyển nội thành HCM.")
                        },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(120.dp),
                        maxLines = 6,
                    )
                    Spacer(Modifier.height(8.dp))
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Button(
                            onClick = {
                                if (newDocContent.isNotBlank()) {
                                    rag.addDocument(newDocContent.trim())
                                    newDocContent = ""
                                    documents = rag.getDocuments()
                                }
                            },
                            modifier = Modifier.weight(1f),
                            enabled = newDocContent.isNotBlank(),
                        ) {
                            Icon(Icons.Default.Add, null)
                            Spacer(Modifier.width(4.dp))
                            Text("Thêm")
                        }
                        OutlinedButton(
                            onClick = {
                                // Bulk add: split by newlines
                                if (newDocContent.isNotBlank()) {
                                    val lines = newDocContent.trim()
                                        .split("\n")
                                        .map { it.trim() }
                                        .filter { it.isNotBlank() }
                                    lines.forEach { rag.addDocument(it) }
                                    newDocContent = ""
                                    documents = rag.getDocuments()
                                }
                            },
                            modifier = Modifier.weight(1f),
                            enabled = newDocContent.contains("\n"),
                        ) {
                            Text("Thêm nhiều dòng")
                        }
                    }
                }
            }

            // ─── Search test ──────────────────────────
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 12.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.tertiaryContainer.copy(alpha = 0.3f),
                ),
            ) {
                Column(modifier = Modifier.padding(12.dp)) {
                    Text(
                        "🔍 Thử tìm kiếm",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(4.dp))
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        OutlinedTextField(
                            value = searchQuery,
                            onValueChange = { searchQuery = it },
                            placeholder = { Text("Hỏi thử: giá sản phẩm X?") },
                            modifier = Modifier.weight(1f),
                            singleLine = true,
                        )
                        IconButton(
                            onClick = {
                                searchResults = rag.search(searchQuery, topK = 3)
                            },
                            enabled = searchQuery.isNotBlank(),
                        ) {
                            Icon(Icons.Default.Search, "Tìm")
                        }
                    }

                    // Search results
                    searchResults?.let { results ->
                        Spacer(Modifier.height(4.dp))
                        if (results.isEmpty()) {
                            Text(
                                "❌ Không tìm thấy kết quả phù hợp",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                            )
                        } else {
                            Text(
                                "✅ ${results.size} kết quả:",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.tertiary,
                            )
                            results.forEach { doc ->
                                Text(
                                    "• ${doc.content}",
                                    style = MaterialTheme.typography.bodySmall,
                                    maxLines = 2,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                    }
                }
            }

            Spacer(Modifier.height(8.dp))

            // ─── Document list ──────────────────────────
            Text(
                "  📄 Tài liệu (${documents.size})",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(horizontal = 12.dp),
            )

            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = PaddingValues(12.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                if (documents.isEmpty()) {
                    item {
                        Card(
                            modifier = Modifier.fillMaxWidth(),
                            colors = CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            ),
                        ) {
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(32.dp),
                                contentAlignment = Alignment.Center,
                            ) {
                                Text(
                                    "Chưa có tài liệu. Thêm kiến thức ở trên! ☝️",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }

                items(documents) { doc ->
                    val isEditing = editingDocId == doc.id
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(
                            containerColor = if (isEditing)
                                MaterialTheme.colorScheme.secondaryContainer
                            else
                                MaterialTheme.colorScheme.surfaceVariant,
                        ),
                    ) {
                        if (isEditing) {
                            // Edit mode
                            Column(modifier = Modifier.padding(12.dp)) {
                                OutlinedTextField(
                                    value = editContent,
                                    onValueChange = { editContent = it },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(100.dp),
                                    maxLines = 5,
                                )
                                Spacer(Modifier.height(4.dp))
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Button(
                                        onClick = {
                                            rag.removeDocument(doc.id)
                                            rag.addDocument(editContent.trim())
                                            editingDocId = null
                                            documents = rag.getDocuments()
                                        },
                                        enabled = editContent.isNotBlank(),
                                    ) { Text("Lưu") }
                                    TextButton(onClick = { editingDocId = null }) {
                                        Text("Huỷ")
                                    }
                                }
                            }
                        } else {
                            // View mode
                            Row(
                                modifier = Modifier.padding(12.dp),
                                verticalAlignment = Alignment.Top,
                            ) {
                                Text(
                                    doc.content,
                                    modifier = Modifier.weight(1f),
                                    style = MaterialTheme.typography.bodySmall,
                                )
                                // Edit button
                                IconButton(
                                    onClick = {
                                        editingDocId = doc.id
                                        editContent = doc.content
                                    },
                                    modifier = Modifier.size(28.dp),
                                ) {
                                    Icon(
                                        Icons.Default.Edit,
                                        "Sửa",
                                        modifier = Modifier.size(16.dp),
                                    )
                                }
                                // Delete button
                                IconButton(
                                    onClick = {
                                        rag.removeDocument(doc.id)
                                        documents = rag.getDocuments()
                                    },
                                    modifier = Modifier.size(28.dp),
                                ) {
                                    Icon(
                                        Icons.Default.Close,
                                        "Xoá",
                                        modifier = Modifier.size(16.dp),
                                        tint = MaterialTheme.colorScheme.error,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
