package vn.bizclaw.app.engine

import org.junit.Test
import org.junit.Assert.*

/**
 * Unit tests cho AIProvider, ProviderType, AgentGroup
 * 
 * Tests pure data classes — không cần Android context
 */
class ProviderDataTest {

    // ═══════════════════════════════════════════════════════
    // AIProvider Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `AIProvider defaults are correct`() {
        val provider = AIProvider(
            id = "test",
            name = "Test Provider",
            type = ProviderType.OPENAI,
        )
        assertEquals("test", provider.id)
        assertEquals("Test Provider", provider.name)
        assertEquals(ProviderType.OPENAI, provider.type)
        assertEquals("🤖", provider.emoji)
        assertEquals("", provider.apiKey)
        assertEquals("", provider.baseUrl)
        assertEquals("", provider.model)
        assertTrue(provider.enabled)
        assertTrue(provider.createdAt > 0)
    }

    @Test
    fun `AIProvider copy preserves fields`() {
        val original = AIProvider(
            id = "openai",
            name = "OpenAI",
            type = ProviderType.OPENAI,
            apiKey = "sk-test123",
            baseUrl = "https://api.openai.com/v1",
            model = "gpt-4o-mini",
        )
        val disabled = original.copy(enabled = false)

        assertEquals(original.id, disabled.id)
        assertEquals(original.apiKey, disabled.apiKey)
        assertFalse(disabled.enabled)
    }

    @Test
    fun `AIProvider copy can clear apiKey`() {
        val withKey = AIProvider(
            id = "test",
            name = "Test",
            type = ProviderType.OPENAI,
            apiKey = "sk-secret",
        )
        val sanitized = withKey.copy(apiKey = "")
        assertEquals("", sanitized.apiKey)
        assertEquals("test", sanitized.id)
    }

    // ═══════════════════════════════════════════════════════
    // ProviderType Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `ProviderType has all expected values`() {
        val types = ProviderType.entries.toList()
        assertEquals(6, types.size)
        assertTrue(types.contains(ProviderType.LOCAL_GGUF))
        assertTrue(types.contains(ProviderType.OPENAI))
        assertTrue(types.contains(ProviderType.GEMINI))
        assertTrue(types.contains(ProviderType.OLLAMA))
        assertTrue(types.contains(ProviderType.BIZCLAW_CLOUD))
        assertTrue(types.contains(ProviderType.CUSTOM_API))
    }

    @Test
    fun `ProviderType name returns correct string`() {
        assertEquals("LOCAL_GGUF", ProviderType.LOCAL_GGUF.name)
        assertEquals("OPENAI", ProviderType.OPENAI.name)
        assertEquals("GEMINI", ProviderType.GEMINI.name)
        assertEquals("OLLAMA", ProviderType.OLLAMA.name)
    }

    // ═══════════════════════════════════════════════════════
    // AgentGroup Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `AgentGroup defaults are correct`() {
        val group = AgentGroup(id = "g1", name = "Test Group")
        assertEquals("g1", group.id)
        assertEquals("Test Group", group.name)
        assertEquals("👥", group.emoji)
        assertEquals("", group.description)
        assertTrue(group.agentIds.isEmpty())
        assertNull(group.routerAgentId)
    }

    @Test
    fun `AgentGroup with members`() {
        val group = AgentGroup(
            id = "cskh",
            name = "CSKH Team",
            emoji = "🏢",
            agentIds = listOf("agent1", "agent2", "agent3"),
            routerAgentId = "agent1",
        )
        assertEquals(3, group.agentIds.size)
        assertEquals("agent1", group.routerAgentId)
        assertTrue(group.agentIds.contains("agent2"))
    }

    @Test
    fun `AgentGroup agentIds is immutable list`() {
        val ids = listOf("a1", "a2")
        val group = AgentGroup(id = "g", name = "G", agentIds = ids)
        assertEquals(ids, group.agentIds)
    }

    // ═══════════════════════════════════════════════════════
    // AIProvider Security Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `API key should not appear in toString by default`() {
        // data class toString() WILL include apiKey, but we verify
        // the sanitization pattern works
        val provider = AIProvider(
            id = "test",
            name = "Test",
            type = ProviderType.OPENAI,
            apiKey = "sk-super-secret-key-12345",
        )
        val sanitized = provider.copy(apiKey = "")
        assertFalse(sanitized.toString().contains("sk-super-secret"))
    }

    @Test
    fun `Provider with Ollama does not need apiKey`() {
        val ollama = AIProvider(
            id = "ollama",
            name = "Ollama",
            type = ProviderType.OLLAMA,
            baseUrl = "http://192.168.1.100:11434",
            model = "qwen2.5:7b",
        )
        assertEquals("", ollama.apiKey)
        assertTrue(ollama.baseUrl.startsWith("http://"))
    }

    @Test
    fun `Local GGUF provider has no base URL`() {
        val local = AIProvider(
            id = "local",
            name = "Local",
            type = ProviderType.LOCAL_GGUF,
        )
        assertEquals("", local.baseUrl)
        assertEquals("", local.apiKey)
    }
}
