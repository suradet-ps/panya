//! INVS SQL Server connection management.
//!
//! A `tiberius::Client` over a Tokio TCP stream (`Compat`, because tiberius
//! speaks `futures` I/O while Tokio does not). The app shell owns the client
//! behind an `Arc<tokio::sync::Mutex<Option<...>>>` in Tauri managed state;
//! this crate only knows how to build one.

use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::InvsError;
use crate::config::InvsDbConfig;

/// A connected INVS client.
pub type InvsClient = Client<Compat<TcpStream>>;

/// Open a connection to INVS with `cfg`.
///
/// # Errors
///
/// Returns [`InvsError::InvalidPort`] when the port is not a number,
/// [`InvsError::Tcp`] when the TCP connection fails, and
/// [`InvsError::Connect`] when the TDS handshake fails.
pub async fn connect(cfg: &InvsDbConfig) -> Result<InvsClient, InvsError> {
    let mut config = Config::new();

    config.host(&cfg.host);
    config.port(
        cfg.port
            .parse()
            .map_err(|_| InvsError::InvalidPort(cfg.port.clone()))?,
    );
    config.authentication(AuthMethod::sql_server(&cfg.user, &cfg.password));
    config.database(&cfg.database);
    // Hospital LAN: INVS instances are typically self-signed on the local
    // network, matching the balance project's behaviour.
    config.encryption(EncryptionLevel::NotSupported);
    config.trust_cert();

    if let Some(instance) = cfg.instance.as_ref().filter(|i| !i.is_empty()) {
        config.instance_name(instance);
    }

    let tcp = TcpStream::connect(config.get_addr())
        .await
        .map_err(|e| InvsError::Tcp(e.to_string()))?;

    tcp.set_nodelay(true)
        .map_err(|e| InvsError::Tcp(e.to_string()))?;

    Client::connect(config, tcp.compat_write())
        .await
        .map_err(|e| InvsError::Connect(e.to_string()))
}
