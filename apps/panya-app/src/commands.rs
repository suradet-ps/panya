//! Tauri IPC commands - thin adapters over `panya-invs` and `panya-core`.
//!
//! Every command returns `Result<_, String>` with a Thai, operator-facing
//! message (the project override on `AGENTS-RUST.md` §5.2): the frontend
//! displays the string as-is, while the typed `thiserror` errors live in the
//! library crates.

use std::sync::Arc;

use panya_core::tracking::{Thresholds, TrackingRow, TrackingSummary, YearPlan, track};
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

/// The full tracking payload for one fiscal year and quarter scope.
#[derive(Debug, Serialize)]
pub struct TrackingResponse {
    /// The selected fiscal year (CE, as stored in INVS).
    pub year: i32,
    /// The selected scope: `0` = ทั้งปี, `1..=4` = quarter.
    pub quarter: u8,
    /// The year-level plan from `BUYPLAN`.
    pub year_plan: YearPlan,
    /// Headline numbers for the KPI strip.
    pub summary: TrackingSummary,
    /// One row per drug (plan lines first, unplanned purchases appended).
    pub rows: Vec<TrackingRow>,
}

/// Fetch plans + actual purchases for `year`, then run the pure engine with
/// the `quarter` scope (`0` = ทั้งปี).
#[tauri::command]
pub async fn invs_get_tracking(
    year: i32,
    quarter: u8,
    state: State<'_, InvsDbState>,
) -> Result<TrackingResponse, String> {
    let mut guard = state.0.lock().await;
    let client = guard.as_mut().ok_or_else(|| NOT_CONNECTED.to_string())?;

    let year_plan = panya_invs::get_year_plan(client, year)
        .await
        .map_err(|e| format!("ดึงแผนรายปีไม่สำเร็จ: {e}"))?;
    let plan_lines = panya_invs::get_plan_lines(client, year)
        .await
        .map_err(|e| format!("ดึงแผนรายยาไม่สำเร็จ: {e}"))?;
    // The actual window is always the whole fiscal year: the quarter scope
    // only affects the verdict math, so the table can always show all four
    // quarters.
    let actual = panya_invs::get_actual_quarters(client, year, 0)
        .await
        .map_err(|e| format!("ดึงยอดซื้อจริงไม่สำเร็จ: {e}"))?;

    let (rows, summary) = track(&plan_lines, &actual, quarter, Thresholds::default());

    Ok(TrackingResponse {
        year,
        quarter,
        year_plan,
        summary,
        rows,
    })
}
