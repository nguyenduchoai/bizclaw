package vn.bizclaw.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.*
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import vn.bizclaw.app.ui.agents.AgentsScreen
import vn.bizclaw.app.ui.chat.ChatScreen
import vn.bizclaw.app.ui.chat.ChatViewModel
import vn.bizclaw.app.ui.settings.SettingsScreen
import vn.bizclaw.app.ui.theme.BizClawTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            BizClawTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    BizClawApp()
                }
            }
        }
    }
}

enum class Screen {
    Chat, Agents, Settings
}

@Composable
fun BizClawApp() {
    val chatViewModel: ChatViewModel = viewModel()
    var currentScreen by remember { mutableStateOf(Screen.Chat) }

    // Server config from SharedPreferences (simplified)
    var serverUrl by remember { mutableStateOf("http://localhost:3001") }
    var apiKey by remember { mutableStateOf("") }

    // Initialize connection
    LaunchedEffect(Unit) {
        chatViewModel.updateServer(serverUrl, apiKey)
    }

    when (currentScreen) {
        Screen.Chat -> {
            ChatScreen(
                viewModel = chatViewModel,
                onOpenAgents = { currentScreen = Screen.Agents },
                onOpenSettings = { currentScreen = Screen.Settings },
            )
        }

        Screen.Agents -> {
            AgentsScreen(
                agents = chatViewModel.agents,
                currentAgent = chatViewModel.currentAgent.value,
                onSelectAgent = { chatViewModel.selectAgent(it) },
                onBack = { currentScreen = Screen.Chat },
            )
        }

        Screen.Settings -> {
            SettingsScreen(
                serverUrl = serverUrl,
                apiKey = apiKey,
                isConnected = chatViewModel.isConnected.value,
                onUpdateServer = { url, key ->
                    serverUrl = url
                    apiKey = key
                    chatViewModel.updateServer(url, key)
                },
                onBack = { currentScreen = Screen.Chat },
            )
        }
    }
}
