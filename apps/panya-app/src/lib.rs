//! PanYa - Tauri application entry point.
//!
//! Wires the encrypted settings vault, the INVS connection state, and the
//! IPC command handlers. All domain logic lives in `panya-core`; all SQL in
//! `panya-invs`; this shell only adapts them to the Tauri boundary.

mod commands;
mod settings;

use commands::InvsDbState;
use settings::VaultState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let vault =
        encryptman_keyring::Vault::new("panya").expect("failed to initialize OS keychain vault");

    tauri::Builder::default()
        .manage(InvsDbState::new())
        .manage(VaultState(vault))
        .invoke_handler(tauri::generate_handler![
            settings::save_settings,
            settings::load_settings,
            commands::invs_connect,
            commands::invs_ping,
            commands::invs_get_plan_years,
            commands::invs_get_tracking,
        ])
        .run(tauri::generate_context!())
        .expect("invariant: tauri context is generated at compile time and is always valid");
}
