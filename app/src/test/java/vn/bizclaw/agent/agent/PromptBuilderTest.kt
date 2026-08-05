package vn.bizclaw.agent.agent

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import vn.bizclaw.agent.data.Channel
import vn.bizclaw.agent.data.KnowledgeDoc

class PromptBuilderTest {

    @Test
    fun `escalation marker is stripped and flagged`() {
        val (reply, needsOwner) = PromptBuilder.parse(
            "Dạ em kiểm tra rồi báo lại anh ngay ạ. $ESCALATE_TOKEN",
        )
        assertEquals("Dạ em kiểm tra rồi báo lại anh ngay ạ.", reply)
        assertTrue(needsOwner)
    }

    @Test
    fun `plain reply is left alone`() {
        val (reply, needsOwner) = PromptBuilder.parse("Ship nội thành 25k anh nhé.")
        assertEquals("Ship nội thành 25k anh nhé.", reply)
        assertFalse(needsOwner)
    }

    @Test
    fun `models that wrap the reply in quotes get unwrapped`() {
        val (reply, _) = PromptBuilder.parse("\"Dạ còn hàng ạ.\"")
        assertEquals("Dạ còn hàng ạ.", reply)
    }

    @Test
    fun `prompt carries the knowledge and the customer message`() {
        val prompt = PromptBuilder.build(
            businessName = "Shop ABC",
            persona = "Nhân viên CSKH",
            channel = Channel.ZALO,
            sender = "Khách A",
            message = "Phí ship Hà Nội bao nhiêu?",
            knowledge = listOf(
                KnowledgeDoc("kb-1", "Phí ship", "Nội thành HN 25k.", 0L),
            ),
        )
        assertTrue(prompt.contains("Shop ABC"))
        assertTrue(prompt.contains("Nội thành HN 25k."))
        assertTrue(prompt.contains("Phí ship Hà Nội bao nhiêu?"))
        assertTrue(prompt.contains(ESCALATE_TOKEN))
    }

    @Test
    fun `empty knowledge base is stated explicitly rather than omitted`() {
        val prompt = PromptBuilder.build(
            businessName = "",
            persona = "Nhân viên CSKH",
            channel = Channel.MESSENGER,
            sender = "Khách B",
            message = "Còn hàng không shop?",
            knowledge = emptyList(),
        )
        assertTrue(prompt.contains("(trống"))
    }
}
