package vn.bizclaw.app.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

// ─── BizClaw Brand Colors ────────────────────────────────────────────

val BizClawPrimary = Color(0xFF6C5CE7)      // Purple — BizClaw brand
val BizClawSecondary = Color(0xFF00D2D3)     // Teal — accent
val BizClawTertiary = Color(0xFFFF6B6B)      // Coral — alerts
val BizClawSurface = Color(0xFF1A1A2E)       // Deep navy
val BizClawOnSurface = Color(0xFFE8E8EE)     // Light text
val BizClawBackground = Color(0xFF0F0F23)    // Darkest navy

// ─── Dark Color Scheme (Default) ─────────────────────────────────────

private val DarkColorScheme = darkColorScheme(
    primary = BizClawPrimary,
    secondary = BizClawSecondary,
    tertiary = BizClawTertiary,
    background = BizClawBackground,
    surface = BizClawSurface,
    onPrimary = Color.White,
    onSecondary = Color.White,
    onTertiary = Color.White,
    onBackground = BizClawOnSurface,
    onSurface = BizClawOnSurface,
    surfaceVariant = Color(0xFF2A2A4A),
    onSurfaceVariant = Color(0xFFB8B8CC),
)

private val LightColorScheme = lightColorScheme(
    primary = BizClawPrimary,
    secondary = BizClawSecondary,
    tertiary = BizClawTertiary,
)

@Composable
fun BizClawTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit,
) {
    val colorScheme = when {
        // Material You dynamic colors on Android 12+
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = BizClawTypography,
        content = content,
    )
}
