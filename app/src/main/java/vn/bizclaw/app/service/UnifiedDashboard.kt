package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * Unified Dashboard - Simplified version.
 */
class UnifiedDashboard(private val context: Context) {

    companion object {
        private const val TAG = "UnifiedDashboard"
    }

    data class DashboardData(
        val totalUnread: Int = 0,
        val totalPending: Int = 0,
        val healthScore: Float = 100f,
        val lastSync: Long = 0,
    )

    private val _dashboardData = MutableStateFlow(DashboardData())
    val dashboardData: StateFlow<DashboardData> = _dashboardData

    fun initialize() {
        Log.w(TAG, "Dashboard initialized")
    }

    suspend fun refresh() {
        _dashboardData.value = DashboardData(
            totalUnread = 0,
            totalPending = 0,
            healthScore = 100f,
            lastSync = System.currentTimeMillis(),
        )
    }

    fun formatAsDailyReport(): String {
        val data = _dashboardData.value
        return buildString {
            appendLine("📊 BIZCLAW DAILY REPORT")
            appendLine()
            appendLine("Health Score: ${data.healthScore.toInt()}%")
            appendLine("Unread: ${data.totalUnread}")
            appendLine("Pending: ${data.totalPending}")
            appendLine()
            appendLine("⚠️ Cần cấu hình các service để thu thập dữ liệu")
        }
    }
}
