//! # BizClaw Platform
//!
//! Multi-tenant management platform — run multiple BizClaw agents on a single VPS.
//! Includes admin dashboard, tenant lifecycle management, pairing security,
//! subdomain routing, resource monitoring, and audit logging.
//! Now with PostgreSQL support, ReMe Memory, Heartbeat/Cron, and Skills.

pub mod admin;
pub mod auth;
pub mod config;
pub mod db;
pub mod db_pg;
pub mod tenant;
pub mod self_serve;

pub use admin::AdminServer;
pub use db::PlatformDb;
pub use db_pg::PgDb;
pub use tenant::TenantManager;
