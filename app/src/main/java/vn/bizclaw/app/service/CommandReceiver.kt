package vn.bizclaw.app.service

import android.content.Context
import android.os.Build
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.serialization.encodeToString
import okhttp3.*
import vn.bizclaw.app.engine.GlobalLLM
import java.util.concurrent.TimeUnit

/**
 * CommandReceiver — WebSocket client that connects to BizClaw backend.
 *
 * Lifecycle:
 * 1. DaemonService starts → CommandReceiver.connect()
 * 2. Sends DeviceInfo registration
 * 3. Receives DeviceCommand from backend
 * 4. Routes to CommandExecutor
 * 5. Sends CommandResult back
 * 6. Auto-reconnect on disconnect
 *
 * Protocol: WSS with JSON messages (WsMessage wrapper)
 */
class CommandReceiver(
    private val context: Context,
    private val serverUrl: String,
) {
    companion object {
        private const val TAG = "CommandRecv"
        private const val RECONNECT_DELAY_MS = 5_000L
        private const val MAX_RECONNECT_DELAY_MS = 60_000L
        private const val PING_INTERVAL_MS = 30_000L

        var instance: CommandReceiver? = null
            private set

        var isConnected: Boolean = false
            private set

        // Callback for UI to observe connection state
        var onConnectionChange: ((Boolean) -> Unit)? = null
        var onCommandReceived: ((DeviceCommand) -> Unit)? = null
        var onCommandResult: ((CommandResult) -> Unit)? = null
    }

    private val client = OkHttpClient.Builder()
        .pingInterval(PING_INTERVAL_MS, TimeUnit.MILLISECONDS)
        .readTimeout(0, TimeUnit.MILLISECONDS) // No timeout for WebSocket
        .build()

    private var webSocket: WebSocket? = null
    private var reconnectAttempt = 0
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var reconnectJob: Job? = null

    fun connect() {
        instance = this
        doConnect()
    }

    fun disconnect() {
        reconnectJob?.cancel()
        webSocket?.close(1000, "Client disconnect")
        webSocket = null
        isConnected = false
        instance = null
        onConnectionChange?.invoke(false)
        Log.i(TAG, "🔌 Disconnected from command server")
    }

    fun sendResult(result: CommandResult) {
        val msg = WsMessage(
            type = "result",
            payload = commandJson.encodeToString(result),
        )
        val json = commandJson.encodeToString(msg)
        val sent = webSocket?.send(json) ?: false
        if (!sent) {
            Log.w(TAG, "Failed to send result for command ${result.id}")
        }
    }

    private fun doConnect() {
        val request = Request.Builder()
            .url(serverUrl)
            .build()

        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.i(TAG, "🟢 Connected to command server: $serverUrl")
                isConnected = true
                reconnectAttempt = 0
                onConnectionChange?.invoke(true)

                // Send device registration
                sendRegistration(webSocket)
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                handleMessage(text)
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "🟡 Server closing: $code $reason")
                webSocket.close(1000, null)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "🔴 Disconnected: $code $reason")
                isConnected = false
                onConnectionChange?.invoke(false)
                scheduleReconnect()
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e(TAG, "🔴 Connection failed: ${t.message?.take(100)}")
                isConnected = false
                onConnectionChange?.invoke(false)
                scheduleReconnect()
            }
        })
    }

    private fun sendRegistration(ws: WebSocket) {
        val deviceInfo = DeviceRegistration(
            deviceId = getDeviceId(),
            deviceName = "${Build.MANUFACTURER} ${Build.MODEL}",
            androidVersion = Build.VERSION.SDK_INT,
            appVersion = try {
                context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "?"
            } catch (_: Exception) { "?" },
            modelLoaded = GlobalLLM.loadedModelName,
            accessibilityEnabled = BizClawAccessibilityService.isRunning(),
            notificationListenerEnabled = BizClawNotificationListener.instance != null,
            batteryPercent = CommandExecutor.run {
                // Quick battery check
                try {
                    val bm = context.getSystemService(Context.BATTERY_SERVICE) as android.os.BatteryManager
                    bm.getIntProperty(android.os.BatteryManager.BATTERY_PROPERTY_CAPACITY)
                } catch (_: Exception) { -1 }
            },
        )

        val msg = WsMessage(
            type = "register",
            payload = commandJson.encodeToString(deviceInfo),
        )
        ws.send(commandJson.encodeToString(msg))
        Log.i(TAG, "📱 Registered: ${deviceInfo.deviceName}")
    }

    private fun handleMessage(text: String) {
        try {
            val msg = commandJson.decodeFromString<WsMessage>(text)
            when (msg.type) {
                "command" -> {
                    val cmd = commandJson.decodeFromString<DeviceCommand>(msg.payload)
                    Log.i(TAG, "📥 Command: ${cmd.type}/${cmd.action} (${cmd.id})")
                    onCommandReceived?.invoke(cmd)

                    // Execute async
                    scope.launch {
                        val result = withTimeoutOrNull(cmd.timeoutMs) {
                            CommandExecutor.execute(context, cmd)
                        } ?: CommandResult(
                            id = cmd.id,
                            status = CommandStatus.timeout,
                            error = "Timeout after ${cmd.timeoutMs}ms",
                        )

                        Log.i(TAG, "📤 Result: ${result.status} (${result.durationMs}ms)")
                        onCommandResult?.invoke(result)
                        sendResult(result)
                    }
                }
                "ping" -> {
                    val pong = WsMessage(type = "pong", payload = "")
                    webSocket?.send(commandJson.encodeToString(pong))
                }
                else -> Log.d(TAG, "Unknown message type: ${msg.type}")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse message: ${e.message?.take(100)}")
        }
    }

    private fun scheduleReconnect() {
        reconnectJob?.cancel()
        reconnectJob = scope.launch {
            val delay = minOf(
                RECONNECT_DELAY_MS * (1L shl minOf(reconnectAttempt, 5)),
                MAX_RECONNECT_DELAY_MS,
            )
            reconnectAttempt++
            Log.i(TAG, "⏳ Reconnecting in ${delay}ms (attempt $reconnectAttempt)")
            delay(delay)
            doConnect()
        }
    }

    private fun getDeviceId(): String {
        val prefs = context.getSharedPreferences("bizclaw", Context.MODE_PRIVATE)
        var id = prefs.getString("device_id", null)
        if (id == null) {
            id = "device_${java.util.UUID.randomUUID().toString().take(8)}"
            prefs.edit().putString("device_id", id).apply()
        }
        return id
    }
}
