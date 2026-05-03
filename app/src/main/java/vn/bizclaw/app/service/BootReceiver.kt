package vn.bizclaw.app.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * Boot Receiver — auto-start BizClaw daemon after phone reboot.
 *
 * Ensures agents keep running 24/7, surviving reboots.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            // Check if user had daemon enabled before reboot
            val prefs = context.getSharedPreferences("bizclaw", Context.MODE_PRIVATE)
            val autoStart = prefs.getBoolean("auto_start_on_boot", false)

            if (autoStart) {
                BizClawDaemonService.start(context)
                android.util.Log.i("BizClaw", "🔄 Auto-started daemon after boot")
            }
        }
    }
}
