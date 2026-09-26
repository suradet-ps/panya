//! PanYa INVS - the read-only SQL Server access layer.
//!
//! [`client`] owns the `tiberius` connection lifecycle, [`config`] the
//! connection settings, and [`queries`] every `SELECT` the app needs
//! (`BUYPLAN` / `BUYPLAN_C` plans, `MS_IVO` / `MS_IVO_C` purchases,
//! `DRUG_GN` names). No Tauri types, no UI concerns: the app shell maps
//! [`InvsError`] into IPC errors, and the pure verdicts live in
//! `panya-core`.

pub mod client;
pub mod config;
pub mod queries;

pub use client::{InvsClient, connect};
pub use config::InvsDbConfig;
pub use panya_core::tracking::{ActualLine, PlanLine, YearPlan};
pub use queries::{get_actual_quarters, get_plan_lines, get_plan_years, get_year_plan, ping};

/// Everything that can go wrong while talking to INVS.
#[derive(Debug, thiserror::Error)]
pub enum InvsError {
    /// The configured port is not a number.
    #[error("invalid port: {0}")]
    InvalidPort(String),
    /// The TCP connection could not be established.
    #[error("TCP connect failed: {0}")]
    Tcp(String),
    /// The TDS/SQL Server handshake failed.
    #[error("SQL Server connect failed: {0}")]
    Connect(String),
    /// A query was rejected or failed while streaming.
    #[error("query failed: {0}")]
    Query(String),
    /// A result row could not be decoded into the expected shape.
    #[error("row decode failed: {0}")]
    Row(String),
}

/// Convenience alias used across the crate.
pub type Result<T> = std::result::Result<T, InvsError>;
