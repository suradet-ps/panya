//! Encrypted settings persistence.
//!
//! The INVS connection settings are the only persisted state (PanYa is
//! otherwise stateless). Values are encrypted field-by-field with
//! AES-256-GCM via `encryptman-keyring`; the master key lives in the OS
//! keychain, so no plaintext credential ever reaches disk, logs, or errors.

use encryptman_keyring::Vault;
use panya_invs::InvsDbConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

/// Tauri-managed vault handle.
pub struct VaultState(pub Vault);

/// The on-disk settings shape (values encrypted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsFile {
    /// INVS connection settings; `None` until the operator configures it.
    pub invs: Option<InvsDbConfig>,
}

/// Encrypt every field of an INVS config.
fn encrypt_config(vault: &Vault, cfg: &InvsDbConfig) -> Result<InvsDbConfig, String> {
    Ok(InvsDbConfig {
        host: vault.encrypt(&cfg.host).map_err(|e| e.to_string())?,
        port: vault.encrypt(&cfg.port).map_err(|e| e.to_string())?,
        user: vault.encrypt(&cfg.user).map_err(|e| e.to_string())?,
        password: vault.encrypt(&cfg.password).map_err(|e| e.to_string())?,
        database: vault.encrypt(&cfg.database).map_err(|e| e.to_string())?,
        instance: cfg
            .instance
            .as_ref()
            .map(|v| vault.encrypt(v))
            .transpose()
            .map_err(|e| e.to_string())?,
    })
}

/// Decrypt every field of an INVS config.
fn decrypt_config(vault: &Vault, cfg: &InvsDbConfig) -> Result<InvsDbConfig, String> {
    Ok(InvsDbConfig {
        host: vault.decrypt(&cfg.host).map_err(|e| e.to_string())?,
        port: vault.decrypt(&cfg.port).map_err(|e| e.to_string())?,
        user: vault.decrypt(&cfg.user).map_err(|e| e.to_string())?,
        password: vault.decrypt(&cfg.password).map_err(|e| e.to_string())?,
        database: vault.decrypt(&cfg.database).map_err(|e| e.to_string())?,
        instance: cfg
            .instance
            .as_ref()
            .map(|v| vault.decrypt(v))
            .transpose()
            .map_err(|e| e.to_string())?,
    })
}

/// Path of `settings.json` inside the app-data directory.
fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create app data dir: {e}"))?;
    dir.push("settings.json");
    Ok(dir)
}

/// Persist the (encrypted) INVS connection settings.
#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    vault: tauri::State<'_, VaultState>,
    invs: Option<InvsDbConfig>,
) -> Result<(), String> {
    let encrypted = SettingsFile {
        invs: invs.map(|c| encrypt_config(&vault.0, &c)).transpose()?,
    };
    let path = settings_path(&app)?;
    let json = serde_json::to_string_pretty(&encrypted)
        .map_err(|e| format!("serialization failed: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write failed: {e}"))?;
    Ok(())
}

/// Load the persisted settings (decrypted). A missing file is a valid empty
/// state (first launch), not an error.
#[tauri::command]
pub async fn load_settings(
    app: tauri::AppHandle,
    vault: tauri::State<'_, VaultState>,
) -> Result<SettingsFile, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(SettingsFile { invs: None });
    }
    let json =
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read settings file: {e}"))?;
    let encrypted: SettingsFile =
        serde_json::from_str(&json).map_err(|e| format!("parse failed: {e}"))?;
    Ok(SettingsFile {
        invs: encrypted
            .invs
            .map(|c| decrypt_config(&vault.0, &c))
            .transpose()?,
    })
}
