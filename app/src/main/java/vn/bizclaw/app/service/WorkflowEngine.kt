package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.delay

/**
 * WorkflowEngine — Chain nhiều app/goal thành 1 workflow, LLM hỗ trợ khi cần.
 *
 * Khác với FlowRunner (chạy instant, không LLM), WorkflowEngine:
 * - Cho phép mỗi step có 1 "goal" tự nhiên → LLM agent thực hiện
 * - Link output step trước → input step sau (data pipeline)
 * - Retry failed steps
 * - Mixed mode: vừa macro (FlowRunner) vừa AI (agent loop)
 *
 * ```
 * WorkflowDefinition:
 *   Step 1: [app: Shopee] goal: "Check đơn hàng mới, lấy danh sách"
 *     ↓ result: "3 đơn mới: A, B, C"
 *   Step 2: [app: Zalo] goal: "Báo cáo đơn hàng cho Boss: {prev_result}"
 *     ↓ result: "Đã gửi"
 *   Step 3: [flow: cross_post] content: "Xin cảm ơn quý khách!"
 *     ↓ result: "3/3 posted"
 * ```
 *
 * Modes:
 * - AGENT: Use LocalAgentLoop to achieve a goal (AI-powered)
 * - FLOW: Run a predefined FlowDefinition (instant, no LLM)
 */
class WorkflowEngine(private val context: Context) {

    companion object {
        private const val TAG = "WorkflowEngine"
    }

    private val appController = AppController(context)
    private val flowRunner = FlowRunner(context)

    /**
     * Run a complete workflow from start to finish.
     *
     * @param workflow The workflow to execute
     * @param agentRunner Function that runs the AI agent for AGENT-type steps
     * @param onStepComplete Callback for UI progress reporting
     * @return WorkflowResult with all step outcomes
     */
    suspend fun run(
        workflow: WorkflowDefinition,
        agentRunner: (suspend (query: String) -> String)? = null,
        onStepComplete: ((Int, WorkflowStepResult) -> Unit)? = null,
    ): WorkflowResult {
        Log.i(TAG, "🚀 Starting workflow: ${workflow.name} (${workflow.steps.size} steps)")
        val startTime = System.currentTimeMillis()
        val stepResults = mutableListOf<WorkflowStepResult>()
        val contextVars = mutableMapOf<String, String>()

        for ((index, step) in workflow.steps.withIndex()) {
            if (!step.enabled) {
                Log.d(TAG, "⏭️ Step ${index + 1} skipped (disabled)")
                stepResults.add(WorkflowStepResult(
                    stepName = step.name,
                    mode = step.mode,
                    success = true,
                    output = "Skipped",
                    durationMs = 0
                ))
                continue
            }

            Log.i(TAG, "🔧 Workflow step ${index + 1}/${workflow.steps.size}: ${step.name}")
            val stepStart = System.currentTimeMillis()

            // Resolve template variables in goal/params
            val resolvedGoal = resolveTemplate(step.goal, contextVars)
            val resolvedParams = step.params.mapValues { (_, v) -> resolveTemplate(v, contextVars) }

            val result = try {
                when (step.mode) {
                    WorkflowStepMode.AGENT -> executeAgentStep(step, resolvedGoal, agentRunner)
                    WorkflowStepMode.FLOW -> executeFlowStep(step, resolvedParams)
                    WorkflowStepMode.ACTION -> executeActionStep(step, resolvedParams)
                }
            } catch (e: Exception) {
                Log.e(TAG, "Workflow step ${index + 1} failed", e)
                WorkflowStepResult(
                    stepName = step.name,
                    mode = step.mode,
                    success = false,
                    output = "Exception: ${e.message}",
                    durationMs = System.currentTimeMillis() - stepStart
                )
            }

            // Store output for next steps
            contextVars["step_${index + 1}"] = result.output
            contextVars["prev_result"] = result.output
            contextVars["last_output"] = result.output

            stepResults.add(result.copy(durationMs = System.currentTimeMillis() - stepStart))
            onStepComplete?.invoke(index, result)

            // Delay between steps
            if (step.delayAfterMs > 0 && index < workflow.steps.size - 1) {
                delay(step.delayAfterMs)
            }

            // Stop on failure
            if (!result.success && workflow.stopOnFailure) {
                Log.w(TAG, "⛔ Workflow stopped at step ${index + 1}")
                break
            }
        }

        val totalMs = System.currentTimeMillis() - startTime
        Log.i(TAG, "✅ Workflow '${workflow.name}' done in ${totalMs}ms")

        return WorkflowResult(
            workflowName = workflow.name,
            totalSteps = workflow.steps.size,
            successCount = stepResults.count { it.success },
            stepResults = stepResults,
            totalDurationMs = totalMs,
            contextVars = contextVars
        )
    }

    // ═══════════════════════════════════════════════════════════════
    // Step Executors
    // ═══════════════════════════════════════════════════════════════

    /**
     * AGENT mode: Use LLM agent to achieve a natural language goal.
     */
    private suspend fun executeAgentStep(
        step: WorkflowStep,
        resolvedGoal: String,
        agentRunner: (suspend (String) -> String)?
    ): WorkflowStepResult {
        if (agentRunner == null) {
            return WorkflowStepResult(
                stepName = step.name,
                mode = WorkflowStepMode.AGENT,
                success = false,
                output = "No agent runner configured"
            )
        }

        // Open target app if specified
        step.appPackage?.let { pkg ->
            appController.openApp(pkg)
            delay(2500) // Wait for app to load
        }

        val output = agentRunner(resolvedGoal)
        return WorkflowStepResult(
            stepName = step.name,
            mode = WorkflowStepMode.AGENT,
            success = output.isNotBlank(),
            output = output
        )
    }

    /**
     * FLOW mode: Run a predefined FlowDefinition (instant, no LLM).
     */
    private suspend fun executeFlowStep(
        step: WorkflowStep,
        resolvedParams: Map<String, String>
    ): WorkflowStepResult {
        // Build flow from params
        val flowType = resolvedParams["flow_type"] ?: "cross_post"
        val content = resolvedParams["content"] ?: ""

        val flow = when (flowType) {
            "cross_post" -> FlowRunner.crossPostFlow(content)
            else -> {
                // Try to load saved flow by ID
                val savedFlow = flowRunner.loadFlow(flowType)
                if (savedFlow != null) savedFlow
                else FlowRunner.crossPostFlow(content)
            }
        }

        val result = flowRunner.run(flow)
        return WorkflowStepResult(
            stepName = step.name,
            mode = WorkflowStepMode.FLOW,
            success = result.allSuccess,
            output = result.summary()
        )
    }

    /**
     * ACTION mode: Single direct action (like FlowStep but standalone).
     */
    private suspend fun executeActionStep(
        step: WorkflowStep,
        resolvedParams: Map<String, String>
    ): WorkflowStepResult {
        val actionName = resolvedParams["action"] ?: return WorkflowStepResult(
            stepName = step.name, mode = WorkflowStepMode.ACTION,
            success = false, output = "Missing 'action' param"
        )

        val flowAction = try {
            FlowAction.valueOf(actionName.uppercase())
        } catch (e: Exception) {
            return WorkflowStepResult(
                stepName = step.name, mode = WorkflowStepMode.ACTION,
                success = false, output = "Unknown action: $actionName"
            )
        }

        val flowStep = FlowStep(action = flowAction, params = resolvedParams)
        val flowDef = FlowDefinition(name = step.name, steps = listOf(flowStep))
        val result = flowRunner.run(flowDef)

        return WorkflowStepResult(
            stepName = step.name,
            mode = WorkflowStepMode.ACTION,
            success = result.allSuccess,
            output = result.stepResults.firstOrNull()?.message ?: "No result"
        )
    }

    // ═══════════════════════════════════════════════════════════════
    // Template Resolution
    // ═══════════════════════════════════════════════════════════════

    /**
     * Resolve {{variable}} templates in text using context vars.
     * Also supports {prev_result}, {step_1}, etc.
     */
    private fun resolveTemplate(template: String, vars: Map<String, String>): String {
        var result = template
        for ((key, value) in vars) {
            result = result.replace("{{$key}}", value)
            result = result.replace("{$key}", value)
        }
        return result
    }
}

// ═══════════════════════════════════════════════════════════════
// Data Types
// ═══════════════════════════════════════════════════════════════

data class WorkflowDefinition(
    val name: String,
    val description: String = "",
    val steps: List<WorkflowStep>,
    val stopOnFailure: Boolean = true,
)

data class WorkflowStep(
    val name: String,
    val mode: WorkflowStepMode,
    val goal: String = "",              // For AGENT mode: natural language goal
    val appPackage: String? = null,     // App to open before this step
    val params: Map<String, String> = emptyMap(),  // For FLOW/ACTION modes
    val delayAfterMs: Long = 3000,
    val enabled: Boolean = true,
)

enum class WorkflowStepMode {
    AGENT,   // Use LLM agent to achieve goal
    FLOW,    // Run a predefined flow (no LLM)
    ACTION,  // Single direct action
}

data class WorkflowStepResult(
    val stepName: String,
    val mode: WorkflowStepMode,
    val success: Boolean,
    val output: String,
    val durationMs: Long = 0,
)

data class WorkflowResult(
    val workflowName: String,
    val totalSteps: Int,
    val successCount: Int,
    val stepResults: List<WorkflowStepResult>,
    val totalDurationMs: Long,
    val contextVars: Map<String, String>,
) {
    val allSuccess: Boolean get() = successCount == totalSteps

    fun summary(): String = buildString {
        appendLine("🔗 Workflow: $workflowName")
        appendLine("✅ Success: $successCount/$totalSteps")
        appendLine("⏱️ Duration: ${totalDurationMs / 1000}s")
        appendLine()
        for ((i, r) in stepResults.withIndex()) {
            val icon = if (r.success) "✅" else "❌"
            val mode = when (r.mode) {
                WorkflowStepMode.AGENT -> "🤖"
                WorkflowStepMode.FLOW -> "⚡"
                WorkflowStepMode.ACTION -> "🔧"
            }
            appendLine("  ${i + 1}. $icon $mode ${r.stepName}: ${r.output.take(80)}")
        }
    }
}
