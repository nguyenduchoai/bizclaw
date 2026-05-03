package vn.bizclaw.app.ui.meeting

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.*
import kotlinx.coroutines.Dispatchers
import vn.bizclaw.app.engine.ProviderChat
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.service.AppController
import vn.bizclaw.app.service.AudioRecorder
import vn.bizclaw.app.service.CalendarIntegration
import vn.bizclaw.app.service.SpeechToText
import java.io.File
import java.text.SimpleDateFormat
import java.util.*

// ═══════════════════════════════════════════════════════
// Meeting Settings — persistent config via SharedPreferences
// ═══════════════════════════════════════════════════════

private const val PREFS_NAME = "meeting_settings"
private const val KEY_PROMPT = "recap_prompt"
private const val KEY_ZALO_CONTACT = "default_zalo_contact"
private const val KEY_EMAIL = "default_email"
private const val KEY_AUTO_SEND = "auto_send_recap"

private val DEFAULT_PROMPT = """Bạn là trợ lý AI chuyên recap cuộc họp.

Hãy tạo bản recap cuộc họp với format sau:

📋 RECAP CUỘC HỌP
📅 Thời gian: [ngày giờ từ tên file]
⏱️ Thời lượng: [ước tính từ kích thước file]

🎯 Các điểm chính:
1. [Điểm quan trọng 1]
2. [Điểm quan trọng 2]
3. [Điểm quan trọng 3]

📌 Hành động tiếp theo:
- [Action item 1]
- [Action item 2]

💡 Ghi chú thêm:
[Các nhận xét khác]""".trimIndent()

data class MeetingConfig(
    val recapPrompt: String = DEFAULT_PROMPT,
    val defaultZaloContact: String = "",
    val defaultEmail: String = "",
    val autoSendRecap: Boolean = false,
)

private fun loadMeetingConfig(context: Context): MeetingConfig {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    return MeetingConfig(
        recapPrompt = prefs.getString(KEY_PROMPT, DEFAULT_PROMPT) ?: DEFAULT_PROMPT,
        defaultZaloContact = prefs.getString(KEY_ZALO_CONTACT, "") ?: "",
        defaultEmail = prefs.getString(KEY_EMAIL, "") ?: "",
        autoSendRecap = prefs.getBoolean(KEY_AUTO_SEND, false),
    )
}

private fun saveMeetingConfig(context: Context, config: MeetingConfig) {
    context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        .edit()
        .putString(KEY_PROMPT, config.recapPrompt)
        .putString(KEY_ZALO_CONTACT, config.defaultZaloContact)
        .putString(KEY_EMAIL, config.defaultEmail)
        .putBoolean(KEY_AUTO_SEND, config.autoSendRecap)
        .apply()
}

/**
 * MeetingScreen — Record meetings, transcribe with AI, and share via Zalo/Email.
 *
 * Features:
 * 1. 🎙️ One-tap recording with live timer + waveform animation
 * 2. 📋 List all saved recordings with size/date
 * 3. 🤖 AI recap: transcribe + summarize via configured LLM provider
 * 4. 📨 Send recap to Zalo contact (by name or phone number) / Email
 * 5. ⚙️ Settings: custom prompt, default contacts, auto-send
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MeetingScreen(
    onBack: () -> Unit = {},
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    // Audio recorder
    val recorder = remember { AudioRecorder(context) }
    var isRecording by remember { mutableStateOf(false) }
    var elapsedMs by remember { mutableLongStateOf(0L) }
    var recordings by remember { mutableStateOf(recorder.listRecordings()) }

    // Settings
    var meetingConfig by remember { mutableStateOf(loadMeetingConfig(context)) }
    var showSettings by remember { mutableStateOf(false) }

    // Recap state
    var recapText by remember { mutableStateOf<String?>(null) }
    var recapFileName by remember { mutableStateOf<String?>(null) }
    var isRecapping by remember { mutableStateOf(false) }

    // Zalo send dialog
    var showZaloDialog by remember { mutableStateOf(false) }
    var zaloContact by remember { mutableStateOf(meetingConfig.defaultZaloContact) }
    var zaloMessage by remember { mutableStateOf("") }
    var isSendingZalo by remember { mutableStateOf(false) }

    // Email send dialog
    var showEmailDialog by remember { mutableStateOf(false) }
    var emailAddress by remember { mutableStateOf(meetingConfig.defaultEmail) }
    var emailMessage by remember { mutableStateOf("") }

    // Saved recaps (file -> recap text)
    val savedRecaps = remember { mutableStateMapOf<String, String>() }

    // Permission launcher
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            if (recorder.startRecording()) {
                isRecording = true
            }
        } else {
            Toast.makeText(context, "Cần quyền ghi âm để sử dụng", Toast.LENGTH_SHORT).show()
        }
    }

    // Timer update
    LaunchedEffect(isRecording) {
        while (isRecording) {
            elapsedMs = recorder.getElapsedMs()
            delay(100)
        }
    }

    // Load saved recaps
    LaunchedEffect(Unit) {
        val recapDir = File(context.filesDir, "recaps")
        if (recapDir.exists()) {
            recapDir.listFiles()?.forEach { file ->
                val recordingName = file.nameWithoutExtension
                savedRecaps[recordingName] = file.readText()
            }
        }
    }

    // Auto-send helper
    fun autoSendRecap(recap: String) {
        if (!meetingConfig.autoSendRecap) return
        if (meetingConfig.defaultZaloContact.isNotBlank()) {
            scope.launch {
                try {
                    val controller = AppController(context)
                    val result = controller.zaloSendMessage(
                        meetingConfig.defaultZaloContact, recap
                    )
                    withContext(Dispatchers.Main) {
                        Toast.makeText(
                            context,
                            if (result.success) "✅ Tự động gửi Zalo thành công"
                            else "❌ Zalo: ${result.message}",
                            Toast.LENGTH_SHORT,
                        ).show()
                    }
                } catch (e: Exception) {
                    withContext(Dispatchers.Main) {
                        Toast.makeText(
                            context,
                            "❌ Auto-send Zalo lỗi: ${e.message?.take(60)}",
                            Toast.LENGTH_SHORT,
                        ).show()
                    }
                }
            }
        }
        if (meetingConfig.defaultEmail.isNotBlank()) {
            scope.launch {
                try {
                    val intent = android.content.Intent(android.content.Intent.ACTION_SEND).apply {
                        type = "text/plain"
                        putExtra(android.content.Intent.EXTRA_EMAIL, arrayOf(meetingConfig.defaultEmail))
                        putExtra(android.content.Intent.EXTRA_SUBJECT, "📋 Recap Cuộc Họp — BizClaw")
                        putExtra(android.content.Intent.EXTRA_TEXT, recap)
                    }
                    withContext(Dispatchers.Main) {
                        context.startActivity(
                            android.content.Intent.createChooser(intent, "Gửi email recap")
                        )
                    }
                } catch (_: Exception) { }
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("Ghi Âm Cuộc Họp", fontWeight = FontWeight.Bold)
                        Text(
                            "${recordings.size} bản ghi",
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
                    IconButton(onClick = { showSettings = !showSettings }) {
                        Icon(
                            Icons.Default.Settings,
                            "Cài đặt",
                            tint = if (showSettings) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
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
            // ═══════════════════════════════════════════════
            // Settings Panel (expandable)
            // ═══════════════════════════════════════════════
            AnimatedVisibility(
                visible = showSettings,
                enter = fadeIn() + expandVertically(),
                exit = fadeOut() + shrinkVertically(),
            ) {
                SettingsPanel(
                    config = meetingConfig,
                    onConfigChange = { newConfig ->
                        meetingConfig = newConfig
                        saveMeetingConfig(context, newConfig)
                    },
                )
            }

            // ═══════════════════════════════════════════════
            // Recording Control Panel
            // ═══════════════════════════════════════════════
            RecordingPanel(
                isRecording = isRecording,
                elapsedMs = elapsedMs,
                onStartRecording = {
                    val hasPerm = ContextCompat.checkSelfPermission(
                        context, Manifest.permission.RECORD_AUDIO
                    ) == PackageManager.PERMISSION_GRANTED

                    if (hasPerm) {
                        if (recorder.startRecording()) {
                            isRecording = true
                        }
                    } else {
                        permissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
                    }
                },
                onStopRecording = {
                    val result = recorder.stopRecording()
                    isRecording = false
                    elapsedMs = 0
                    if (result != null) {
                        recordings = recorder.listRecordings()
                        Toast.makeText(
                            context,
                            "✅ Đã lưu: ${result.fileName}",
                            Toast.LENGTH_SHORT,
                        ).show()
                    }
                },
                onCancelRecording = {
                    recorder.cancelRecording()
                    isRecording = false
                    elapsedMs = 0
                },
            )

            // ═══════════════════════════════════════════════
            // Recap viewer (when a recap is available)
            // ═══════════════════════════════════════════════
            AnimatedVisibility(
                visible = recapText != null,
                enter = fadeIn() + expandVertically(),
                exit = fadeOut() + shrinkVertically(),
            ) {
                RecapViewer(
                    fileName = recapFileName ?: "",
                    recapText = recapText ?: "",
                    onClose = { recapText = null; recapFileName = null },
                    onSendZalo = {
                        zaloContact = meetingConfig.defaultZaloContact
                        zaloMessage = recapText ?: ""
                        showZaloDialog = true
                    },
                    onSendEmail = {
                        emailAddress = meetingConfig.defaultEmail
                        emailMessage = recapText ?: ""
                        showEmailDialog = true
                    },
                )
            }

            // ═══════════════════════════════════════════════
            // Recordings List
            // ═══════════════════════════════════════════════
            if (recordings.isEmpty() && !isRecording) {
                EmptyRecordingsPlaceholder()
            } else {
                LazyColumn(
                    modifier = Modifier.weight(1f),
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(recordings, key = { it.filePath }) { recording ->
                        RecordingCard(
                            recording = recording,
                            hasRecap = savedRecaps.containsKey(recording.fileName),
                            isRecapping = isRecapping && recapFileName == recording.fileName,
                            onRecap = {
                                // Check if recap already saved
                                val existing = savedRecaps[recording.fileName]
                                if (existing != null) {
                                    recapText = existing
                                    recapFileName = recording.fileName
                                    return@RecordingCard
                                }

                                isRecapping = true
                                recapFileName = recording.fileName
                                scope.launch {
                                    try {
                                        val result = generateRecap(
                                            context, recording, meetingConfig.recapPrompt
                                        )
                                        recapText = result.recap
                                        recapFileName = recording.fileName

                                        // Save recap to disk
                                        savedRecaps[recording.fileName] = result.recap
                                        val recapDir = File(context.filesDir, "recaps")
                                        recapDir.mkdirs()
                                        File(recapDir, recording.fileName).writeText(result.recap)

                                        // Show transcription info
                                        if (result.transcript != null) {
                                            recapText = "📝 Transcription: ${result.transcript.take(200)}...\n\n${result.recap}"
                                        }

                                        // Auto-send if enabled
                                        autoSendRecap(result.recap)
                                    } catch (e: Exception) {
                                        recapText = "❌ Lỗi: ${e.message}"
                                        recapFileName = recording.fileName
                                    }
                                    isRecapping = false
                                }
                            },
                            onViewRecap = {
                                val existing = savedRecaps[recording.fileName]
                                if (existing != null) {
                                    recapText = existing
                                    recapFileName = recording.fileName
                                }
                            },
                            onSendZalo = {
                                val existing = savedRecaps[recording.fileName]
                                if (existing != null) {
                                    zaloContact = meetingConfig.defaultZaloContact
                                    zaloMessage = existing
                                    showZaloDialog = true
                                } else {
                                    Toast.makeText(
                                        context, "Hãy tạo recap trước", Toast.LENGTH_SHORT
                                    ).show()
                                }
                            },
                            onDelete = {
                                recorder.deleteRecording(recording.filePath)
                                savedRecaps.remove(recording.fileName)
                                File(context.filesDir, "recaps/${recording.fileName}").delete()
                                recordings = recorder.listRecordings()
                            },
                        )
                    }
                }
            }
        }
    }

    // ═══════════════════════════════════════════════
    // Zalo Send Dialog
    // ═══════════════════════════════════════════════
    if (showZaloDialog) {
        AlertDialog(
            onDismissRequest = { if (!isSendingZalo) showZaloDialog = false },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("📨", fontSize = 24.sp)
                    Spacer(Modifier.width(8.dp))
                    Text("Gửi Recap qua Zalo")
                }
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = zaloContact,
                        onValueChange = { zaloContact = it },
                        label = { Text("Tên liên hệ / SĐT") },
                        placeholder = { Text("VD: 0901234567 hoặc Nguyễn Văn A") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )

                    // Save as default checkbox
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                val newConfig = meetingConfig.copy(
                                    defaultZaloContact = zaloContact
                                )
                                meetingConfig = newConfig
                                saveMeetingConfig(context, newConfig)
                                Toast.makeText(
                                    context,
                                    "✅ Đã lưu SĐT/tên Zalo mặc định",
                                    Toast.LENGTH_SHORT,
                                ).show()
                            }
                            .padding(vertical = 4.dp),
                    ) {
                        Icon(
                            Icons.Default.Save,
                            null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(16.dp),
                        )
                        Spacer(Modifier.width(6.dp))
                        Text(
                            "Lưu làm SĐT Zalo mặc định",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }

                    OutlinedTextField(
                        value = zaloMessage,
                        onValueChange = { zaloMessage = it },
                        label = { Text("Nội dung") },
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 120.dp, max = 200.dp),
                        maxLines = 10,
                    )

                    if (isSendingZalo) {
                        LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (zaloContact.isBlank() || zaloMessage.isBlank()) return@Button
                        isSendingZalo = true
                        scope.launch {
                            try {
                                val controller = AppController(context)
                                val result = controller.zaloSendMessage(zaloContact, zaloMessage)
                                withContext(Dispatchers.Main) {
                                    Toast.makeText(
                                        context,
                                        if (result.success) "✅ Đã gửi" else "❌ ${result.message}",
                                        Toast.LENGTH_LONG,
                                    ).show()
                                }
                                if (result.success) {
                                    showZaloDialog = false
                                    zaloContact = ""
                                }
                            } catch (e: Exception) {
                                withContext(Dispatchers.Main) {
                                    Toast.makeText(
                                        context,
                                        "❌ Lỗi: ${e.message?.take(80)}",
                                        Toast.LENGTH_LONG,
                                    ).show()
                                }
                            }
                            isSendingZalo = false
                        }
                    },
                    enabled = !isSendingZalo && zaloContact.isNotBlank() && zaloMessage.isNotBlank(),
                ) {
                    Icon(Icons.AutoMirrored.Filled.Send, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Gửi Zalo")
                }
            },
            dismissButton = {
                TextButton(
                    onClick = { showZaloDialog = false },
                    enabled = !isSendingZalo,
                ) {
                    Text("Huỷ")
                }
            },
        )
    }

    // ═══════════════════════════════════════════════
    // Email Send Dialog
    // ═══════════════════════════════════════════════
    if (showEmailDialog) {
        AlertDialog(
            onDismissRequest = { showEmailDialog = false },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("📧", fontSize = 24.sp)
                    Spacer(Modifier.width(8.dp))
                    Text("Gửi Recap qua Email")
                }
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = emailAddress,
                        onValueChange = { emailAddress = it },
                        label = { Text("Email người nhận") },
                        placeholder = { Text("VD: sếp@company.com") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )

                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                val newConfig = meetingConfig.copy(defaultEmail = emailAddress)
                                meetingConfig = newConfig
                                saveMeetingConfig(context, newConfig)
                                Toast.makeText(
                                    context, "✅ Đã lưu email mặc định", Toast.LENGTH_SHORT
                                ).show()
                            }
                            .padding(vertical = 4.dp),
                    ) {
                        Icon(
                            Icons.Default.Save,
                            null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(16.dp),
                        )
                        Spacer(Modifier.width(6.dp))
                        Text(
                            "Lưu làm email mặc định",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (emailAddress.isBlank()) return@Button
                        try {
                            val intent = android.content.Intent(
                                android.content.Intent.ACTION_SEND
                            ).apply {
                                type = "text/plain"
                                putExtra(
                                    android.content.Intent.EXTRA_EMAIL,
                                    arrayOf(emailAddress)
                                )
                                putExtra(
                                    android.content.Intent.EXTRA_SUBJECT,
                                    "📋 Recap Cuộc Họp — BizClaw"
                                )
                                putExtra(android.content.Intent.EXTRA_TEXT, emailMessage)
                            }
                            context.startActivity(
                                android.content.Intent.createChooser(intent, "Gửi email recap")
                            )
                            showEmailDialog = false
                        } catch (e: Exception) {
                            Toast.makeText(
                                context,
                                "❌ Không tìm thấy ứng dụng email",
                                Toast.LENGTH_SHORT,
                            ).show()
                        }
                    },
                    enabled = emailAddress.isNotBlank(),
                ) {
                    Icon(Icons.Default.Email, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Gửi Email")
                }
            },
            dismissButton = {
                TextButton(onClick = { showEmailDialog = false }) {
                    Text("Huỷ")
                }
            },
        )
    }
}

// ═══════════════════════════════════════════════════════
// Settings Panel
// ═══════════════════════════════════════════════════════

@Composable
private fun SettingsPanel(
    config: MeetingConfig,
    onConfigChange: (MeetingConfig) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var editPrompt by remember(config) { mutableStateOf(config.recapPrompt) }
    var editZalo by remember(config) { mutableStateOf(config.defaultZaloContact) }
    var editEmail by remember(config) { mutableStateOf(config.defaultEmail) }
    var autoSend by remember(config) { mutableStateOf(config.autoSendRecap) }
    var isExpanded by remember { mutableStateOf(false) }

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.15f),
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Header
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("⚙️", fontSize = 20.sp)
                Spacer(Modifier.width(8.dp))
                Text(
                    "Cài Đặt Recap",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f),
                )
            }

            Spacer(Modifier.height(16.dp))

            // ── Zalo Contact ──
            OutlinedTextField(
                value = editZalo,
                onValueChange = { editZalo = it },
                label = { Text("📱 SĐT / Tên Zalo mặc định") },
                placeholder = { Text("VD: 0901234567") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Color(0xFF0068FF),
                ),
            )

            Spacer(Modifier.height(12.dp))

            // ── Email ──
            OutlinedTextField(
                value = editEmail,
                onValueChange = { editEmail = it },
                label = { Text("📧 Email người nhận mặc định") },
                placeholder = { Text("VD: manager@company.com") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(Modifier.height(12.dp))

            // ── Auto-send toggle ──
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        "🚀 Tự động gửi sau recap",
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Medium,
                    )
                    Text(
                        "Gửi recap ngay khi AI tạo xong (Zalo + Email)",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = autoSend,
                    onCheckedChange = { autoSend = it },
                )
            }

            Spacer(Modifier.height(12.dp))

            // ── Custom Prompt (collapsible) ──
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { isExpanded = !isExpanded }
                    .padding(vertical = 4.dp),
            ) {
                Text("🤖", fontSize = 16.sp)
                Spacer(Modifier.width(8.dp))
                Text(
                    "Prompt AI Recap",
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    modifier = Modifier.weight(1f),
                )
                Icon(
                    if (isExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    null,
                    modifier = Modifier.size(20.dp),
                )
            }

            AnimatedVisibility(
                visible = isExpanded,
                enter = fadeIn() + expandVertically(),
                exit = fadeOut() + shrinkVertically(),
            ) {
                Column {
                    Spacer(Modifier.height(8.dp))
                    OutlinedTextField(
                        value = editPrompt,
                        onValueChange = { editPrompt = it },
                        label = { Text("System Prompt cho AI") },
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(min = 120.dp, max = 200.dp),
                        maxLines = 15,
                        textStyle = MaterialTheme.typography.bodySmall,
                    )

                    Spacer(Modifier.height(4.dp))

                    TextButton(
                        onClick = { editPrompt = DEFAULT_PROMPT },
                        modifier = Modifier.align(Alignment.End),
                    ) {
                        Icon(Icons.Default.Restore, null, Modifier.size(14.dp))
                        Spacer(Modifier.width(4.dp))
                        Text("Khôi phục mặc định", style = MaterialTheme.typography.labelSmall)
                    }
                }
            }

            Spacer(Modifier.height(16.dp))

            // ── Save Button ──
            Button(
                onClick = {
                    onConfigChange(
                        MeetingConfig(
                            recapPrompt = editPrompt,
                            defaultZaloContact = editZalo,
                            defaultEmail = editEmail,
                            autoSendRecap = autoSend,
                        )
                    )
                    // Show confirmation
                },
                modifier = Modifier.fillMaxWidth(),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                ),
            ) {
                Icon(Icons.Default.Save, null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text("💾 Lưu Cài Đặt", fontWeight = FontWeight.Bold)
            }

            Spacer(Modifier.height(8.dp))

            // ── Calendar Integration ──
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                OutlinedButton(
                    onClick = {
                        try {
                            val calendar = CalendarIntegration(context)
                            calendar.openCalendarApp()
                        } catch (_: Exception) {
                            Toast.makeText(context, "Không mở được Calendar", Toast.LENGTH_SHORT).show()
                        }
                    },
                    modifier = Modifier.weight(1f),
                ) {
                    Icon(Icons.Default.CalendarMonth, null, Modifier.size(16.dp))
                    Spacer(Modifier.width(4.dp))
                    Text("📅 Calendar", fontSize = 12.sp)
                }

                Button(
                    onClick = {
                        try {
                            val calendar = CalendarIntegration(context)
                            scope.launch {
                                val events = calendar.getUpcomingEvents(5)
                                if (events.isEmpty()) {
                                    withContext(Dispatchers.Main) {
                                        Toast.makeText(context, "Không có sự kiện sắp tới", Toast.LENGTH_SHORT).show()
                                    }
                                } else {
                                    withContext(Dispatchers.Main) {
                                        val msg = events.take(3).joinToString("\n") {
                                            "• ${it.title} (${SimpleDateFormat("dd/MM HH:mm", Locale.getDefault()).format(Date(it.startTime))})"
                                        }
                                        Toast.makeText(context, "📅 Sắp tới:\n$msg", Toast.LENGTH_LONG).show()
                                    }
                                }
                            }
                        } catch (_: Exception) {
                            Toast.makeText(context, "Lỗi đọc Calendar", Toast.LENGTH_SHORT).show()
                        }
                    },
                    modifier = Modifier.weight(1f),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.secondary,
                    ),
                ) {
                    Icon(Icons.Default.Schedule, null, Modifier.size(16.dp))
                    Spacer(Modifier.width(4.dp))
                    Text("📋 Sự kiện", fontSize = 12.sp)
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
// Recording Control Panel
// ═══════════════════════════════════════════════════════

@Composable
private fun RecordingPanel(
    isRecording: Boolean,
    elapsedMs: Long,
    onStartRecording: () -> Unit,
    onStopRecording: () -> Unit,
    onCancelRecording: () -> Unit,
) {
    val infiniteTransition = rememberInfiniteTransition(label = "pulse")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 1f,
        targetValue = 1.15f,
        animationSpec = infiniteRepeatable(
            animation = tween(600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "pulse",
    )

    Surface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp),
        shape = RoundedCornerShape(24.dp),
        color = if (isRecording)
            Color(0xFFFF1744).copy(alpha = 0.08f)
        else
            MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        tonalElevation = 2.dp,
    ) {
        Column(
            modifier = Modifier.padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            if (isRecording) {
                // Timer
                Text(
                    text = formatDuration(elapsedMs),
                    style = MaterialTheme.typography.displaySmall,
                    fontWeight = FontWeight.Bold,
                    color = Color(0xFFFF1744),
                )

                Spacer(Modifier.height(4.dp))

                // Animated recording indicator
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Box(
                        modifier = Modifier
                            .size(10.dp)
                            .scale(pulseScale)
                            .clip(CircleShape)
                            .background(Color(0xFFFF1744))
                    )
                    Text(
                        "Đang ghi âm...",
                        style = MaterialTheme.typography.labelLarge,
                        color = Color(0xFFFF1744),
                    )
                }

                Spacer(Modifier.height(20.dp))

                // Stop / Cancel buttons
                Row(
                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    // Cancel
                    OutlinedButton(
                        onClick = onCancelRecording,
                        colors = ButtonDefaults.outlinedButtonColors(
                            contentColor = MaterialTheme.colorScheme.error,
                        ),
                    ) {
                        Icon(Icons.Default.Close, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(6.dp))
                        Text("Huỷ")
                    }

                    // Stop & Save
                    Button(
                        onClick = onStopRecording,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = Color(0xFFFF1744),
                        ),
                    ) {
                        Icon(Icons.Default.Stop, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(6.dp))
                        Text("Dừng & Lưu")
                    }
                }
            } else {
                // Start recording button
                Text(
                    "🎙️ Ghi Âm Cuộc Họp",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                )

                Spacer(Modifier.height(8.dp))

                Text(
                    "Bấm để bắt đầu ghi âm. Sau đó AI sẽ tạo recap tự động.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                Spacer(Modifier.height(16.dp))

                FilledTonalButton(
                    onClick = onStartRecording,
                    modifier = Modifier.fillMaxWidth(),
                    colors = ButtonDefaults.filledTonalButtonColors(
                        containerColor = Color(0xFFFF1744).copy(alpha = 0.12f),
                    ),
                ) {
                    Icon(
                        Icons.Default.Mic,
                        null,
                        tint = Color(0xFFFF1744),
                        modifier = Modifier.size(24.dp),
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(
                        "Bắt Đầu Ghi Âm",
                        color = Color(0xFFFF1744),
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
// Recording Card (list item)
// ═══════════════════════════════════════════════════════

@Composable
private fun RecordingCard(
    recording: AudioRecorder.RecordingResult,
    hasRecap: Boolean,
    isRecapping: Boolean,
    onRecap: () -> Unit,
    onViewRecap: () -> Unit,
    onSendZalo: () -> Unit,
    onDelete: () -> Unit,
) {
    var showDeleteConfirm by remember { mutableStateOf(false) }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                // Icon
                Surface(
                    shape = CircleShape,
                    color = if (hasRecap) Color(0xFF00E676).copy(alpha = 0.15f)
                    else MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.5f),
                    modifier = Modifier.size(44.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(if (hasRecap) "📝" else "🎙️", fontSize = 20.sp)
                    }
                }

                Spacer(Modifier.width(12.dp))

                // File info
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        recording.fileName,
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Medium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )

                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(
                            recording.sizeFormatted,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )

                        val date = try {
                            val ts = File(recording.filePath).lastModified()
                            SimpleDateFormat("HH:mm dd/MM", Locale.getDefault()).format(Date(ts))
                        } catch (_: Exception) { "" }

                        if (date.isNotEmpty()) {
                            Text(
                                date,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }

                        if (hasRecap) {
                            Text(
                                "✅ Đã recap",
                                style = MaterialTheme.typography.labelSmall,
                                color = Color(0xFF00E676),
                            )
                        }
                    }
                }

                // Delete
                IconButton(
                    onClick = { showDeleteConfirm = true },
                    modifier = Modifier.size(32.dp),
                ) {
                    Icon(
                        Icons.Default.Delete,
                        "Xoá",
                        tint = MaterialTheme.colorScheme.error.copy(alpha = 0.6f),
                        modifier = Modifier.size(18.dp),
                    )
                }
            }

            Spacer(Modifier.height(12.dp))

            // Action buttons
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (hasRecap) {
                    // View recap
                    FilledTonalButton(
                        onClick = onViewRecap,
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(Icons.Default.Description, null, Modifier.size(16.dp))
                        Spacer(Modifier.width(4.dp))
                        Text("Xem Recap", style = MaterialTheme.typography.labelMedium)
                    }

                    // Send Zalo
                    FilledTonalButton(
                        onClick = onSendZalo,
                        modifier = Modifier.weight(1f),
                        colors = ButtonDefaults.filledTonalButtonColors(
                            containerColor = Color(0xFF0068FF).copy(alpha = 0.12f),
                        ),
                    ) {
                        Text("📨", fontSize = 14.sp)
                        Spacer(Modifier.width(4.dp))
                        Text(
                            "Gửi Zalo",
                            style = MaterialTheme.typography.labelMedium,
                            color = Color(0xFF0068FF),
                        )
                    }
                } else {
                    // AI Recap button
                    Button(
                        onClick = onRecap,
                        modifier = Modifier.fillMaxWidth(),
                        enabled = !isRecapping,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primary,
                        ),
                    ) {
                        if (isRecapping) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(16.dp),
                                strokeWidth = 2.dp,
                                color = MaterialTheme.colorScheme.onPrimary,
                            )
                            Spacer(Modifier.width(8.dp))
                            Text("Đang phân tích...")
                        } else {
                            Icon(Icons.Default.AutoAwesome, null, Modifier.size(16.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("🤖 AI Recap")
                        }
                    }
                }
            }
        }
    }

    // Delete confirmation
    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text("Xoá bản ghi?") },
            text = { Text("Bạn có chắc muốn xoá ${recording.fileName}? Không thể hoàn tác.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        onDelete()
                        showDeleteConfirm = false
                    },
                    colors = ButtonDefaults.textButtonColors(
                        contentColor = MaterialTheme.colorScheme.error,
                    ),
                ) { Text("Xoá") }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirm = false }) { Text("Huỷ") }
            },
        )
    }
}

// ═══════════════════════════════════════════════════════
// Recap Viewer
// ═══════════════════════════════════════════════════════

@Composable
private fun RecapViewer(
    fileName: String,
    recapText: String,
    onClose: () -> Unit,
    onSendZalo: () -> Unit,
    onSendEmail: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(
            containerColor = Color(0xFF1B5E20).copy(alpha = 0.06f),
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("📝", fontSize = 20.sp)
                Spacer(Modifier.width(8.dp))
                Text(
                    "Recap: $fileName",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                IconButton(onClick = onClose, modifier = Modifier.size(28.dp)) {
                    Icon(Icons.Default.Close, "Đóng", Modifier.size(18.dp))
                }
            }

            Spacer(Modifier.height(8.dp))

            Surface(
                shape = RoundedCornerShape(12.dp),
                color = MaterialTheme.colorScheme.surface,
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(max = 200.dp),
            ) {
                Text(
                    text = recapText,
                    modifier = Modifier.padding(12.dp),
                    style = MaterialTheme.typography.bodySmall,
                    lineHeight = 20.sp,
                )
            }

            Spacer(Modifier.height(12.dp))

            // Action buttons: Zalo + Email
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Button(
                    onClick = onSendZalo,
                    modifier = Modifier.weight(1f),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Color(0xFF0068FF),
                    ),
                ) {
                    Text("📨", fontSize = 14.sp)
                    Spacer(Modifier.width(4.dp))
                    Text("Gửi Zalo", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                }

                OutlinedButton(
                    onClick = onSendEmail,
                    modifier = Modifier.weight(1f),
                ) {
                    Text("📧", fontSize = 14.sp)
                    Spacer(Modifier.width(4.dp))
                    Text("Gửi Email", fontSize = 13.sp)
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
// Empty State
// ═══════════════════════════════════════════════════════

@Composable
private fun EmptyRecordingsPlaceholder() {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("🎙️", fontSize = 64.sp)
        Spacer(Modifier.height(16.dp))
        Text(
            "Chưa có bản ghi nào",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            "Bấm nút Ghi Âm ở trên để bắt đầu.\nSau khi ghi xong, AI sẽ tạo recap & gửi qua Zalo.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            lineHeight = 20.sp,
        )
    }
}

// ═══════════════════════════════════════════════════════
// AI Recap Generation with Real Transcription
// ═══════════════════════════════════════════════════════

data class RecapResult(
    val recap: String,
    val transcript: String?,
    val actionItems: List<ActionItem>,
    val duration: Long,
    val provider: String,
)

data class ActionItem(
    val task: String,
    val assignee: String?,
    val deadline: String?,
    val priority: String = "normal",
)

private suspend fun generateRecap(
    context: android.content.Context,
    recording: AudioRecorder.RecordingResult,
    customPrompt: String,
): RecapResult = withContext(Dispatchers.IO) {
    val providerManager = ProviderManager(context)
    val providers = providerManager.loadProviders()
    val provider = providers.firstOrNull { it.enabled }
        ?: throw Exception("Chưa cấu hình AI Provider. Vào Settings để thêm.")

    // Step 1: Transcribe audio to text
    val stt = SpeechToText(context)
    val audioFile = File(recording.filePath)
    val transcription = stt.transcribe(audioFile)

    val transcriptText = if (transcription.success && transcription.text.isNotBlank()) {
        transcription.text
    } else {
        null
    }

    // Step 2: Build enhanced prompt with transcript
    val enhancedPrompt = buildEnhancedRecapPrompt(customPrompt, transcriptText, recording)

    // Step 3: Generate recap using LLM
    ProviderChat.appContext = context
    val recapText = ProviderChat.chat(provider, "", enhancedPrompt)

    // Step 4: Extract action items
    val actionItems = extractActionItems(recapText)

    RecapResult(
        recap = recapText,
        transcript = transcriptText,
        actionItems = actionItems,
        duration = recording.sizeFormatted.let {
            // Estimate duration from file size (rough: 1MB ≈ 1 minute for m4a)
            val sizeMb = recording.sizeFormatted.replace(Regex("[^0-9.]"), "").toFloatOrNull() ?: 0f
            (sizeMb * 60 * 1000).toLong()
        },
        provider = transcription.provider,
    )
}

private fun buildEnhancedRecapPrompt(basePrompt: String, transcript: String?, recording: AudioRecorder.RecordingResult): String {
    return buildString {
        appendLine(basePrompt)
        appendLine()
        
        if (transcript != null && transcript.isNotBlank()) {
            appendLine("=== TRANSCRIPT ===")
            appendLine(transcript)
            appendLine()
            appendLine("Hãy tạo recap dựa trên transcript thực tế ở trên.")
            appendLine()
        } else {
            appendLine("⚠️ LƯU Ý: Không có transcription. File chỉ có thông tin:")
            appendLine("- Tên file: ${recording.fileName}")
            appendLine("- Kích thước: ${recording.sizeFormatted}")
            appendLine()
            appendLine("Hãy tạo recap mẫu với placeholder. Khi có transcription, recap sẽ chi tiết hơn.")
            appendLine()
        }

        appendLine("=== YÊU CẦU THÊM ===")
        appendLine("1. Trích xuất các ACTION ITEMS rõ ràng:")
        appendLine("   - Ai làm gì (assignee)")
        appendLine("   - Deadline nếu có")
        appendLine("   - Priority: cao/trung bình/thấp")
        appendLine()
        appendLine("2. Đánh dấu các quyết định đã được đưa ra")
        appendLine()
        appendLine("3. Ghi nhận các câu hỏi cần follow-up")
    }
}

private fun extractActionItems(text: String): List<ActionItem> {
    val items = mutableListOf<ActionItem>()
    
    // Common patterns for action items in Vietnamese
    val patterns = listOf(
        Regex("(?:@|phân công|giao cho)\\s*([A-ZÀ-ỹ][a-zà-ỹ\\s]+?)(?::|,|\\s+-|$)", RegexOption.IGNORE_CASE),
        Regex("(?:deadline|hạn chót)\\s*:?\\s*(\\d{1,2}[/.-]\\d{1,2}[/.-]\\d{2,4})", RegexOption.IGNORE_CASE),
        Regex("(?:ưu tiên|priority)\\s*:?\\s*(cao|trung bình|thấp|high|medium|low)", RegexOption.IGNORE_CASE),
        Regex("[-•*]\\s*([A-ZÀ-ỹ][^.!?\n]{10,100}?(?:làm|xong|hoàn thành|gửi|báo cáo|tạo|cập nhật))", RegexOption.IGNORE_CASE),
    )
    
    // Simple extraction - look for lines with action verbs
    val actionVerbs = listOf("làm", "xong", "hoàn thành", "gửi", "báo cáo", "tạo", "cập nhật", "liên hệ", "prepare", "send", "complete", "finish", "update", "do", "call")
    
    text.lines().forEach { line ->
        val trimmed = line.trim()
        if (trimmed.startsWith("-") || trimmed.startsWith("•")) {
            val content = trimmed.removePrefix("-").removePrefix("•").trim()
            if (actionVerbs.any { content.contains(it, ignoreCase = true) }) {
                // Try to extract assignee
                val assigneeMatch = Regex("@?([A-ZÀ-ỹ][a-zà-ỹ]+)").find(content)
                val assignee = assigneeMatch?.groupValues?.getOrNull(1)
                
                // Try to extract deadline
                val deadlineMatch = Regex("\\d{1,2}[/.-]\\d{1,2}[/.-]\\d{2,4}").find(content)
                val deadline = deadlineMatch?.value
                
                // Try to extract priority
                val priority = when {
                    content.contains("ưu tiên cao", ignoreCase = true) || content.contains("priority cao", ignoreCase = true) -> "cao"
                    content.contains("ưu tiên thấp", ignoreCase = true) || content.contains("priority thấp", ignoreCase = true) -> "thấp"
                    else -> "trung bình"
                }
                
                items.add(ActionItem(
                    task = content,
                    assignee = assignee,
                    deadline = deadline,
                    priority = priority,
                ))
            }
        }
    }
    
    return items
}

// ═══════════════════════════════════════════════════════
// Utility
// ═══════════════════════════════════════════════════════

private fun formatDuration(ms: Long): String {
    val totalSecs = ms / 1000
    val hours = totalSecs / 3600
    val mins = (totalSecs % 3600) / 60
    val secs = totalSecs % 60
    return if (hours > 0) {
        String.format("%d:%02d:%02d", hours, mins, secs)
    } else {
        String.format("%02d:%02d", mins, secs)
    }
}

