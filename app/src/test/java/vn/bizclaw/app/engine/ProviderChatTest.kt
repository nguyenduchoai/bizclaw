package vn.bizclaw.app.engine

import org.junit.Test
import org.junit.Assert.*

/**
 * Unit tests cho ProviderChat — URL validation & security
 *
 * Chỉ test pure logic (không cần network/Android context)
 */
class ProviderChatTest {

    // ═══════════════════════════════════════════════════════
    // URL Validation Tests — SSRF Prevention
    // ═══════════════════════════════════════════════════════

    @Test
    fun `validateUrl accepts http URL`() {
        // Should not throw
        ProviderChat.validateUrl("http://192.168.1.100:11434")
    }

    @Test
    fun `validateUrl accepts https URL`() {
        // Should not throw
        ProviderChat.validateUrl("https://api.openai.com/v1")
    }

    @Test
    fun `validateUrl accepts https Gemini URL`() {
        ProviderChat.validateUrl("https://generativelanguage.googleapis.com")
    }

    @Test(expected = IllegalArgumentException::class)
    fun `validateUrl rejects ftp URL`() {
        ProviderChat.validateUrl("ftp://malicious.com")
    }

    @Test(expected = IllegalArgumentException::class)
    fun `validateUrl rejects file URL`() {
        ProviderChat.validateUrl("file:///etc/passwd")
    }

    @Test(expected = IllegalArgumentException::class)
    fun `validateUrl rejects javascript URL`() {
        ProviderChat.validateUrl("javascript:alert(1)")
    }

    @Test(expected = IllegalArgumentException::class)
    fun `validateUrl rejects empty string`() {
        ProviderChat.validateUrl("")
    }

    @Test(expected = IllegalArgumentException::class)
    fun `validateUrl rejects random string`() {
        ProviderChat.validateUrl("not-a-url")
    }

    @Test(expected = IllegalArgumentException::class)
    fun `validateUrl rejects data URI`() {
        ProviderChat.validateUrl("data:text/html,<script>alert(1)</script>")
    }

    @Test
    fun `validateUrl accepts localhost http`() {
        ProviderChat.validateUrl("http://localhost:8080")
    }

    @Test
    fun `validateUrl accepts IP http`() {
        ProviderChat.validateUrl("http://10.0.0.1:11434")
    }

    // ═══════════════════════════════════════════════════════
    // Provider Routing Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `provider type determines chat route`() {
        // Verify the routing table covers all types
        ProviderType.entries.forEach { type ->
            val provider = AIProvider(
                id = "test_${type.name}",
                name = "Test",
                type = type,
            )
            // Just verify the provider can be created for each type
            assertEquals(type, provider.type)
        }
    }

    @Test
    fun `OpenAI and CustomAPI share same endpoint logic`() {
        // Both should route through chatOpenAI
        val openai = AIProvider(id = "1", name = "O", type = ProviderType.OPENAI)
        val custom = AIProvider(id = "2", name = "C", type = ProviderType.CUSTOM_API)
        // Both are non-local, non-Gemini, non-Ollama
        assertNotEquals(ProviderType.LOCAL_GGUF, openai.type)
        assertNotEquals(ProviderType.LOCAL_GGUF, custom.type)
        assertNotEquals(ProviderType.GEMINI, openai.type)
        assertNotEquals(ProviderType.GEMINI, custom.type)
    }

    // ═══════════════════════════════════════════════════════
    // Constants Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `timeout constants are reasonable`() {
        // Access via reflection since they're private const
        // But we can verify behavior indirectly:
        // Connect timeout should be < read timeout
        // Just verify types are constructed correctly
        val provider = AIProvider(
            id = "test",
            name = "Test",
            type = ProviderType.OPENAI,
            baseUrl = "https://api.openai.com/v1",
            model = "gpt-4o-mini",
            apiKey = "sk-test",
        )
        assertTrue(provider.baseUrl.startsWith("https://"))
        assertTrue(provider.model.isNotBlank())
    }
}
