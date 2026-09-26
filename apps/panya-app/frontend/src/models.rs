//! Wire types and pure formatting helpers.
//!
//! Domain types (`TrackingRow`, `TrackingSummary`, `PlanLine`, `ActualLine`,
//! `YearPlan`, `Status`) come from `panya-core`, so the verdict engine and
//! its labels have exactly one definition shared with the backend. Only the
//! IPC envelopes and the connection settings are mirrored here.

use std::collections::HashMap;

use panya_core::tracking::{ActualLine, PlanLine, YearPlan};
use serde::{Deserialize, Serialize};

/// INVS connection settings (mirror of `panya_invs::InvsDbConfig`).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct InvsDbConfig {
    /// Host name or IP address.
    pub host: String,
    /// TCP port.
    pub port: String,
    /// SQL Server login.
    pub user: String,
    /// SQL Server password.
    pub password: String,
    /// Database name.
    pub database: String,
    /// Optional named instance.
    pub instance: Option<String>,
}

/// The persisted settings payload (already decrypted by the backend).
#[derive(Clone, Debug, Deserialize)]
pub struct SettingsFile {
    /// INVS connection settings; `None` on first launch.
    pub invs: Option<InvsDbConfig>,
}

/// The `invs_get_year_data` payload: the raw per-year data. The shared
/// `panya-core` engine turns this into verdicts for the selected quarter,
/// so the heavy SQL runs once per fiscal year.
#[derive(Clone, Debug, Deserialize)]
pub struct YearData {
    /// The requested fiscal year (CE).
    pub year: i32,
    /// The year-level plan from `BUYPLAN`.
    pub year_plan: YearPlan,
    /// The per-drug plan lines from `BUYPLAN_C`.
    pub plan_lines: Vec<PlanLine>,
    /// Actual purchases per quarter, keyed by `WORKING_CODE`.
    pub actual: HashMap<String, ActualLine>,
    /// Wall-clock time the backend queries took.
    pub elapsed_ms: u64,
}

// ─── Backend error ────────────────────────────────────────────────────

/// A rejected Tauri command; the backend returns plain Thai messages.
#[derive(Clone, Debug)]
pub struct BackendError {
    /// Human-readable description suitable for display.
    pub message: String,
}

impl BackendError {
    /// Decode a rejected `invoke` payload.
    #[must_use]
    pub fn from_js(raw: wasm_bindgen::JsValue) -> Self {
        Self {
            message: raw.as_string().unwrap_or_else(|| format!("{raw:?}")),
        }
    }
}

// ─── Fiscal clock ─────────────────────────────────────────────────────

/// The current Thai fiscal year (CE), read from the browser clock.
#[must_use]
pub fn current_fiscal_year() -> i32 {
    let now = js_sys::Date::new_0();
    let month = now.get_month() + 1;
    let year = now.get_full_year() as i32;
    if month >= 10 { year + 1 } else { year }
}

// ─── Formatting helpers ───────────────────────────────────────────────

/// Format a number with thousands separators and at most `max_decimals`
/// decimal places (trailing zeros dropped).
#[must_use]
pub fn format_number(value: f64, max_decimals: u32) -> String {
    let rounded = format!("{value:.prec$}", prec = max_decimals as usize);
    let (int_part, frac_part) = rounded.split_once('.').unwrap_or((&rounded, ""));
    let frac = frac_part.trim_end_matches('0');
    if frac.is_empty() {
        group_digits(int_part)
    } else {
        format!("{}.{frac}", group_digits(int_part))
    }
}

/// Format a number as Thai Baht: `฿1,234,567` (whole baht by default).
#[must_use]
pub fn format_baht(value: f64, decimals: u32) -> String {
    format!("฿{}", format_number(value, decimals))
}

/// Format a percentage with one decimal: `92.5%`.
#[must_use]
pub fn format_pct(pct: f64) -> String {
    format!("{pct:.1}%")
}

/// Format an optional percentage; `-` when there is no plan.
#[must_use]
pub fn format_pct_opt(pct: Option<f64>) -> String {
    pct.map_or_else(|| "-".to_string(), format_pct)
}

/// Group the integer part of a decimal string with `,` separators.
fn group_digits(int_part: &str) -> String {
    let (negative, digits) = match int_part.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, int_part),
    };
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if negative { format!("-{out}") } else { out }
}
