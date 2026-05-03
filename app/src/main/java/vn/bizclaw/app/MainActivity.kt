package vn.bizclaw.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import vn.bizclaw.app.ui.agents.AgentsScreen
import vn.bizclaw.app.ui.agents.KnowledgeBaseScreen
import vn.bizclaw.app.ui.chat.ChatScreen
import vn.bizclaw.app.ui.chat.ChatViewModel
import vn.bizclaw.app.ui.dashboard.DashboardScreen
import vn.bizclaw.app.engine.GlobalLLM
import vn.bizclaw.app.engine.LocalAgent
import vn.bizclaw.app.engine.ProviderChat
import vn.bizclaw.app.ui.localllm.LocalLLMScreen
import vn.bizclaw.app.ui.automation.AutomationScreen
import vn.bizclaw.app.ui.meeting.MeetingScreen
import vn.bizclaw.app.ui.settings.SettingsScreen
import vn.bizclaw.app.ui.settings.ProviderScreen
import vn.bizclaw.app.ui.theme.BizClawTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Set app context for app-based AI providers (Gemini/ChatGPT/Grok)
        ProviderChat.appContext = applicationContext

        setContent {
            BizClawTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    BizClawNavHost()
                }
            }
        }
    }
}

enum class Screen {
    Chat, Agents, Settings, Dashboard, LocalLLM, KnowledgeBase, Automation, Providers, Meeting
}

@Composable
fun BizClawNavHost() {
    val context = LocalContext.current
    val chatViewModel: ChatViewModel = viewModel()

    // ─── Navigation with back stack ───
    // Keep a history so back gesture pops to previous screen
    val navStack = remember { mutableStateListOf(Screen.Chat) }
    val currentScreen = navStack.last()

    fun navigateTo(screen: Screen) {
        if (screen != currentScreen) {
            navStack.add(screen)
        }
    }

    fun goBack(): Boolean {
        return if (navStack.size > 1) {
            navStack.removeAt(navStack.lastIndex)
            true
        } else {
            false // Already at root — let system handle (exit app)
        }
    }

    // ─── System back handler ───
    // When user swipes back or presses back button:
    // - If on a sub-screen → go to previous screen
    // - If on Chat (home) → default behavior (exit or minimize)
    BackHandler(enabled = navStack.size > 1) {
        goBack()
    }

    // Server config
    var serverUrl by remember { mutableStateOf("http://127.0.0.1:3001") }
    var apiKey by remember { mutableStateOf("") }

    // Initialize — check local models first, only check server if no local model
    LaunchedEffect(Unit) {
        chatViewModel.refreshLocalModels(context)
        // Only check server if local model not available
        if (!GlobalLLM.instance.isLoaded) {
            chatViewModel.updateServer(serverUrl, apiKey)
        } else {
            // Sync GlobalLLM state to chat view model
            chatViewModel.checkConnection()
        }
    }

    when (currentScreen) {
        Screen.Chat -> {
            ChatScreen(
                viewModel = chatViewModel,
                onOpenAgents = { navigateTo(Screen.Agents) },
                onOpenSettings = { navigateTo(Screen.Settings) },
                onOpenAutomation = { navigateTo(Screen.Automation) },
                onOpenLocalLLM = { navigateTo(Screen.LocalLLM) },
                onOpenMeeting = { navigateTo(Screen.Meeting) },
            )
        }

        Screen.Agents -> {
            AgentsScreen(
                onSelectAgent = { agent ->
                    // When user selects an agent, go to LocalLLM chat with that prompt
                    if (GlobalLLM.instance.isLoaded) {
                        GlobalLLM.instance.addSystemPrompt(agent.systemPrompt)
                    }
                    navigateTo(Screen.LocalLLM)
                },
                onOpenKB = { navigateTo(Screen.KnowledgeBase) },
                onBack = { goBack() },
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
                onOpenProviders = { navigateTo(Screen.Providers) },
                onBack = { goBack() },
            )
        }

        Screen.Dashboard -> {
            DashboardScreen(
                onBack = { goBack() },
            )
        }

        Screen.LocalLLM -> {
            LocalLLMScreen(
                onBack = {
                    // Refresh local models when returning
                    chatViewModel.refreshLocalModels(context)
                    goBack()
                },
            )
        }

        Screen.KnowledgeBase -> {
            KnowledgeBaseScreen(
                onBack = { goBack() },
            )
        }

        Screen.Automation -> {
            AutomationScreen(
                onBack = { goBack() },
            )
        }

        Screen.Providers -> {
            ProviderScreen(
                onBack = { goBack() },
            )
        }

        Screen.Meeting -> {
            MeetingScreen(
                onBack = { goBack() },
            )
        }
    }
}
