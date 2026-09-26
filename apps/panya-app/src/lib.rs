//! PanYa - Tauri application entry point.
//!
//! Wires the encrypted settings vault, the INVS connection state, and the
//! IPC command handlers. All domain logic lives in `panya-core`; all SQL in
//! `panya-invs`; this shell only adapts them to the Tauri boundary.

mod commands;
mod settings;

use commands::InvsDbState;
use settings::VaultState;
use std::time::Duration;
use tauri::{Manager, WebviewWindow};

/// Safety net: how long to wait for the frontend's `show_main_window` call
/// before revealing the window anyway, so a broken UI can never leave an
/// invisible app running.
const WINDOW_SHOW_FALLBACK: Duration = Duration::from_secs(5);

/// Reveal the main window. The frontend calls this once it has mounted, so
/// the user never sees a white WebView flash - the window appears with the
/// app shell already rendered.
#[tauri::command]
fn show_main_window(window: WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let vault =
        encryptman_keyring::Vault::new("panya").expect("failed to initialize OS keychain vault");

    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(WINDOW_SHOW_FALLBACK).await;
                    let _ = window.show();
                });
            }
            Ok(())
        })
        .manage(InvsDbState::new())
        .manage(VaultState(vault))
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            settings::save_settings,
            settings::load_settings,
            commands::invs_connect,
            commands::invs_ping,
            commands::invs_get_plan_years,
            commands::invs_get_year_data,
        ])
        .run(tauri::generate_context!())
        .expect("invariant: tauri context is generated at compile time and is always valid");
}
