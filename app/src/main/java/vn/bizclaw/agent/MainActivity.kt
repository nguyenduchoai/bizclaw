package vn.bizclaw.agent

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Inbox
import androidx.compose.material.icons.filled.ReceiptLong
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.MenuBook
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import vn.bizclaw.agent.ui.BizClawTheme
import vn.bizclaw.agent.ui.HomeScreen
import vn.bizclaw.agent.ui.InboxScreen
import vn.bizclaw.agent.ui.KnowledgeScreen
import vn.bizclaw.agent.ui.OrdersScreen
import vn.bizclaw.agent.ui.ProviderScreen

private enum class Tab(val title: String, val icon: ImageVector) {
    Home("Trạng thái", Icons.Default.Speed),
    Inbox("Hộp thư", Icons.Default.Inbox),
    Orders("Đơn hàng", Icons.Default.ReceiptLong),
    Knowledge("Cửa hàng", Icons.Default.MenuBook),
    Model("Model", Icons.Default.Memory),
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            BizClawTheme { BizClawShell(BizClawApp.from(applicationContext)) }
        }
    }
}

@Composable
private fun BizClawShell(app: BizClawApp) {
    var tab by remember { mutableStateOf(Tab.Home) }
    val context = LocalContext.current

    // Without POST_NOTIFICATIONS the foreground service notification is silently
    // dropped on Android 13+, which makes the agent look dead.
    val permission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { }
    LaunchedEffect(Unit) {
        val granted = context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        if (!granted) permission.launch(Manifest.permission.POST_NOTIFICATIONS)
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        bottomBar = {
            NavigationBar {
                Tab.entries.forEach { item ->
                    NavigationBarItem(
                        selected = tab == item,
                        onClick = { tab = item },
                        icon = { Icon(item.icon, contentDescription = item.title) },
                        label = { Text(item.title, maxLines = 1) },
                    )
                }
            }
        },
    ) { padding ->
        Surface(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize(),
            color = MaterialTheme.colorScheme.background,
        ) {
            when (tab) {
                Tab.Home -> HomeScreen(app)
                Tab.Inbox -> InboxScreen(app)
                Tab.Orders -> OrdersScreen(app)
                Tab.Knowledge -> KnowledgeScreen(app)
                Tab.Model -> ProviderScreen(app)
            }
        }
    }
}
