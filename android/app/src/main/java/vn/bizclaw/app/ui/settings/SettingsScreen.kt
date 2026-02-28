package vn.bizclaw.app.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    serverUrl: String,
    apiKey: String,
    isConnected: Boolean,
    onUpdateServer: (String, String) -> Unit,
    onBack: () -> Unit,
) {
    var url by remember { mutableStateOf(serverUrl) }
    var key by remember { mutableStateOf(apiKey) }
    var showKey by remember { mutableStateOf(false) }
    var testResult by remember { mutableStateOf<String?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Cài Đặt", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
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
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            // Connection status card
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (isConnected)
                        MaterialTheme.colorScheme.secondaryContainer
                    else
                        MaterialTheme.colorScheme.errorContainer,
                ),
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        if (isConnected) Icons.Default.CheckCircle else Icons.Default.Error,
                        contentDescription = null,
                        tint = if (isConnected)
                            MaterialTheme.colorScheme.secondary
                        else
                            MaterialTheme.colorScheme.error,
                    )
                    Spacer(Modifier.width(12.dp))
                    Column {
                        Text(
                            if (isConnected) "Đã kết nối" else "Mất kết nối",
                            fontWeight = FontWeight.SemiBold,
                        )
                        Text(
                            url,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            // Server URL
            Text("Máy Chủ", style = MaterialTheme.typography.titleMedium)

            OutlinedTextField(
                value = url,
                onValueChange = { url = it },
                label = { Text("Server URL") },
                placeholder = { Text("http://192.168.1.100:3001") },
                modifier = Modifier.fillMaxWidth(),
                leadingIcon = { Icon(Icons.Default.Dns, null) },
                singleLine = true,
            )

            // API Key
            OutlinedTextField(
                value = key,
                onValueChange = { key = it },
                label = { Text("API Key (optional)") },
                placeholder = { Text("Pairing code từ server") },
                modifier = Modifier.fillMaxWidth(),
                leadingIcon = { Icon(Icons.Default.Key, null) },
                singleLine = true,
                visualTransformation = if (showKey)
                    VisualTransformation.None
                else
                    PasswordVisualTransformation(),
                trailingIcon = {
                    IconButton(onClick = { showKey = !showKey }) {
                        Icon(
                            if (showKey) Icons.Default.VisibilityOff
                            else Icons.Default.Visibility,
                            contentDescription = "Toggle key visibility",
                        )
                    }
                },
            )

            // Save + Test buttons
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                OutlinedButton(
                    onClick = {
                        onUpdateServer(url, key)
                        testResult = "Testing..."
                    },
                    modifier = Modifier.weight(1f),
                ) {
                    Icon(Icons.Default.NetworkCheck, null)
                    Spacer(Modifier.width(8.dp))
                    Text("Test")
                }

                Button(
                    onClick = {
                        onUpdateServer(url, key)
                        onBack()
                    },
                    modifier = Modifier.weight(1f),
                ) {
                    Icon(Icons.Default.Save, null)
                    Spacer(Modifier.width(8.dp))
                    Text("Lưu")
                }
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            // Preset servers
            Text("Máy Chủ Nhanh", style = MaterialTheme.typography.titleMedium)

            PresetServerButton(
                label = "🏠 Local (localhost)",
                serverUrl = "http://localhost:3001",
                onClick = { url = it },
            )
            PresetServerButton(
                label = "🌐 VPS (bizclaw.vn)",
                serverUrl = "https://apps.bizclaw.vn",
                onClick = { url = it },
            )
            PresetServerButton(
                label = "🍓 Raspberry Pi",
                serverUrl = "http://raspberrypi.local:3001",
                onClick = { url = it },
            )

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            // App info
            Text("Thông tin", style = MaterialTheme.typography.titleMedium)
            Text(
                "BizClaw Android v0.3.0\nThin client kết nối bizclaw-gateway",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
fun PresetServerButton(
    label: String,
    serverUrl: String,
    onClick: (String) -> Unit,
) {
    OutlinedCard(
        onClick = { onClick(serverUrl) },
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(label, modifier = Modifier.weight(1f))
            Text(
                serverUrl,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
