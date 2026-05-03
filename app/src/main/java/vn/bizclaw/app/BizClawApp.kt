package vn.bizclaw.app

import android.app.Application
import vn.bizclaw.app.engine.GlobalLLM
import vn.bizclaw.app.engine.ProviderChat

class BizClawApp : Application() {
    override fun onCreate() {
        super.onCreate()
        // Set app context for provider management and vision fallback
        GlobalLLM.appContext = applicationContext
        ProviderChat.appContext = applicationContext
    }
}
