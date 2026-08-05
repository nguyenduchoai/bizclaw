package vn.bizclaw.agent.agent

import android.content.Context
import android.util.Log
import vn.bizclaw.agent.data.Channel
import vn.bizclaw.agent.data.Exchange
import vn.bizclaw.agent.data.ReplyState
import vn.bizclaw.agent.llm.GemmaEngine

private const val TAG = "ReplyAgent"

/** Outcome of handling one inbound message. */
data class ReplyOutcome(
    val exchange: Exchange,
    /** True when the reply may be delivered without the owner reading it first. */
    val safeToAutoSend: Boolean,
)

/**
 * Generates a customer reply from an inbound message.
 *
 * Owns no state of its own — the engine, knowledge and settings are injected so this
 * stays testable and so the notification listener can call it directly.
 */
class ReplyAgent(private val context: Context) {

    suspend fun handle(
        id: String,
        channel: Channel,
        sender: String,
        message: String,
        receivedAt: Long,
    ): ReplyOutcome {
        val app = vn.bizclaw.agent.BizClawApp.from(context)
        val settings = app.settings
        val variant = settings.modelVariant

        val base = Exchange(
            id = id,
            channel = channel,
            sender = sender,
            incoming = message,
            receivedAt = receivedAt,
        )

        if (!GemmaEngine.ensureLoaded(context, variant)) {
            return ReplyOutcome(
                base.copy(state = ReplyState.FAILED, error = GemmaEngine.lastError),
                safeToAutoSend = false,
            )
        }

        val knowledge = app.knowledge.relevantTo(message)
        val prompt = PromptBuilder.build(
            businessName = settings.businessName,
            persona = settings.persona,
            channel = channel,
            sender = sender,
            message = message,
            knowledge = knowledge,
        )

        val startedAt = System.currentTimeMillis()
        val result = GemmaEngine.generate(prompt)
        val latency = System.currentTimeMillis() - startedAt

        return result.fold(
            onSuccess = { raw ->
                val (reply, needsOwner) = PromptBuilder.parse(raw)
                if (reply.isBlank()) {
                    return@fold ReplyOutcome(
                        base.copy(
                            state = ReplyState.FAILED,
                            error = "Model trả về rỗng",
                            latencyMs = latency,
                        ),
                        safeToAutoSend = false,
                    )
                }
                ReplyOutcome(
                    base.copy(reply = reply, state = ReplyState.DRAFT, latencyMs = latency),
                    // Escalations and questions with no supporting document always wait
                    // for a human, regardless of the auto-send setting.
                    safeToAutoSend = settings.autoSend && !needsOwner && knowledge.isNotEmpty(),
                )
            },
            onFailure = { error ->
                Log.e(TAG, "Generation failed", error)
                ReplyOutcome(
                    base.copy(
                        state = ReplyState.FAILED,
                        error = error.message ?: "Lỗi sinh câu trả lời",
                        latencyMs = latency,
                    ),
                    safeToAutoSend = false,
                )
            },
        )
    }
}
