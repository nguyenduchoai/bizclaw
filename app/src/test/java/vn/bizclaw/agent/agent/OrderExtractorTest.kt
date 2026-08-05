package vn.bizclaw.agent.agent

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class OrderExtractorTest {

    @Test
    fun `a phone number alone is enough to suspect an order`() {
        assertTrue(OrderExtractor.looksLikeOrder("0912345678 nhé shop"))
        assertTrue(OrderExtractor.looksLikeOrder("sdt cua minh la +84912345678"))
    }

    @Test
    fun `order keywords trigger extraction even without accents`() {
        assertTrue(OrderExtractor.looksLikeOrder("chốt đơn cho mình cái này"))
        assertTrue(OrderExtractor.looksLikeOrder("giao den 12 Le Loi"))
        assertTrue(OrderExtractor.looksLikeOrder("mua 2 cái"))
    }

    @Test
    fun `plain questions do not pay for a second inference`() {
        assertFalse(OrderExtractor.looksLikeOrder("Shop ơi cái này còn không?"))
        assertFalse(OrderExtractor.looksLikeOrder("Bảo hành bao lâu vậy ạ"))
    }

    @Test
    fun `full payload parses into an order with nothing missing`() {
        val extracted = OrderExtractor.parse(
            """
            {"has_order":true,"customer_name":"Chị Lan","phone":"0912345678",
             "address":"12 Lê Lợi, Q1","product":"Váy lụa","quantity":2}
            """,
        )!!
        assertEquals("Chị Lan", extracted.customerName)
        assertEquals("0912345678", extracted.phone)
        assertEquals(2, extracted.quantity)
        assertTrue(extracted.missing.isEmpty())
    }

    @Test
    fun `blank fields are reported as missing rather than invented`() {
        val extracted = OrderExtractor.parse(
            """
            {"has_order":true,"customer_name":"","phone":"","address":"",
             "product":"Váy lụa","quantity":1}
            """,
        )!!
        assertEquals(listOf("tên", "số điện thoại", "địa chỉ"), extracted.missing)
    }

    @Test
    fun `a placeholder phone is discarded instead of stored`() {
        val extracted = OrderExtractor.parse(
            """
            {"has_order":true,"customer_name":"Anh Nam","phone":"số điện thoại của bạn",
             "address":"Hà Nội","product":"Váy lụa","quantity":1}
            """,
        )!!
        assertEquals("", extracted.phone)
        assertTrue(extracted.missing.contains("số điện thoại"))
    }

    @Test
    fun `quantity below one is clamped`() {
        val extracted = OrderExtractor.parse(
            """
            {"has_order":true,"customer_name":"A","phone":"0912345678",
             "address":"HN","product":"X","quantity":0}
            """,
        )!!
        assertEquals(1, extracted.quantity)
    }

    @Test
    fun `garbage output yields null rather than a half-built order`() {
        assertNull(OrderExtractor.parse("xin lỗi em không hiểu"))
        assertNull(OrderExtractor.parse(""))
    }

    @Test
    fun `extraction prompt carries the catalog and forbids inventing fields`() {
        val prompt = OrderExtractor.buildPrompt(
            catalog = "- Váy lụa: 450,000₫ (còn 3)",
            sender = "Chị Lan",
            message = "lấy 2 cái nhé",
        )
        assertTrue(prompt.contains("Váy lụa"))
        assertTrue(prompt.contains("KHÔNG được bịa"))
        assertTrue(prompt.contains("lấy 2 cái nhé"))
    }
}
