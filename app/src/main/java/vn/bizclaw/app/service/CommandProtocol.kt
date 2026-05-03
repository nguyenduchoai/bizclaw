package vn.bizclaw.app.service

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/**
 * Command Protocol — defines the contract between external controllers and device.
 *
 * Flow: Dashboard/API → Backend → WebSocket → CommandReceiver → CommandExecutor
 *
 * Supported command types:
 * - chat: Send message to AI agent
 * - ui_action: Control screen via AccessibilityService
 * - app: Launch apps, press back/home
 * - device: Status, battery, screenshot
 * - social_reply: Reply to a social app message
 * - automation: High-level app workflows (Gmail, Facebook, Instagram)
 * - flow: Multi-step automation
 */

val commandJson = Json {
    ignoreUnknownKeys = true
    isLenient = true
    encodeDefaults = true
}

@Serializable
data class DeviceCommand(
    val id: String,
    val type: CommandType,
    val action: String,
    val params: Map<String, String> = emptyMap(),
    val timeoutMs: Long = 10_000,
    val sentAt: Long = System.currentTimeMillis(),
)

@Serializable
enum class CommandType {
    chat,
    ui_action,
    app,
    device,
    social_reply,
    automation,
    schedule,
    mama,
    flow,
    voice,
}

@Serializable
data class CommandResult(
    val id: String,
    val status: CommandStatus,
    val result: Map<String, String> = emptyMap(),
    val error: String? = null,
    val executedAt: Long = System.currentTimeMillis(),
    val durationMs: Long = 0,
)

@Serializable
enum class CommandStatus {
    success,
    failed,
    timeout,
    unsupported,
}

/** Device registration info sent on WebSocket connect */
@Serializable
data class DeviceRegistration(
    val deviceId: String,
    val deviceName: String,
    val androidVersion: Int,
    val appVersion: String,
    val modelLoaded: String? = null,
    val accessibilityEnabled: Boolean = false,
    val notificationListenerEnabled: Boolean = false,
    val batteryPercent: Int = -1,
)

/** WebSocket message wrapper */
@Serializable
data class WsMessage(
    val type: String,  // "command", "result", "register", "ping", "pong"
    val payload: String, // JSON string of DeviceCommand / CommandResult / DeviceInfo
)
