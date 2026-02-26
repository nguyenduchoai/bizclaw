//! # BizClaw Database Abstraction Layer
//!
//! Provides a unified `DataStore` trait with implementations for:
//! - **SQLite** (default, standalone mode) — zero-config, file-based
//! - **PostgreSQL** (optional, managed mode) — multi-tenant, pgvector
//!
//! All orchestration data (delegations, teams, tasks, handoffs, traces)
//! flows through this abstraction layer.

pub mod store;
pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;

pub use store::DataStore;
pub use sqlite::SqliteStore;
#[cfg(feature = "postgres")]
pub use postgres::PostgresStore;
