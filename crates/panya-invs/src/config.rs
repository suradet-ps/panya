//! INVS connection settings (the plain value type).
//!
//! Encryption and persistence are the app shell's job (`settings.rs`); this
//! crate only needs the decrypted values to open a connection.

use serde::{Deserialize, Serialize};

/// SQL Server connection settings for INVS.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct InvsDbConfig {
    /// Host name or IP address.
    pub host: String,
    /// TCP port (kept as a string for form binding; parsed on connect).
    pub port: String,
    /// SQL Server login.
    pub user: String,
    /// SQL Server password.
    pub password: String,
    /// Database name.
    pub database: String,
    /// Optional named instance (SQL Browser resolves the port when set).
    pub instance: Option<String>,
}
