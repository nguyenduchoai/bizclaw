package vn.bizclaw.app.engine

import android.os.HardwarePropertiesManager
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.util.concurrent.atomic.AtomicBoolean

/**
 * On-device AI Meeting Assistant Use Case
 * Translated from the HearoPilot architecture to fix BizClaw OOM crashing issues on Android.
 * Features:
 * 1. Stateless LLM inference (Sliding window of last 3 segments)
 * 2. Memory Throttling & Thermal Checking
 * 3. Atomic thread locks to prevent duplicate JNI calls.
 */
class MeetingAssistantUseCase(
    private val useAiEdge: Boolean = true // Mặc định sử dụng Gemma 4 qua LiteRT-LM (AiEdgeEngine)
) {
    private val isLlmBusy = AtomicBoolean(false)
    private val TAG = "MeetingAssistant"

    // Rolling buffer of the last N complete transcriptions
    private val segmentRollingBuffer = mutableListOf<String>()
    private val MAX_HISTORY_SEGMENTS = 3

    // Meeting insight output
    private val _meetingInsight = MutableStateFlow("")
    val meetingInsight: StateFlow<String> = _meetingInsight

    /**
     * Called automatically whenever Sherpa-ONNX completes an STT segment.
     */
    suspend fun onNewTranscriptionSegment(
        newSegment: String,
        availableRamMb: Long,
        isDeviceHot: Boolean
    ) {
        // 1. Min-word gate
        if (newSegment.trim().split(" ").size < 5) {
            Log.d(TAG, "Segment too short. Skipping.")
            return
        }

        // 2. Memory & Thermal Guards
        // ThermalThrottle: Double wait or skip if overheating
        if (isDeviceHot || availableRamMb < 400) {
            Log.e(TAG, "Thermal/RAM Guard active! Skipping LLM inference. RAM: \$availableRamMb MB, Hot: \$isDeviceHot")
            return
        }

        // 3. Concurrent-call guard (Prevent JNI crash)
        if (!isLlmBusy.compareAndSet(false, true)) {
            Log.d(TAG, "LLM is busy generating previous insight. Skipping to prevent JNI crash.")
            return
        }

        try {
            // 4. Sliding Window Context Build
            val contextHistory = segmentRollingBuffer.joinToString("\n")
            
            // Re-build prompt using the Sliding Window
            val prompt = """
                [SYSTEM]: Bạn là Thư ký Cuộc họp (BizClaw AI). Dựa vào Lịch sử và Câu nói mới, hãy cập nhật Tóm tắt và Hành động.
                
                [LỊCH SỬ]:
                \$contextHistory
                
                [MỚI]:
                \$newSegment
                
                [TÓM TẮT & HÀNH ĐỘNG]:
            """.trimIndent()

            // 5. Native Interference (JNI cache reused implicitly if implemented in PicoLMEngine)
            Log.d(TAG, "Sending prompt to Gemma 4 (Size: \${prompt.length})...")
            
            // Call local engine (AiEdge LiteRT hoặc llama.cpp cũ)
            val newInsight = if (useAiEdge && GlobalLLM.aiEdgeEngine?.isLoaded == true) {
                Log.d(TAG, "Routing inference to AiEdgeEngine (Gemma 4 LiteRT)...")
                GlobalLLM.aiEdgeEngine!!.getResponse(prompt)
            } else {
                Log.d(TAG, "Routing inference to legacy BizClawLLM (llama.cpp JNI)...")
                GlobalLLM.instance.getResponse(prompt)
            }
            
            // Push to flow
            _meetingInsight.value = newInsight

            // 6. Push to sliding window
            segmentRollingBuffer.add(newSegment)
            if (segmentRollingBuffer.size > MAX_HISTORY_SEGMENTS) {
                segmentRollingBuffer.removeAt(0)
            }

        } catch (e: Exception) {
            Log.e(TAG, "Error generating insight: \${e.message}")
        } finally {
            // Unlock LLM
            isLlmBusy.set(false)
        }
    }

    fun clearSession() {
        segmentRollingBuffer.clear()
        _meetingInsight.value = ""
    }
}
