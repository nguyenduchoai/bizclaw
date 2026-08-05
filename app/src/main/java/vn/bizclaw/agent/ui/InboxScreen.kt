package vn.bizclaw.agent.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.data.Exchange
import vn.bizclaw.agent.data.ReplyState
import vn.bizclaw.agent.messaging.ReplySender
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private val TIME_FORMAT = SimpleDateFormat("HH:mm dd/MM", Locale.getDefault())

@Composable
fun InboxScreen(app: BizClawApp) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(app.exchanges.items, key = { it.id }) { exchange ->
            ExchangeCard(app, exchange)
        }
        if (app.exchanges.items.isEmpty()) {
            item {
                EmptyCard(
                    "Chưa có tin nhắn nào. Bật agent ở tab Trạng thái, rồi nhờ ai đó " +
                        "nhắn thử vào Zalo/Messenger của anh.",
                )
            }
        }
    }
}

@Composable
private fun ExchangeCard(app: BizClawApp, exchange: Exchange) {
    val context = LocalContext.current

    ElevatedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    exchange.sender,
                    modifier = Modifier.weight(1f),
                    fontWeight = FontWeight.SemiBold,
                )
                StatusPill(statusLabel(exchange.state), statusColor(exchange.state))
            }
            Text(
                "${exchange.channel.label} · ${TIME_FORMAT.format(Date(exchange.receivedAt))}" +
                    if (exchange.latencyMs > 0) " · ${exchange.latencyMs / 1000}s" else "",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            Text(exchange.incoming, style = MaterialTheme.typography.bodyMedium)

            if (exchange.reply.isNotBlank()) {
                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                ) {
                    Text(
                        exchange.reply,
                        modifier = Modifier.padding(12.dp),
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }

            exchange.error?.let {
                Text(it, color = Bad, style = MaterialTheme.typography.bodySmall)
            }

            if (exchange.isRepliable) {
                val canSend = ReplySender.canSend(exchange.id)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(
                        enabled = canSend,
                        onClick = {
                            val error = ReplySender.send(context, exchange.id, exchange.reply)
                            app.exchanges.update(exchange.id) { current ->
                                if (error == null) current.copy(state = ReplyState.SENT)
                                else current.copy(state = ReplyState.FAILED, error = error)
                            }
                        },
                        modifier = Modifier.weight(1f),
                    ) { Text(if (canSend) "Gửi" else "Hết hạn") }
                    OutlinedButton(
                        onClick = {
                            ReplySender.forget(exchange.id)
                            app.exchanges.update(exchange.id) {
                                it.copy(state = ReplyState.DISMISSED)
                            }
                        },
                        modifier = Modifier.weight(1f),
                    ) { Text("Bỏ qua") }
                }
                if (!canSend) {
                    Text(
                        "Thông báo gốc đã bị đóng nên không trả lời trực tiếp được nữa — " +
                            "mở app chat và dán tay.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

private fun statusLabel(state: ReplyState) = when (state) {
    ReplyState.DRAFT -> "chờ duyệt"
    ReplyState.SENT -> "đã gửi"
    ReplyState.DISMISSED -> "bỏ qua"
    ReplyState.FAILED -> "lỗi"
}

private fun statusColor(state: ReplyState) = when (state) {
    ReplyState.SENT -> Ok
    ReplyState.DRAFT -> Warn
    ReplyState.FAILED -> Bad
    ReplyState.DISMISSED -> Warn
}
