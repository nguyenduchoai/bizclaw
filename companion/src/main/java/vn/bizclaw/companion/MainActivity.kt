package vn.bizclaw.companion

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : Activity() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private lateinit var serverInput: EditText
    private lateinit var tokenInput: EditText
    private lateinit var requestInput: EditText
    private lateinit var output: TextView
    private lateinit var ticketList: TextView
    private lateinit var modeStatus: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        buildLayout()
        hydrateFromPrefs()
        handleSharedText(intent)
        refreshTickets()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleSharedText(intent)
    }

    override fun onPause() {
        persistSettings()
        super.onPause()
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private fun buildLayout() {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
            setBackgroundColor(Color.rgb(17, 24, 39))
        }

        root.addView(title("BizClaw Companion"))
        root.addView(body("Android chạy được độc lập bằng Local Mama, hoặc kết nối Desktop để đẩy việc vào Mama tổng quản. Emulator dùng 10.0.2.2:3001; máy thật dùng LAN URL từ Desktop."))

        modeStatus = body("")
        root.addView(row(
            action("Độc lập") { setMode(CompanionPrefs.MODE_LOCAL) },
            action("Kết nối Desktop") { setMode(CompanionPrefs.MODE_DESKTOP) },
        ))
        root.addView(modeStatus)

        serverInput = field(BuildConfig.DEFAULT_SERVER_URL, "Desktop/Gateway URL")
        tokenInput = field("", "API key / Bearer token")
        requestInput = field("Khách hỏi phí ship và bảo hành, hãy tạo ticket và soạn trả lời an toàn.", "Yêu cầu cho Mama").apply {
            minLines = 4
            gravity = Gravity.TOP
            imeOptions = EditorInfo.IME_ACTION_SEND
        }

        root.addView(serverInput)
        root.addView(tokenInput)
        root.addView(requestInput)
        root.addView(row(
            action("Local Mama") { runLocalMama() },
            action("Gửi Desktop") { sendDesktopMama() },
        ))
        root.addView(row(
            action("Test Health") { testHealth() },
            action("Sync Ticket") { syncLatestTicket() },
        ))
        root.addView(row(
            action("Notification") { startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)) },
            action("Accessibility") { startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)) },
        ))
        root.addView(action("Battery Settings") {
            startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS).apply {
                data = Uri.parse("package:$packageName")
            })
        })
        root.addView(action("Clear Local Tickets") {
            LocalMama.clearTickets(this)
            refreshTickets()
            output.text = "Đã xoá queue local."
        })

        output = body("Chưa xử lý.")
        ticketList = body("")
        root.addView(output)
        root.addView(title("Local Queue"))
        root.addView(ticketList)

        setContentView(ScrollView(this).apply { addView(root) })
    }

    private fun hydrateFromPrefs() {
        val prefs = CompanionPrefs.open(this)
        serverInput.setText(prefs.getString(CompanionPrefs.KEY_SERVER_URL, BuildConfig.DEFAULT_SERVER_URL))
        tokenInput.setText(prefs.getString(CompanionPrefs.KEY_TOKEN, ""))
        updateModeStatus()
    }

    private fun persistSettings() {
        CompanionPrefs.open(this).edit()
            .putString(CompanionPrefs.KEY_SERVER_URL, server())
            .putString(CompanionPrefs.KEY_TOKEN, tokenInput.text.toString().trim())
            .apply()
    }

    private fun setMode(mode: String) {
        CompanionPrefs.open(this).edit().putString(CompanionPrefs.KEY_MODE, mode).apply()
        updateModeStatus()
    }

    private fun updateModeStatus() {
        val mode = CompanionPrefs.mode(this)
        modeStatus.text = if (mode == CompanionPrefs.MODE_DESKTOP) {
            "Mode: Kết nối Desktop. Notification/share sẽ gửi về Gateway nếu online; lỗi mạng sẽ fallback vào queue local."
        } else {
            "Mode: Độc lập. Android tự tạo ticket local, không cần Desktop hay API."
        }
    }

    private fun handleSharedText(intent: Intent?) {
        if (intent?.action != Intent.ACTION_SEND || intent.type?.startsWith("text/") != true) return
        val sharedText = intent.getStringExtra(Intent.EXTRA_TEXT).orEmpty().trim()
        if (sharedText.isNotEmpty()) {
            requestInput.setText(sharedText)
            output.text = "Đã nhận nội dung share. Chọn Local Mama hoặc Gửi Desktop."
        }
    }

    private fun runLocalMama() {
        val ticket = LocalMama.route(requestInput.text.toString(), "manual_android")
        LocalMama.appendTicket(this, ticket)
        output.text = ticket.reply
        refreshTickets()
    }

    private fun sendDesktopMama() {
        persistSettings()
        runWork {
            LocalMama.postToDesktop(
                context = this,
                request = requestInput.text.toString(),
                source = "android_companion",
                customerRef = android.os.Build.MODEL ?: "android-device",
            )
        }
    }

    private fun testHealth() {
        persistSettings()
        runWork {
            val code = withContext(Dispatchers.IO) { LocalMama.httpGet("${server()}/health") }
            "Desktop/Gateway health: HTTP $code"
        }
    }

    private fun syncLatestTicket() {
        persistSettings()
        val ticket = LocalMama.latestTicket(this)
        if (ticket == null) {
            output.text = "Không có local ticket để sync."
            return
        }
        runWork {
            LocalMama.postToDesktop(
                context = this,
                request = "[Sync local ticket ${ticket.id}] ${ticket.request}\n\nLocal reply:\n${ticket.reply}",
                source = "android_companion_local_sync",
                customerRef = android.os.Build.MODEL ?: "android-device",
            )
        }
    }

    private fun runWork(work: suspend () -> String) {
        output.text = "Đang xử lý..."
        scope.launch {
            output.text = runCatching { work() }
                .getOrElse { error ->
                    val ticket = LocalMama.route(requestInput.text.toString(), "desktop_fallback")
                    LocalMama.appendTicket(this@MainActivity, ticket)
                    refreshTickets()
                    "Không gửi được Desktop, đã lưu local ticket ${ticket.id}.\nLỗi: ${error.message}"
                }
        }
    }

    private fun refreshTickets() {
        ticketList.text = LocalMama.ticketSummary(this)
    }

    private fun server(): String = serverInput.text.toString().trim().trimEnd('/')

    private fun title(text: String) = TextView(this).apply {
        this.text = text
        textSize = 24f
        setTextColor(Color.WHITE)
        setPadding(0, 8, 0, 14)
        setTypeface(typeface, android.graphics.Typeface.BOLD)
    }

    private fun body(text: String) = TextView(this).apply {
        this.text = text
        textSize = 15f
        setTextColor(Color.rgb(229, 231, 235))
        setPadding(0, 0, 0, 18)
    }

    private fun field(value: String, hint: String) = EditText(this).apply {
        setText(value)
        this.hint = hint
        setHintTextColor(Color.rgb(156, 163, 175))
        setTextColor(Color.WHITE)
        setSingleLine(false)
        setPadding(20, 16, 20, 16)
        setBackgroundColor(Color.rgb(31, 41, 55))
    }

    private fun action(text: String, onClick: () -> Unit) = Button(this).apply {
        this.text = text
        setTextColor(Color.WHITE)
        setBackgroundColor(Color.rgb(233, 69, 96))
        setOnClickListener { onClick() }
    }

    private fun row(vararg children: Button) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        children.forEach { child ->
            addView(child, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                setMargins(0, 8, 8, 8)
            })
        }
    }
}
