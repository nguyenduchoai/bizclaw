package vn.bizclaw.agent.ui

import android.content.Intent
import android.provider.Settings
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.llm.ApiKeyStore
import vn.bizclaw.agent.llm.ProviderKind
import vn.bizclaw.agent.messaging.ChatAutomationService

@Composable
fun ProviderScreen(app: BizClawApp) {
    val context = LocalContext.current
    val keys = app.keys
    val active = app.settings.provider
    val resolved = app.providers.resolve(active)
    val automationOn = ChatAutomationService.isEnabled(context)

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            SectionCard("Model đang dùng") {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    ProviderKind.entries.forEach { kind ->
                        FilterChip(
                            selected = active == kind,
                            onClick = { app.settings.provider = kind },
                            label = { Text(kind.label) },
                        )
                    }
                }
                if (resolved.kind != active) {
                    Text(
                        "${active.label} chưa có API key nên đang chạy tạm bằng ${resolved.kind.label}.",
                        color = Warn,
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
                Text(
                    if (active.onDevice) {
                        "Chạy trên máy: tin nhắn khách không rời khỏi điện thoại, " +
                            "nhưng agent không gọi tool được — chốt đơn đi đường bóc tách JSON."
                    } else {
                        "Chạy trên cloud: mạnh hơn nhiều và gọi tool được (tra giá, tra chính " +
                            "sách, tạo đơn). Đổi lại, nội dung tin nhắn khách được gửi tới " +
                            "${active.label} và anh trả phí theo token."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        item {
            SectionCard("Claude") {
                OutlinedTextField(
                    value = keys.claudeKey,
                    onValueChange = { keys.claudeKey = it },
                    label = { Text("API key") },
                    placeholder = { Text("sk-ant-...") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                )
                OutlinedTextField(
                    value = keys.claudeModel,
                    onValueChange = { keys.claudeModel = it },
                    label = { Text("Model") },
                    placeholder = { Text(ApiKeyStore.DEFAULT_CLAUDE_MODEL) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Text(
                    "Lấy key ở console.anthropic.com. Mặc định ${ApiKeyStore.DEFAULT_CLAUDE_MODEL}; " +
                        "muốn rẻ hơn thì đổi sang claude-sonnet-5 hoặc claude-haiku-4-5.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        item {
            SectionCard("OpenAI") {
                OutlinedTextField(
                    value = keys.openAiKey,
                    onValueChange = { keys.openAiKey = it },
                    label = { Text("API key") },
                    placeholder = { Text("sk-...") },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                )
                OutlinedTextField(
                    value = keys.openAiModel,
                    onValueChange = { keys.openAiModel = it },
                    label = { Text("Model") },
                    placeholder = { Text(ApiKeyStore.DEFAULT_OPENAI_MODEL) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Text(
                    "Lấy key ở platform.openai.com. Tên model gõ tay được — nếu API báo model " +
                        "không tồn tại thì đổi sang tên đang có trong tài khoản của anh.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        item {
            SectionCard("Tự nhắn tin trước") {
                StatusPill(if (automationOn) "đã bật" else "chưa bật", if (automationOn) Ok else Warn)
                Text(
                    "Cho phép app tự mở Zalo/Messenger, tìm khách và gửi tin chăm sóc sau bán. " +
                        "Chỉ chạy khi anh bấm nút gửi trong tab Đơn hàng — agent không tự ý nhắn.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    "Lưu ý: cách này bám vào giao diện Zalo/Messenger nên có thể hỏng khi hai " +
                        "app đó cập nhật. Hỏng thì app báo rõ bước nào lỗi, anh gửi tay.",
                    style = MaterialTheme.typography.bodySmall,
                    color = Warn,
                )
                Button(
                    onClick = {
                        context.startActivity(
                            Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
                                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                        )
                    },
                    modifier = Modifier.fillMaxWidth(),
                ) { Text(if (automationOn) "Mở cài đặt Accessibility" else "Bật quyền Accessibility") }
            }
        }
    }
}
