//! Pre-built workflow templates — ready to use out of the box.

use crate::step::{
    CollectStrategy, Condition, LoopConfig, StepType, Workflow, WorkflowStep,
};

/// Get all built-in workflow templates.
pub fn builtin_workflows() -> Vec<Workflow> {
    vec![
        content_pipeline(),
        expert_consensus(),
        quality_pipeline(),
        research_pipeline(),
        translation_pipeline(),
        code_review_pipeline(),
    ]
}

/// Content creation pipeline: Draft → Review → Edit → Publish.
pub fn content_pipeline() -> Workflow {
    Workflow::new("content_pipeline", "Content creation pipeline — Draft → Review → Edit → Publish")
        .with_tags(vec!["content", "writing", "marketing"])
        .add_step(
            WorkflowStep::new("draft", "content-writer", StepType::Sequential)
                .with_prompt("Write a comprehensive article about: {{input}}")
                .with_timeout(600)
                .with_retries(1),
        )
        .add_step(
            WorkflowStep::new("review", "content-reviewer", StepType::Sequential)
                .with_prompt("Review this article for quality, accuracy, and engagement. Provide specific feedback and suggested improvements:\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("edit", "content-editor", StepType::Sequential)
                .with_prompt("Apply the review feedback and create the final polished version of this article:\n\n{{input}}")
                .with_timeout(300),
        )
}

/// Expert consensus: 3 experts analyze independently → vote/merge.
pub fn expert_consensus() -> Workflow {
    Workflow::new("expert_consensus", "Expert consensus — 3 independent analyses merged into one")
        .with_tags(vec!["analysis", "consensus", "decision"])
        .add_step(
            WorkflowStep::new("expert-a", "analyst-a", StepType::Sequential)
                .with_prompt("As Expert A, analyze this independently:\n\n{{input}}"),
        )
        .add_step(
            WorkflowStep::new("expert-b", "analyst-b", StepType::Sequential)
                .with_prompt("As Expert B, analyze this independently:\n\n{{input}}"),
        )
        .add_step(
            WorkflowStep::new("expert-c", "analyst-c", StepType::Sequential)
                .with_prompt("As Expert C, analyze this independently:\n\n{{input}}"),
        )
        .add_step(WorkflowStep::new(
            "parallel-analysis",
            "coordinator",
            StepType::FanOut {
                parallel_steps: vec![
                    "expert-a".into(),
                    "expert-b".into(),
                    "expert-c".into(),
                ],
            },
        ))
        .add_step(WorkflowStep::new(
            "merge",
            "coordinator",
            StepType::Collect {
                strategy: CollectStrategy::Merge,
                evaluator: None,
            },
        ))
}

/// Quality pipeline with evaluate loop: generate → review → revise until approved.
pub fn quality_pipeline() -> Workflow {
    Workflow::new("quality_pipeline", "Quality-gated pipeline — generate and revise until approved")
        .with_tags(vec!["quality", "review", "iterate"])
        .add_step(
            WorkflowStep::new("generate", "writer", StepType::Sequential)
                .with_prompt("Create high-quality content for: {{input}}")
                .with_timeout(600),
        )
        .add_step(WorkflowStep::new(
            "refine",
            "reviewer",
            StepType::Loop {
                body_step: "generate".into(),
                config: LoopConfig::new(
                    3,
                    Condition::new("quality", "contains", "APPROVED"),
                ),
            },
        ))
}

/// Research pipeline: Search → Analyze → Synthesize → Report.
pub fn research_pipeline() -> Workflow {
    Workflow::new("research_pipeline", "Research pipeline — Search → Analyze → Synthesize → Report")
        .with_tags(vec!["research", "analysis", "report"])
        .add_step(
            WorkflowStep::new("search", "researcher", StepType::Sequential)
                .with_prompt("Research the following topic thoroughly. Find key facts, data, and sources:\n\n{{input}}")
                .with_timeout(600),
        )
        .add_step(
            WorkflowStep::new("analyze", "analyst", StepType::Sequential)
                .with_prompt("Analyze the research findings below. Identify patterns, insights, and key takeaways:\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("synthesize", "synthesizer", StepType::Sequential)
                .with_prompt("Synthesize the analysis into a coherent narrative with conclusions and recommendations:\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("report", "report-writer", StepType::Sequential)
                .with_prompt("Format the synthesis into a professional report with executive summary, findings, and next steps:\n\n{{input}}")
                .with_timeout(300),
        )
}

/// Translation pipeline with quality check.
pub fn translation_pipeline() -> Workflow {
    Workflow::new("translation_pipeline", "Translation with quality verification")
        .with_tags(vec!["translation", "i18n", "quality"])
        .add_step(
            WorkflowStep::new("translate", "translator", StepType::Sequential)
                .with_prompt("Translate the following text to the target language, maintaining tone and meaning:\n\n{{input}}")
                .with_retries(1),
        )
        .add_step(
            WorkflowStep::new("verify", "translation-reviewer", StepType::Sequential)
                .with_prompt("Review this translation for accuracy, naturalness, and cultural appropriateness. If issues found, provide the corrected version:\n\n{{input}}")
                .optional(),
        )
}

/// Code review pipeline: Analyze → Security check → Style check → Summary.
pub fn code_review_pipeline() -> Workflow {
    Workflow::new("code_review", "Code review pipeline — analyze, security, style, summary")
        .with_tags(vec!["code", "review", "security"])
        .add_step(
            WorkflowStep::new("analyze", "code-analyst", StepType::Sequential)
                .with_prompt("Analyze this code for bugs, logic errors, and potential improvements:\n\n{{input}}")
                .with_timeout(600),
        )
        .add_step(
            WorkflowStep::new("security", "security-expert", StepType::Sequential)
                .with_prompt("Review this code for security vulnerabilities (injection, auth bypass, data exposure):\n\n{{input}}")
                .with_timeout(300),
        )
        .add_step(
            WorkflowStep::new("style", "style-checker", StepType::Sequential)
                .with_prompt("Review this code for style, readability, and best practices:\n\n{{input}}")
                .optional(),
        )
        .add_step(
            WorkflowStep::new(
                "summary",
                "coordinator",
                StepType::Transform {
                    template: "## Code Review Summary\n\n{{input}}".to_string(),
                },
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_workflows_count() {
        let workflows = builtin_workflows();
        assert_eq!(workflows.len(), 6);
    }

    #[test]
    fn test_content_pipeline_structure() {
        let wf = content_pipeline();
        assert_eq!(wf.name, "content_pipeline");
        assert_eq!(wf.step_count(), 3);
        assert!(wf.get_step("draft").is_some());
        assert!(wf.get_step("review").is_some());
        assert!(wf.get_step("edit").is_some());
    }

    #[test]
    fn test_expert_consensus_structure() {
        let wf = expert_consensus();
        assert_eq!(wf.name, "expert_consensus");
        assert!(wf.step_count() >= 4);
    }

    #[test]
    fn test_code_review_has_optional() {
        let wf = code_review_pipeline();
        let style_step = wf.get_step("style").unwrap();
        assert!(style_step.optional);
    }

    #[test]
    fn test_all_workflows_have_tags() {
        for wf in builtin_workflows() {
            assert!(!wf.tags.is_empty(), "Workflow '{}' has no tags", wf.name);
        }
    }
}
