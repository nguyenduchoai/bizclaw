//! # BizClaw Workflows
//!
//! Workflow execution engine — orchestrate multi-step, multi-agent pipelines.
//!
//! ## Workflow Types
//! | Type | Description |
//! |------|-------------|
//! | Sequential | Steps run one after another, output chains |
//! | FanOut | Multiple steps run in parallel |
//! | Collect | Gather results from parallel steps with strategy |
//! | Conditional | If/else branching based on output |
//! | Loop | Repeat until condition is met |
//!
//! ## Example
//! ```rust,no_run
//! use bizclaw_workflows::{Workflow, WorkflowStep, StepType};
//!
//! let workflow = Workflow::new("content_pipeline", "Content creation pipeline")
//!     .add_step(WorkflowStep::new("draft", "writer", StepType::Sequential))
//!     .add_step(WorkflowStep::new("review", "reviewer", StepType::Sequential))
//!     .add_step(WorkflowStep::new("final", "editor", StepType::Sequential));
//! ```

pub mod engine;
pub mod state;
pub mod step;
pub mod templates;

pub use engine::WorkflowEngine;
pub use state::{WorkflowState, WorkflowStatus};
pub use step::{
    CollectStrategy, Condition, LoopConfig, StepType, Workflow, WorkflowStep, WorkflowStepResult,
};
pub use templates::builtin_workflows;
