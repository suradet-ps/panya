//! Tauri IPC commands - thin adapters over `panya-invs` and `panya-core`.
//!
//! Every command returns `Result<_, String>` with a Thai, operator-facing
//! message (the project override on `AGENTS-RUST.md` §5.2): the frontend
//! displays the string as-is, while the typed `thiserror` errors live in the
//! library crates.
//!
//! Verdicts are **not** computed here: `invs_get_year_data` returns the raw
//! per-year payload and the frontend runs the shared `panya-core` engine, so
//! the heavy SQL runs once per fiscal year and switching quarters is instant.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use panya_core::tracking::{ActualLine, PlanLine, YearPlan};
use panya_invs::{InvsClient, InvsDbConfig};
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

/// Shared INVS client state (one connection for the whole session).
pub struct InvsDbState(pub Arc<Mutex<Option<InvsClient>>>);

impl InvsDbState {
    /// An empty state; the client is set by `invs_connect`.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

impl Default for InvsDbState {
    fn default() -> Self {
        Self::new()
    }
}

/// Thai message used when no connection has been established yet.
const NOT_CONNECTED: &str = "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS";

/// Connect to INVS and keep the client for the session.
#[tauri::command]
pub async fn invs_connect(cfg: InvsDbConfig, state: State<'_, InvsDbState>) -> Result<(), String> {
    let client = panya_invs::connect(&cfg)
        .await
        .map_err(|e| format!("เชื่อมต่อ INVS ไม่สำเร็จ: {e}"))?;
    let mut guard = state.0.lock().await;
    *guard = Some(client);
    Ok(())
}

/// Cheap round-trip for the connection-health poll.
#[tauri::command]
pub async fn invs_ping(state: State<'_, InvsDbState>) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    let client = guard.as_mut().ok_or_else(|| NOT_CONNECTED.to_string())?;
    panya_invs::ping(client)
        .await
        .map_err(|e| format!("ตรวจสอบการเชื่อมต่อ INVS ไม่สำเร็จ: {e}"))
}

/// Distinct fiscal years present in `BUYPLAN_C`, newest first.
#[tauri::command]
pub async fn invs_get_plan_years(state: State<'_, InvsDbState>) -> Result<Vec<i32>, String> {
    let mut guard = state.0.lock().await;
    let client = guard.as_mut().ok_or_else(|| NOT_CONNECTED.to_string())?;
    panya_invs::get_plan_years(client)
        .await
        .map_err(|e| format!("ดึงปีงบประมาณจาก INVS ไม่สำเร็จ: {e}"))
}

/// The raw tracking payload for one fiscal year.
#[derive(Debug, Serialize)]
pub struct YearData {
    /// The requested fiscal year (CE, as stored in INVS).
    pub year: i32,
    /// The year-level plan from `BUYPLAN`.
    pub year_plan: YearPlan,
    /// The per-drug plan lines from `BUYPLAN_C`.
    pub plan_lines: Vec<PlanLine>,
    /// Actual purchases per quarter, keyed by `WORKING_CODE`.
    pub actual: HashMap<String, ActualLine>,
    /// Wall-clock time the queries took, for the UI's load-time hint.
    pub elapsed_ms: u64,
}

/// Fetch the plan + actual purchases for `year` - the only command that runs
/// the heavy queries, called once per fiscal year.
#[tauri::command]
pub async fn invs_get_year_data(
    year: i32,
    state: State<'_, InvsDbState>,
) -> Result<YearData, String> {
    let mut guard = state.0.lock().await;
    let client = guard.as_mut().ok_or_else(|| NOT_CONNECTED.to_string())?;

    let started = std::time::Instant::now();

    let year_plan = panya_invs::get_year_plan(client, year)
        .await
        .map_err(|e| format!("ดึงแผนรายปีไม่สำเร็จ: {e}"))?;
    let plan_lines = panya_invs::get_plan_lines(client, year)
        .await
        .map_err(|e| format!("ดึงแผนรายยาไม่สำเร็จ: {e}"))?;
    let mut actual = panya_invs::get_actual_quarters(client, year, 0)
        .await
        .map_err(|e| format!("ดึงยอดซื้อจริงไม่สำเร็จ: {e}"))?;

    // The heavy query skips the drug master, so only the unplanned codes
    // (absent from the plan, or unnamed there) need a name lookup.
    let planned: HashSet<&str> = plan_lines
        .iter()
        .map(|line| line.working_code.as_str())
        .collect();
    let missing: Vec<String> = actual
        .iter()
        .filter(|(code, line)| line.drug_name.is_none() && !planned.contains(code.as_str()))
        .map(|(code, _)| code.clone())
        .collect();
    if !missing.is_empty() {
        let names = panya_invs::get_drug_names(client, &missing)
            .await
            .map_err(|e| format!("ดึงชื่อยานอกแผนไม่สำเร็จ: {e}"))?;
        for (code, name) in names {
            if let Some(line) = actual.get_mut(&code) {
                line.drug_name = Some(name);
            }
        }
    }

    Ok(YearData {
        year,
        year_plan,
        plan_lines,
        actual,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}
