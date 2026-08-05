package vn.bizclaw.agent.ui

import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import vn.bizclaw.agent.BizClawApp
import vn.bizclaw.agent.agent.PromptBuilder
import vn.bizclaw.agent.messaging.ChatAutomationService
import vn.bizclaw.agent.messaging.ProactiveSender
import vn.bizclaw.agent.messaging.SendStep
import vn.bizclaw.agent.agent.FollowUpWorker
import vn.bizclaw.agent.data.Order
import vn.bizclaw.agent.data.OrderState

@Composable
fun OrdersScreen(app: BizClawApp) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                MetricCard(
                    "Chờ xác nhận",
                    app.orders.countBy(OrderState.DRAFT).toString(),
                    Modifier.weight(1f),
                )
                MetricCard(
                    "Đã chốt",
                    app.orders.countBy(OrderState.CONFIRMED).toString(),
                    Modifier.weight(1f),
                )
            }
        }

        val due = app.orders.dueFollowUps()
        if (due.isNotEmpty()) {
            item { FollowUpCard(app, due) }
        }

        items(app.orders.orders, key = { it.id }) { order ->
            OrderCard(app, order)
        }

        if (app.orders.orders.isEmpty()) {
            item {
                EmptyCard(
                    "Chưa có đơn nào. Khi khách nhắn kèm số điện thoại hoặc nói \"chốt đơn\", " +
                        "agent sẽ tự bóc tách thành đơn ở đây.",
                )
            }
        }
    }
}

/**
 * Post-sale check-ins that are due.
 *
 * "Nhắn ngay" is the only place the app opens a chat app on its own, and it is always
 * owner-initiated: the model drafts, Accessibility types, and any failure names the
 * step that broke so the owner can finish by hand.
 */
@Composable
private fun FollowUpCard(app: BizClawApp, due: List<Order>) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var status by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val automationOn = ChatAutomationService.isEnabled(context)

    SectionCard("Đến hạn hỏi thăm (${due.size})") {
        due.forEach { order ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    "${order.customerName} · ${order.productName}",
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.bodyMedium,
                )
                OutlinedButton(
                    enabled = automationOn && !busy,
                    onClick = {
                        busy = true
                        status = "Đang soạn tin cho ${order.customerName}..."
                        scope.launch {
                            status = sendFollowUp(app, context, order)
                            busy = false
                        }
                    },
                ) { Text("Nhắn ngay") }
                Spacer(Modifier.width(8.dp))
                OutlinedButton(
                    onClick = { app.orders.update(order.id) { it.copy(followUpDone = true) } },
                ) { Text("Đã hỏi") }
            }
        }
        if (!automationOn) {
            Text(
                "Bật quyền Accessibility ở tab Model để app tự nhắn được.",
                style = MaterialTheme.typography.bodySmall,
                color = Warn,
            )
        }
        status?.let {
            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** Drafts a check-in with the active model, then types it into the chat app. */
private suspend fun sendFollowUp(app: BizClawApp, context: Context, order: Order): String {
    val provider = app.providers.resolve(app.settings.provider)
    val draft = provider.chat(
        system = "",
        userMessage = PromptBuilder.followUpPrompt(
            businessName = app.settings.businessName,
            persona = app.settings.persona,
            customer = order.customerName,
            product = order.productName,
        ),
    ).getOrElse { return "Không soạn được tin: ${it.message}" }

    val text = draft.text.trim().ifBlank { return "Model trả về rỗng" }
    return when (val step = ProactiveSender.send(context, order.channel, order.customerName, text)) {
        is SendStep.Done -> {
            app.orders.update(order.id) { it.copy(followUpDone = true) }
            "Đã gửi cho ${order.customerName}."
        }

        is SendStep.Failed -> "Dừng ở bước: ${step.reason}"
    }
}

@Composable
private fun OrderCard(app: BizClawApp, order: Order) {
    ElevatedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(order.customerName, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                StatusPill(order.state.label, stateColor(order.state))
            }
            Text(
                "${order.productName} × ${order.quantity} = ${order.totalLabel}",
                style = MaterialTheme.typography.bodyMedium,
            )
            if (order.phone.isNotBlank()) Text("SĐT: ${order.phone}")
            if (order.address.isNotBlank()) Text("Địa chỉ: ${order.address}")
            Text(
                order.channel.label,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            if (order.missing.isNotEmpty()) {
                Text(
                    "Còn thiếu: ${order.missing.joinToString(", ")}",
                    color = Warn,
                    style = MaterialTheme.typography.bodySmall,
                )
            }

            if (order.state == OrderState.DRAFT) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(
                        enabled = order.isComplete,
                        onClick = {
                            app.orders.update(order.id) {
                                it.copy(
                                    state = OrderState.CONFIRMED,
                                    followUpAt = System.currentTimeMillis() +
                                        FollowUpWorker.DEFAULT_DELAY_MS,
                                )
                            }
                            // Reserve stock only once a human has accepted the order.
                            order.productId?.let { app.products.adjustStock(it, -order.quantity) }
                        },
                        modifier = Modifier.weight(1f),
                    ) { Text("Xác nhận") }
                    OutlinedButton(
                        onClick = {
                            app.orders.update(order.id) { it.copy(state = OrderState.CANCELLED) }
                        },
                        modifier = Modifier.weight(1f),
                    ) { Text("Huỷ") }
                }
            }

            if (order.state == OrderState.CONFIRMED) {
                OutlinedButton(
                    onClick = { app.orders.update(order.id) { it.copy(state = OrderState.SHIPPED) } },
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Đã giao") }
            }
        }
    }
}

private fun stateColor(state: OrderState) = when (state) {
    OrderState.DRAFT -> Warn
    OrderState.CONFIRMED, OrderState.SHIPPED, OrderState.DONE -> Ok
    OrderState.CANCELLED -> Bad
}
