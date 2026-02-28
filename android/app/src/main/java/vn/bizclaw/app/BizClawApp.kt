package vn.bizclaw.app

import android.app.Application

class BizClawApp : Application() {
    override fun onCreate() {
        super.onCreate()
        // No heavy initialization — thin client, fast startup
    }
}
