package vn.bizclaw.app.engine

import android.content.Context
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Local Agent Manager — CRUD cho agents lưu trên thiết bị.
 *
 * Mỗi agent có:
 * - Tên, emoji, vai trò
 * - System prompt riêng
 * - Gắn với 0-N knowledge base (RAG)
 * - Trigger settings (auto-reply rules)
 */
@Serializable
data class LocalAgent(
    val id: String,
    val emoji: String = "🤖",
    val name: String,
    val role: String = "",
    val systemPrompt: String,
    val knowledgeBaseIds: List<String> = emptyList(), // RAG IDs attached
    val triggerApps: List<String> = emptyList(), // e.g. ["com.zing.zalo", "com.facebook.orca"]
    val autoReply: Boolean = false,
    val providerId: String = "local_gguf", // AI provider to use
    val groupId: String? = null, // Agent group membership
    val createdAt: Long = System.currentTimeMillis(),
)

class LocalAgentManager(private val context: Context) {

    private val json = Json { prettyPrint = true; ignoreUnknownKeys = true }
    private val agentsFile = File(context.filesDir, "local_agents.json")

    fun loadAgents(): List<LocalAgent> {
        if (!agentsFile.exists()) return defaultAgents()
        return try {
            val savedAgents = json.decodeFromString<List<LocalAgent>>(agentsFile.readText())
            val defaultAgentsList = defaultAgents()
            var savedAgentsList = savedAgents.toMutableList()
            var modified = false
            
            // Force update old CSKH agent to new V2 version with autoReply
            for (i in savedAgentsList.indices) {
                if (savedAgentsList[i].id == "customer_support" && savedAgentsList[i].name == "CSKH") {
                    savedAgentsList[i] = defaultAgentsList.first { it.id == "customer_support" }
                    modified = true
                }
            }

            // Inject any missing default agents that the user doesn't have yet
            val missingDefaults = defaultAgentsList.filter { def -> savedAgentsList.none { it.id == def.id } }
            if (missingDefaults.isNotEmpty()) {
                val mergedAgents = savedAgentsList + missingDefaults
                saveAgents(mergedAgents)
                mergedAgents
            } else {
                if (modified) saveAgents(savedAgentsList)
                savedAgentsList
            }
        } catch (e: Exception) {
            defaultAgents()
        }
    }

    fun saveAgents(agents: List<LocalAgent>) {
        agentsFile.writeText(json.encodeToString(agents))
    }

    fun addAgent(agent: LocalAgent) {
        val agents = loadAgents().toMutableList()
        agents.add(agent)
        saveAgents(agents)
    }

    fun updateAgent(agent: LocalAgent) {
        val agents = loadAgents().toMutableList()
        val idx = agents.indexOfFirst { it.id == agent.id }
        if (idx >= 0) {
            agents[idx] = agent
            saveAgents(agents)
        }
    }

    fun deleteAgent(agentId: String) {
        val agents = loadAgents().toMutableList()
        agents.removeAll { it.id == agentId }
        saveAgents(agents)
    }

    fun getAgent(agentId: String): LocalAgent? {
        return loadAgents().find { it.id == agentId }
    }

    /** Build the full system prompt for an agent, including RAG context */
    fun buildPromptForAgent(agent: LocalAgent, userQuery: String): String {
        val sb = StringBuilder(agent.systemPrompt)

        // Append RAG context from all attached knowledge bases
        if (agent.knowledgeBaseIds.isNotEmpty()) {
            val ragContexts = agent.knowledgeBaseIds.mapNotNull { kbId ->
                try {
                    val rag = LocalRAG(context, kbId)
                    val ctx = rag.buildContext(userQuery)
                    if (ctx.isNotBlank()) ctx else null
                } catch (e: Exception) {
                    null
                }
            }
            if (ragContexts.isNotEmpty()) {
                sb.append("\n\n")
                sb.append(ragContexts.joinToString("\n\n"))
            }
        }

        return sb.toString()
    }

    /** Default agents — Vietnamese business personas */
    private fun defaultAgents(): List<LocalAgent> {
        val defaults = listOf(
            LocalAgent(
                id = "bizclaw_default",
                emoji = "🤖",
                name = "BizClaw",
                role = "Trợ lý tổng hợp",
                systemPrompt = "Bạn tên là BizClaw, trợ lý AI thân thiện chạy trên điện thoại. " +
                    "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
                    "Trả lời ngắn gọn, tự nhiên. Khi được chào, chào lại thân thiện. " +
                    "Không bịa thông tin.",
            ),
            LocalAgent(
                id = "copywriter",
                emoji = "📝",
                name = "Copywriter",
                role = "Viết nội dung & quảng cáo",
                systemPrompt = "Bạn là chuyên gia viết nội dung marketing. " +
                    "CHỈ viết bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
                    "Viết caption Facebook, mô tả sản phẩm, email marketing, bài blog. " +
                    "Phong cách sáng tạo, thu hút, phù hợp thị trường Việt Nam.",
            ),
            LocalAgent(
                id = "analyst",
                emoji = "📊",
                name = "Phân tích",
                role = "Phân tích dữ liệu & báo cáo",
                systemPrompt = "Bạn là chuyên gia phân tích kinh doanh. " +
                    "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
                    "Giúp phân tích số liệu, đưa ra nhận xét và đề xuất. " +
                    "Trả lời có cấu trúc, dùng gạch đầu dòng.",
            ),
            LocalAgent(
                id = "strategist",
                emoji = "🎯",
                name = "Chiến lược",
                role = "Tư vấn chiến lược kinh doanh",
                systemPrompt = "Bạn là cố vấn chiến lược kinh doanh cho doanh nghiệp Việt Nam. " +
                    "CHỈ trả lời bằng tiếng Việt, KHÔNG dùng tiếng Trung hay tiếng Anh. " +
                    "Tư vấn marketing, bán hàng, vận hành, nhân sự. " +
                    "Đưa lời khuyên thực tế, áp dụng được ngay.",
            ),
            LocalAgent(
                id = "customer_support",
                emoji = "💬",
                name = "Chiến Thần CSKH",
                role = "Chăm sóc khách hàng tự động",
                systemPrompt = "Bạn là nhân viên CSKH cực kỳ chuyên nghiệp, tận tâm và linh hoạt. " +
                    "Tuyệt đối KHÔNG ĐƯỢC dùng tiếng Trung hay tiếng Anh, CHỈ giao tiếp bằng tiếng Việt chuẩn. " +
                    "Nhiệm vụ của bạn:\n" +
                    "1. Trả lời ngay vào trọng tâm câu hỏi của khách hàng một cách ngắn gọn, không lan man.\n" +
                    "2. Luôn xưng hô lễ phép (với ngữ điệu tự nhiên: 'Dạ', 'Vâng', 'Dạ anh/chị', 'Em chào anh/chị').\n" +
                    "3. Giải quyết khiếu nại bằng sự kiên nhẫn, thấu cảm, xoa dịu khách.\n" +
                    "4. Chủ động khơi gợi nhu cầu và dẫn dắt khách hàng để chốt sale (nếu là câu hỏi về sản phẩm).\n" +
                    "Bạn không phải là máy móc, bạn là con người, tư vấn thực tế và khôn khéo.",
                autoReply = true,
                triggerApps = listOf("com.zing.zalo", "com.facebook.orca", "org.telegram.messenger")
            ),
        )
        // Note: Do not call saveAgents(defaults) here to avoid unnecessary disk writes 
        // every time defaultAgents() is evaluated for comparison in loadAgents().
        // They are already saved conditionally in loadAgents() if missing.
        return defaults
    }
}
