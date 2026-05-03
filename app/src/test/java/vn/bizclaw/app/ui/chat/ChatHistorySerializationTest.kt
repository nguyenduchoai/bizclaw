package vn.bizclaw.app.ui.chat

import org.junit.Test
import org.junit.Assert.*
import org.junit.Before
import org.junit.After
import kotlinx.serialization.json.*
import java.io.File

/**
 * Unit tests cho Chat History serialization
 *
 * Tests the JSON format used to persist chat history to disk.
 * Uses temp files — no Android context needed.
 */
class ChatHistorySerializationTest {

    private lateinit var tempDir: File

    @Before
    fun setup() {
        tempDir = File(System.getProperty("java.io.tmpdir"), "bizclaw_test_${System.currentTimeMillis()}")
        tempDir.mkdirs()
    }

    @After
    fun cleanup() {
        tempDir.deleteRecursively()
    }

    // ═══════════════════════════════════════════════════════
    // UiMessage Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `UiMessage defaults are correct`() {
        val msg = UiMessage(role = "user", content = "Hello")
        assertEquals("user", msg.role)
        assertEquals("Hello", msg.content)
        assertFalse(msg.isStreaming)
        assertEquals("", msg.agentName)
        assertEquals(0, msg.tokensUsed)
        assertFalse(msg.isLocal)
        assertEquals(0f, msg.tokPerSec, 0.001f)
        assertEquals("", msg.toolActions)
    }

    @Test
    fun `UiMessage with all fields`() {
        val msg = UiMessage(
            role = "assistant",
            content = "Xin chào!",
            isStreaming = true,
            agentName = "BizClaw AI",
            tokensUsed = 42,
            isLocal = true,
            tokPerSec = 15.5f,
            toolActions = "search_web(query)",
        )
        assertEquals("assistant", msg.role)
        assertEquals("Xin chào!", msg.content)
        assertTrue(msg.isStreaming)
        assertEquals("BizClaw AI", msg.agentName)
        assertEquals(42, msg.tokensUsed)
        assertTrue(msg.isLocal)
        assertEquals(15.5f, msg.tokPerSec, 0.001f)
        assertEquals("search_web(query)", msg.toolActions)
    }

    // ═══════════════════════════════════════════════════════
    // JSON Serialization Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `serialize single message to JSON`() {
        val msg = UiMessage(role = "user", content = "Xin chào")
        val json = buildJsonArray {
            addJsonObject {
                put("role", msg.role)
                put("content", msg.content)
                put("agentName", msg.agentName)
                put("isLocal", msg.isLocal.toString())
                put("toolActions", msg.toolActions)
            }
        }
        val text = json.toString()
        assertTrue(text.contains("\"role\":\"user\""))
        assertTrue(text.contains("\"content\":\"Xin chào\""))
    }

    @Test
    fun `deserialize JSON to UiMessage`() {
        val jsonText = """[{"role":"assistant","content":"Hello!","agentName":"Bot","isLocal":"true","toolActions":""}]"""
        val json = Json { ignoreUnknownKeys = true; isLenient = true }
        val arr = json.parseToJsonElement(jsonText).jsonArray
        val msg = arr.map { elem ->
            val obj = elem.jsonObject
            UiMessage(
                role = obj["role"]?.jsonPrimitive?.content ?: "user",
                content = obj["content"]?.jsonPrimitive?.content ?: "",
                agentName = obj["agentName"]?.jsonPrimitive?.content ?: "",
                isLocal = obj["isLocal"]?.jsonPrimitive?.content == "true",
                toolActions = obj["toolActions"]?.jsonPrimitive?.content ?: "",
            )
        }.first()

        assertEquals("assistant", msg.role)
        assertEquals("Hello!", msg.content)
        assertEquals("Bot", msg.agentName)
        assertTrue(msg.isLocal)
    }

    @Test
    fun `roundtrip serialize and deserialize multiple messages`() {
        val original = listOf(
            UiMessage(role = "user", content = "Hỏi A"),
            UiMessage(role = "assistant", content = "Trả lời A", agentName = "Bot", isLocal = true),
            UiMessage(role = "user", content = "Hỏi B"),
            UiMessage(role = "assistant", content = "Trả lời B", toolActions = "search()"),
        )

        // Serialize
        val jsonArr = buildJsonArray {
            original.forEach { msg ->
                addJsonObject {
                    put("role", msg.role)
                    put("content", msg.content)
                    put("agentName", msg.agentName)
                    put("isLocal", msg.isLocal.toString())
                    put("toolActions", msg.toolActions)
                }
            }
        }
        val text = jsonArr.toString()

        // Deserialize
        val json = Json { ignoreUnknownKeys = true; isLenient = true }
        val arr = json.parseToJsonElement(text).jsonArray
        val restored = arr.map { elem ->
            val obj = elem.jsonObject
            UiMessage(
                role = obj["role"]?.jsonPrimitive?.content ?: "user",
                content = obj["content"]?.jsonPrimitive?.content ?: "",
                agentName = obj["agentName"]?.jsonPrimitive?.content ?: "",
                isLocal = obj["isLocal"]?.jsonPrimitive?.content == "true",
                toolActions = obj["toolActions"]?.jsonPrimitive?.content ?: "",
            )
        }

        assertEquals(original.size, restored.size)
        for (i in original.indices) {
            assertEquals(original[i].role, restored[i].role)
            assertEquals(original[i].content, restored[i].content)
            assertEquals(original[i].agentName, restored[i].agentName)
            assertEquals(original[i].isLocal, restored[i].isLocal)
            assertEquals(original[i].toolActions, restored[i].toolActions)
        }
    }

    @Test
    fun `deserialize handles missing fields gracefully`() {
        val jsonText = """[{"role":"user","content":"Hi"}]"""
        val json = Json { ignoreUnknownKeys = true; isLenient = true }
        val arr = json.parseToJsonElement(jsonText).jsonArray
        val msg = arr.map { elem ->
            val obj = elem.jsonObject
            UiMessage(
                role = obj["role"]?.jsonPrimitive?.content ?: "user",
                content = obj["content"]?.jsonPrimitive?.content ?: "",
                agentName = obj["agentName"]?.jsonPrimitive?.content ?: "",
                isLocal = obj["isLocal"]?.jsonPrimitive?.content == "true",
                toolActions = obj["toolActions"]?.jsonPrimitive?.content ?: "",
            )
        }.first()

        assertEquals("user", msg.role)
        assertEquals("Hi", msg.content)
        assertEquals("", msg.agentName) // Missing → default
        assertFalse(msg.isLocal)        // Missing → false
        assertEquals("", msg.toolActions) // Missing → ""
    }

    @Test
    fun `deserialize handles empty array`() {
        val jsonText = "[]"
        val json = Json { ignoreUnknownKeys = true }
        val arr = json.parseToJsonElement(jsonText).jsonArray
        assertTrue(arr.isEmpty())
    }

    // ═══════════════════════════════════════════════════════
    // File I/O Tests
    // ═══════════════════════════════════════════════════════

    @Test
    fun `save and load history file`() {
        val file = File(tempDir, "test_conv.json")
        val msgs = listOf(
            UiMessage(role = "user", content = "Test message"),
            UiMessage(role = "assistant", content = "Test reply", isLocal = true),
        )

        // Write
        val jsonArr = buildJsonArray {
            msgs.forEach { msg ->
                addJsonObject {
                    put("role", msg.role)
                    put("content", msg.content)
                    put("agentName", msg.agentName)
                    put("isLocal", msg.isLocal.toString())
                    put("toolActions", msg.toolActions)
                }
            }
        }
        file.writeText(jsonArr.toString())

        // Read
        assertTrue(file.exists())
        val json = Json { ignoreUnknownKeys = true }
        val arr = json.parseToJsonElement(file.readText()).jsonArray
        assertEquals(2, arr.size)
    }

    @Test
    fun `conversation ID with slashes is sanitized for filename`() {
        val id = "group/cskh/2026"
        val sanitized = id.replace("/", "_")
        assertEquals("group_cskh_2026", sanitized)

        val file = File(tempDir, "$sanitized.json")
        file.writeText("[]")
        assertTrue(file.exists())
    }

    @Test
    fun `unicode content survives roundtrip`() {
        val unicodeContent = "Xin chào 🇻🇳 Đây là tiếng Việt 🤖"
        val jsonArr = buildJsonArray {
            addJsonObject {
                put("role", "user")
                put("content", unicodeContent)
                put("agentName", "")
                put("isLocal", "false")
                put("toolActions", "")
            }
        }
        val file = File(tempDir, "unicode_test.json")
        file.writeText(jsonArr.toString())

        val json = Json { ignoreUnknownKeys = true }
        val restored = json.parseToJsonElement(file.readText())
            .jsonArray.first().jsonObject["content"]?.jsonPrimitive?.content

        assertEquals(unicodeContent, restored)
    }

    @Test
    fun `special characters in content`() {
        val special = """Line1\nLine2\t"quoted" and <html>"""
        val jsonArr = buildJsonArray {
            addJsonObject {
                put("role", "assistant")
                put("content", special)
                put("agentName", "")
                put("isLocal", "false")
                put("toolActions", "")
            }
        }
        val text = jsonArr.toString()
        val json = Json { ignoreUnknownKeys = true }
        val restored = json.parseToJsonElement(text)
            .jsonArray.first().jsonObject["content"]?.jsonPrimitive?.content

        assertEquals(special, restored)
    }
}
