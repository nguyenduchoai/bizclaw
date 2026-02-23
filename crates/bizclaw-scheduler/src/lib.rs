//! # BizClaw Scheduler
//!
//! Ultra-lightweight task scheduler, notification, and workflow system.
//! Inspired by PicoClaw's file-based state and ZeroClaw's <10ms cold start.
//!
//! ## Design Principles (for 512MB RAM devices)
//! - No external dependencies (no Redis, no RabbitMQ)
//! - SQLite persistence — survives restarts
//! - Tokio timers only — zero overhead when idle
//! - Notification routing + dispatch — actually sends to channels
//! - Workflow engine — trigger→condition→action automation
//!
//! ## Architecture
//! ```text
//! Scheduler (tokio interval)
//!   ├── CronTask: "0 8 * * *" → "Tóm tắt email"
//!   ├── OnceTask: "2026-02-22 15:00" → "Họp team"
//!   ├── IntervalTask: every 30min → "Check server"
//!   └── on trigger → NotificationRouter → Dispatch
//!                      ├── Telegram (sendMessage)
//!                      ├── Discord (webhook)
//!                      ├── Webhook (HTTP POST)
//!                      └── Dashboard (WebSocket)
//!
//! Workflow Engine
//!   ├── Event (message, schedule, metric) → evaluate rules
//!   ├── Matching rules → generate actions
//!   └── Actions: agent_prompt, notify, webhook, delegate
//! ```

pub mod cron;
pub mod dispatch;
pub mod engine;
pub mod notify;
pub mod persistence;
pub mod store;
pub mod tasks;
pub mod workflow;

pub use engine::SchedulerEngine;
pub use notify::{Notification, NotifyChannel, NotifyRouter};
pub use persistence::SchedulerDb;
pub use store::TaskStore;
pub use tasks::{Task, TaskStatus, TaskType};
pub use workflow::{WorkflowAction, WorkflowEngine, WorkflowEvent};
