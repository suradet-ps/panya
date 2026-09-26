//! Typed wrappers around the individual Tauri commands.
//!
//! Each function maps the raw `invoke` result/error into a domain type or a
//! [`BackendError`]. Argument names use the camelCase JS keys that Tauri v2
//! bridges to the snake_case Rust parameters.

use serde::Serialize;
use wasm_bindgen::JsValue;

use super::tauri::{build_args, invoke};
use crate::models::{BackendError, InvsDbConfig, SettingsFile, YearData};

fn arg<T: Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap_or(JsValue::UNDEFINED)
}

/// Reveal the main window once the UI has mounted (the window starts hidden
/// so the user never sees a white WebView flash).
pub async fn show_main_window() -> Result<(), BackendError> {
    invoke::<()>("show_main_window", &JsValue::NULL)
        .await
        .map_err(BackendError::from_js)
}

/// Persist the INVS connection settings (encrypted by the backend).
pub async fn save_settings(invs: Option<&InvsDbConfig>) -> Result<(), BackendError> {
    let args = build_args(&[("invs", &arg(&invs))]);
    invoke::<()>("save_settings", &args)
        .await
        .map_err(BackendError::from_js)
}

/// Load the persisted connection settings (decrypted).
pub async fn load_settings() -> Result<SettingsFile, BackendError> {
    invoke::<SettingsFile>("load_settings", &JsValue::NULL)
        .await
        .map_err(BackendError::from_js)
}

/// Test the INVS connection with `cfg`; on success the backend keeps it.
/// Returns the round-trip time in milliseconds.
pub async fn invs_connect(cfg: &InvsDbConfig) -> Result<u64, BackendError> {
    let args = build_args(&[("cfg", &arg(cfg))]);
    invoke::<u64>("invs_connect", &args)
        .await
        .map_err(BackendError::from_js)
}

/// Cheap round-trip for the connection-health poll.
pub async fn invs_ping() -> Result<(), BackendError> {
    invoke::<()>("invs_ping", &JsValue::NULL)
        .await
        .map_err(BackendError::from_js)
}

/// Distinct fiscal years present in `BUYPLAN_C`, newest first.
pub async fn invs_get_plan_years() -> Result<Vec<i32>, BackendError> {
    invoke::<Vec<i32>>("invs_get_plan_years", &JsValue::NULL)
        .await
        .map_err(BackendError::from_js)
}

/// Fetch the raw plan + actual payload for `year` (once per fiscal year).
pub async fn invs_get_year_data(year: i32) -> Result<YearData, BackendError> {
    let args = build_args(&[("year", &JsValue::from(year))]);
    invoke::<YearData>("invs_get_year_data", &args)
        .await
        .map_err(BackendError::from_js)
}
