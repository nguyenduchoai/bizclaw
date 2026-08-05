package vn.bizclaw.agent.agent

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Guards the tool schemas.
 *
 * A schema that fails strict-mode validation is rejected by the provider on every
 * request, so the whole agent stops answering — worth catching at build time rather
 * than on the owner's phone.
 */
class AgentToolSpecsTest {

    private val json = Json { ignoreUnknownKeys = true }

    @Test
    fun `every schema is parseable JSON`() {
        AgentToolSpecs.ALL.forEach { spec ->
            json.parseToJsonElement(spec.parameters).jsonObject
        }
    }

    @Test
    fun `strict mode requires additionalProperties false`() {
        AgentToolSpecs.ALL.forEach { spec ->
            val schema = json.parseToJsonElement(spec.parameters).jsonObject
            assertEquals(
                "${spec.name} phải đặt additionalProperties=false cho strict mode",
                "false",
                schema["additionalProperties"]?.jsonPrimitive?.content,
            )
        }
    }

    @Test
    fun `strict mode requires every property to be listed as required`() {
        AgentToolSpecs.ALL.forEach { spec ->
            val schema = json.parseToJsonElement(spec.parameters).jsonObject
            val properties = schema["properties"]!!.jsonObject.keys
            val required = schema["required"]!!.jsonArray.map { it.jsonPrimitive.content }.toSet()
            assertEquals("${spec.name} thiếu required", properties, required)
        }
    }

    @Test
    fun `tool names are unique and dispatchable`() {
        val names = AgentToolSpecs.ALL.map { it.name }
        assertEquals(names.size, names.toSet().size)
        assertTrue(names.contains(AgentToolSpecs.LOOKUP_PRODUCT))
        assertTrue(names.contains(AgentToolSpecs.LOOKUP_POLICY))
        assertTrue(names.contains(AgentToolSpecs.CREATE_ORDER))
    }

    @Test
    fun `descriptions state a trigger condition, not just what the tool does`() {
        AgentToolSpecs.ALL.forEach { spec ->
            // "khi" is the Vietnamese "when" — a description without it says what the
            // tool does but never when to reach for it, which is what drives call rate.
            assertTrue("${spec.name} thiếu điều kiện gọi", spec.description.contains("khi"))
            assertTrue("${spec.name} mô tả quá sơ sài", spec.description.length >= 80)
        }
    }

    @Test
    fun `tool system prompt forbids answering prices from memory`() {
        val system = PromptBuilder.toolSystem(
            businessName = "Shop ABC",
            persona = "Nhân viên CSKH",
            channel = vn.bizclaw.agent.data.Channel.ZALO,
        )
        assertTrue(system.contains("PHẢI lấy từ tool"))
        assertTrue(system.contains(ESCALATE_TOKEN))
        assertFalse("Không được nhét bảng giá vào system prompt", system.contains("₫"))
    }
}
