package vn.bizclaw.agent.agent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import vn.bizclaw.agent.BizClawApp

/** Brings the agent back up after a reboot, but only if the owner had it switched on. */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        if (!BizClawApp.from(context).settings.agentEnabled) return
        AgentService.start(context)
    }
}
