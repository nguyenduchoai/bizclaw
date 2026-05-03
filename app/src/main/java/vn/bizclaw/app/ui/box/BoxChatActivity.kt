package vn.bizclaw.app.ui.box

import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import kotlinx.coroutines.launch
import vn.bizclaw.app.R
import vn.bizclaw.app.box.BoxConfig
import vn.bizclaw.app.box.BoxEngine
import vn.bizclaw.app.box.ChatResponse

/**
 * Box Chat Activity - Chat với AI offline
 */
class BoxChatActivity : AppCompatActivity() {
    
    private lateinit var boxEngine: BoxEngine
    private lateinit var messageInput: EditText
    private lateinit var sendButton: ImageButton
    private lateinit var messagesList: RecyclerView
    private lateinit var modelStatus: TextView
    private lateinit var offlineIndicator: ImageView
    
    private val messages = mutableListOf<ChatMessage>()
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_box_chat)
        
        initViews()
        initBoxEngine()
    }
    
    private fun initViews() {
        messageInput = findViewById(R.id.messageInput)
        sendButton = findViewById(R.id.sendButton)
        messagesList = findViewById(R.id.messagesList)
        modelStatus = findViewById(R.id.modelStatus)
        offlineIndicator = findViewById(R.id.offlineIndicator)
        
        sendButton.setOnClickListener { sendMessage() }
        
        offlineIndicator.setColorFilter(getColor(R.color.green))
    }
    
    private fun initBoxEngine() {
        boxEngine = BoxEngine(this)
        
        lifecycleScope.launch {
            val config = BoxConfig(
                modelPath = getExternalFilesDir("models")?.absolutePath + "/hermes-2-pro-q4.gguf",
                quantization = "Q4_K_M",
                visionEnabled = true,
                imageEnabled = true
            )
            
            boxEngine.initialize(config).onSuccess {
                runOnUiThread {
                    modelStatus.text = "Model loaded"
                    modelStatus.setTextColor(getColor(R.color.green))
                }
            }.onFailure { e ->
                runOnUiThread {
                    modelStatus.text = "Model error: ${e.message}"
                    modelStatus.setTextColor(getColor(R.color.red))
                }
            }
        }
    }
    
    private fun sendMessage() {
        val message = messageInput.text.toString().trim()
        if (message.isEmpty()) return
        
        addMessage(message, isUser = true)
        messageInput.text.clear()
        
        lifecycleScope.launch {
            val response = boxEngine.chat(message)
            
            runOnUiThread {
                response.onSuccess { chatResponse ->
                    addMessage(chatResponse.content, isUser = false)
                }.onFailure { e ->
                    addMessage("Error: ${e.message}", isUser = false)
                }
            }
        }
    }
    
    private fun addMessage(text: String, isUser: Boolean) {
        messages.add(ChatMessage(text, isUser))
        // Update RecyclerView
        // messagesAdapter.notifyItemInserted(messages.size - 1)
        // messagesList.scrollToPosition(messages.size - 1)
    }
    
    override fun onDestroy() {
        super.onDestroy()
        boxEngine.release()
    }
}

data class ChatMessage(
    val text: String,
    val isUser: Boolean
)
