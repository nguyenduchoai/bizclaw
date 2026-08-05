package vn.bizclaw.agent.data

import kotlinx.serialization.Serializable

/** Which chat app a message came from. */
enum class Channel(val label: String) {
    ZALO("Zalo"),
    MESSENGER("Messenger"),
    FACEBOOK("Facebook"),
    UNKNOWN("Khác"),
}

/** What happened to a reply we generated. */
enum class ReplyState {
    /** Waiting for the owner to approve before it leaves the phone. */
    DRAFT,

    /** Delivered back into the chat app. */
    SENT,

    /** Owner rejected it, or the reply action expired before we could send. */
    DISMISSED,

    /** Generation failed — see [Exchange.error]. */
    FAILED,
}

/**
 * One inbound message plus the reply we produced for it.
 *
 * A notification only stays repliable while it is on screen, so [replyKey] points at
 * the live reply action held in memory; it is deliberately not persisted.
 */
@Serializable
data class Exchange(
    val id: String,
    val channel: Channel,
    val sender: String,
    val incoming: String,
    val reply: String = "",
    val state: ReplyState = ReplyState.DRAFT,
    val error: String? = null,
    val latencyMs: Long = 0,
    val receivedAt: Long,
) {
    val isRepliable: Boolean get() = state == ReplyState.DRAFT && reply.isNotBlank()
}

/** A business fact the agent may quote — price list, shipping fee, warranty policy. */
@Serializable
data class KnowledgeDoc(
    val id: String,
    val title: String,
    val body: String,
    val updatedAt: Long,
)
