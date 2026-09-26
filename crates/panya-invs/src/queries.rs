//! Read-only INVS queries: plans, actual purchases, and the drug master.
//!
//! Table map (see `AGENTS.md` for the schema reference):
//!
//! - `BUYPLAN`   - year-level plan (`VALUE_THIS_YEAR`, `TRIMESTER1..4`)
//! - `BUYPLAN_C` - per-drug plan lines (`WORKING_CODE`, `TRIMESTER1..4`)
//! - `MS_IVO`    - purchase header (`RECEIVE_DATE` as `YYYYMMDD` int)
//! - `MS_IVO_C`  - purchase lines (`WORKING_CODE`, `VALUE`, `QTY_ORDER`)
//! - `DRUG_GN`   - drug names (`WORKING_CODE`, `DRUG_NAME`)
//!
//! Every query is a `SELECT`; all amounts are `CAST(... AS FLOAT)` so the
//! driver never has to decode SQL Server `DECIMAL`/`MONEY` variants, and
//! date bucketing is done in Rust via `panya-core` (one source of truth).

use std::collections::HashMap;

use futures::TryStreamExt;
use panya_core::fiscal::{cal_month_to_quarter_idx, quarter_range};
use panya_core::tracking::{ActualLine, PlanLine, YearPlan};
use tiberius::{QueryItem, Row, ToSql};

use crate::InvsError;
use crate::client::InvsClient;

/// Convert a driver query error.
fn query_err(error: tiberius::error::Error) -> InvsError {
    InvsError::Query(error.to_string())
}

/// Convert a driver row-stream error.
fn row_err(error: tiberius::error::Error) -> InvsError {
    InvsError::Row(error.to_string())
}

/// Decode a string column, trimming trailing padding and defaulting to `""`.
fn get_str(row: &Row, idx: usize) -> String {
    row.get::<&str, usize>(idx).unwrap_or("").trim().to_string()
}

/// Decode a numeric column tolerating SQL Server's integer/real variants.
fn get_f64(row: &Row, idx: usize) -> f64 {
    if let Some(v) = row.get::<f64, usize>(idx) {
        return v;
    }
    if let Some(v) = row.get::<f32, usize>(idx) {
        return f64::from(v);
    }
    if let Some(v) = row.get::<i32, usize>(idx) {
        return f64::from(v);
    }
    if let Some(v) = row.get::<i16, usize>(idx) {
        return f64::from(v);
    }
    0.0
}

/// Decode an integer column tolerating `SMALLINT`/`TINYINT` variants.
fn get_i32(row: &Row, idx: usize) -> i32 {
    if let Some(v) = row.get::<i32, usize>(idx) {
        return v;
    }
    if let Some(v) = row.get::<i16, usize>(idx) {
        return i32::from(v);
    }
    0
}

/// Distinct fiscal years present in `BUYPLAN_C`, newest first.
///
/// # Errors
///
/// Returns [`InvsError::Query`] / [`InvsError::Row`] when the query fails.
pub async fn get_plan_years(client: &mut InvsClient) -> Result<Vec<i32>, InvsError> {
    let query = "
        SELECT DISTINCT CAST([YEAR] AS INT) AS fiscal_year
        FROM BUYPLAN_C
        WHERE [YEAR] IS NOT NULL
          AND CAST([YEAR] AS INT) > 0
        ORDER BY fiscal_year DESC
    ";

    let mut stream = client.query(query, &[]).await.map_err(query_err)?;
    let mut years = Vec::new();

    while let Some(item) = stream.try_next().await.map_err(row_err)? {
        if let QueryItem::Row(row) = item {
            let year = get_i32(&row, 0);
            if year > 0 {
                years.push(year);
            }
        }
    }

    Ok(years)
}

/// The year-level plan from `BUYPLAN` (zeroes when the year has no row).
///
/// # Errors
///
/// Returns [`InvsError::Query`] / [`InvsError::Row`] when the query fails.
pub async fn get_year_plan(client: &mut InvsClient, year: i32) -> Result<YearPlan, InvsError> {
    let query = "
        SELECT
            ISNULL(SUM(CAST([VALUE_THIS_YEAR] AS FLOAT)), 0) AS value_this_year,
            ISNULL(SUM(CAST([TRIMESTER1] AS FLOAT)), 0) AS q1,
            ISNULL(SUM(CAST([TRIMESTER2] AS FLOAT)), 0) AS q2,
            ISNULL(SUM(CAST([TRIMESTER3] AS FLOAT)), 0) AS q3,
            ISNULL(SUM(CAST([TRIMESTER4] AS FLOAT)), 0) AS q4
        FROM BUYPLAN
        WHERE CAST([YEAR] AS INT) = @P1
    ";

    let mut stream = client.query(query, &[&year]).await.map_err(query_err)?;
    let mut plan = YearPlan::default();

    while let Some(item) = stream.try_next().await.map_err(row_err)? {
        if let QueryItem::Row(row) = item {
            plan = YearPlan {
                value_this_year: get_f64(&row, 0),
                quarters: [
                    get_f64(&row, 1),
                    get_f64(&row, 2),
                    get_f64(&row, 3),
                    get_f64(&row, 4),
                ],
            };
            break;
        }
    }

    Ok(plan)
}

/// The per-drug plan lines from `BUYPLAN_C`, joined to `DRUG_GN` for names.
///
/// Rows are grouped per `WORKING_CODE` and their quarters summed: if the
/// target site ever stacks plan revisions for one code, SUM may over-count -
/// see the open item in `AGENTS.md`.
///
/// # Errors
///
/// Returns [`InvsError::Query`] / [`InvsError::Row`] when the query fails.
pub async fn get_plan_lines(
    client: &mut InvsClient,
    year: i32,
) -> Result<Vec<PlanLine>, InvsError> {
    let query = "
        SELECT
            c.[WORKING_CODE],
            ISNULL(MAX(g.[DRUG_NAME]), '') AS drug_name,
            ISNULL(SUM(CAST(c.[TRIMESTER1] AS FLOAT)), 0) AS q1,
            ISNULL(SUM(CAST(c.[TRIMESTER2] AS FLOAT)), 0) AS q2,
            ISNULL(SUM(CAST(c.[TRIMESTER3] AS FLOAT)), 0) AS q3,
            ISNULL(SUM(CAST(c.[TRIMESTER4] AS FLOAT)), 0) AS q4
        FROM BUYPLAN_C c
        LEFT JOIN DRUG_GN g ON g.[WORKING_CODE] = c.[WORKING_CODE]
        WHERE CAST(c.[YEAR] AS INT) = @P1
        GROUP BY c.[WORKING_CODE]
        ORDER BY c.[WORKING_CODE]
    ";

    let mut stream = client.query(query, &[&year]).await.map_err(query_err)?;
    let mut lines = Vec::new();

    while let Some(item) = stream.try_next().await.map_err(row_err)? {
        if let QueryItem::Row(row) = item {
            lines.push(PlanLine {
                working_code: get_str(&row, 0),
                drug_name: get_str(&row, 1),
                quarters: [
                    get_f64(&row, 2),
                    get_f64(&row, 3),
                    get_f64(&row, 4),
                    get_f64(&row, 5),
                ],
            });
        }
    }

    Ok(lines)
}

/// Actual purchase values per quarter, keyed by `WORKING_CODE`.
///
/// `quarter` `0` covers the whole fiscal year; `1..=4` narrows the window
/// to that quarter. The returned arrays are always full-year shaped, so the
/// table and the detail view keep all four quarters either way.
///
/// No `DRUG_GN` join here: this is the heavy query (a whole fiscal year of
/// purchase lines), and names are only needed for unplanned codes - fetched
/// separately by [`get_drug_names`] for just those codes.
///
/// # Errors
///
/// Returns [`InvsError::Query`] / [`InvsError::Row`] when the query fails.
pub async fn get_actual_quarters(
    client: &mut InvsClient,
    year: i32,
    quarter: u8,
) -> Result<HashMap<String, ActualLine>, InvsError> {
    let (start_date, end_date) = quarter_range(year, quarter);

    // `RECEIVE_DATE` is an INT in YYYYMMDD form, so the calendar month is
    // integer arithmetic - no per-row CAST to varchar/date, and the GROUP BY
    // stays on plain integer expressions.
    let query = "
        SELECT
            c.[WORKING_CODE],
            (h.[RECEIVE_DATE] / 100) % 100 AS cal_month,
            SUM(ISNULL(CAST(c.[VALUE] AS FLOAT), 0)) AS total_value
        FROM MS_IVO_C c
        JOIN MS_IVO h ON c.[INVOICE_NO] = h.[INVOICE_NO]
        WHERE h.[RECEIVE_DATE] >= @P1
          AND h.[RECEIVE_DATE] <= @P2
        GROUP BY
            c.[WORKING_CODE],
            (h.[RECEIVE_DATE] / 100) % 100
        ORDER BY c.[WORKING_CODE]
    ";

    let mut stream = client
        .query(query, &[&start_date, &end_date])
        .await
        .map_err(query_err)?;

    let mut actual: HashMap<String, ActualLine> = HashMap::new();

    while let Some(item) = stream.try_next().await.map_err(row_err)? {
        if let QueryItem::Row(row) = item {
            let code = get_str(&row, 0);
            if code.is_empty() {
                continue;
            }
            let Some(idx) = cal_month_to_quarter_idx(get_i32(&row, 1).unsigned_abs()) else {
                continue;
            };
            let value = get_f64(&row, 2);

            actual.entry(code).or_default().quarters[idx] += value;
        }
    }

    Ok(actual)
}

/// Drug names from `DRUG_GN` for the given codes, chunked into one query per
/// `NAME_LOOKUP_CHUNK` codes (the heavy purchase query no longer joins the
/// drug master, so only the unplanned codes need a name).
///
/// # Errors
///
/// Returns [`InvsError::Query`] / [`InvsError::Row`] when a query fails.
pub async fn get_drug_names(
    client: &mut InvsClient,
    codes: &[String],
) -> Result<HashMap<String, String>, InvsError> {
    /// Codes per `IN (...)` batch; keeps the parameter count sane.
    const NAME_LOOKUP_CHUNK: usize = 500;

    let mut names = HashMap::new();

    for chunk in codes.chunks(NAME_LOOKUP_CHUNK) {
        let placeholders = (1..=chunk.len())
            .map(|i| format!("@P{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT [WORKING_CODE], ISNULL([DRUG_NAME], '') \
             FROM DRUG_GN WHERE [WORKING_CODE] IN ({placeholders})"
        );
        let params: Vec<&dyn ToSql> = chunk.iter().map(|code| code as &dyn ToSql).collect();

        let mut stream = client.query(&query, &params).await.map_err(query_err)?;

        while let Some(item) = stream.try_next().await.map_err(row_err)? {
            if let QueryItem::Row(row) = item {
                let code = get_str(&row, 0);
                let name = get_str(&row, 1);
                if !code.is_empty() && !name.is_empty() {
                    names.insert(code, name);
                }
            }
        }
    }

    Ok(names)
}

/// Cheap round-trip for the connection-health poll; drains the reply.
///
/// # Errors
///
/// Returns [`InvsError::Query`] / [`InvsError::Row`] when the round-trip fails.
pub async fn ping(client: &mut InvsClient) -> Result<(), InvsError> {
    let mut stream = client.simple_query("SELECT 1").await.map_err(query_err)?;
    while stream.try_next().await.map_err(row_err)?.is_some() {}
    Ok(())
}
