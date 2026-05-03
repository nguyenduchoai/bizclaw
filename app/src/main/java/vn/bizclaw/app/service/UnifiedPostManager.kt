package vn.bizclaw.app.service

import android.content.Context
import android.util.Log

/**
 * Unified Post Manager - Simplified version.
 */
class UnifiedPostManager(private val context: Context) {

    companion object {
        private const val TAG = "UnifiedPostManager"
    }

    enum class Platform {
        FACEBOOK, ZALO_OA, INSTAGRAM, LINKEDIN
    }

    data class PostResult(
        val platform: String,
        val success: Boolean,
        val postId: String?,
    )

    fun configurePlatform(platform: Platform, accessToken: String) {
        Log.w(TAG, "Configure platform: $platform")
    }

    suspend fun post(content: String, platforms: List<Platform>): List<PostResult> {
        Log.w(TAG, "Post placeholder - configure API keys first")
        return platforms.map { 
            PostResult(platform = it.name, success = false, postId = null)
        }
    }
}
