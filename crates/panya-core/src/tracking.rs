//! The plan-vs-actual tracking engine.
//!
//! Pure functions: plan values (`BUYPLAN_C`) and actual purchase values
//! (`MS_IVO_C`) go in, per-drug verdicts and a headline summary come out.
//! No I/O, no clock - the caller supplies the quarter scope (`0` = ทั้งปี),
//! so every verdict is reproducible and unit-tested.
//!
//! Verdict precedence: `ไม่มีข้อมูล` → `นอกแผน` → `เกินแผน` → `ล่าช้า` →
//! `เฝ้าระวัง` → `ตามแผน`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::fiscal::QUARTERS;

/// The verdict for one drug line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// No usable plan and no purchase in scope.
    NoData,
    /// Purchase in scope but no plan line.
    Unplanned,
    /// Actual value above the plan (plus the over tolerance).
    Over,
    /// Actual value below the watch floor.
    Behind,
    /// Actual value below the on-track floor.
    Watch,
    /// Actual value within tolerance of the plan.
    OnTrack,
}

impl Status {
    /// Thai label shown to the operator (the color is never the only signal).
    #[must_use]
    pub fn label_th(self) -> &'static str {
        match self {
            Self::NoData => "ไม่มีข้อมูล",
            Self::Unplanned => "นอกแผน",
            Self::Over => "เกินแผน",
            Self::Behind => "ล่าช้า",
            Self::Watch => "เฝ้าระวัง",
            Self::OnTrack => "ตามแผน",
        }
    }

    /// Stable machine key used for CSS state classes.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::NoData => "nodata",
            Self::Unplanned => "unplanned",
            Self::Over => "over",
            Self::Behind => "behind",
            Self::Watch => "watch",
            Self::OnTrack => "ontrack",
        }
    }
}

/// Verdict thresholds as ratios of actual ÷ plan.
///
/// Defaults: on track at ≥ 90 %, watch at ≥ 60 %, over above 120 %.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// Minimum ratio still counted as `ตามแผน`.
    pub ontrack_min: f64,
    /// Minimum ratio still counted as `เฝ้าระวัง` (below this: `ล่าช้า`).
    pub watch_min: f64,
    /// Ratio above which the line counts as `เกินแผน`.
    pub over_max: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            ontrack_min: 0.90,
            watch_min: 0.60,
            over_max: 1.20,
        }
    }
}

/// The year-level plan (`BUYPLAN`): total for the year plus its four quarters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct YearPlan {
    /// `BUYPLAN.VALUE_THIS_YEAR`.
    pub value_this_year: f64,
    /// `BUYPLAN.TRIMESTER1..4`, index 0 = Q1 (ต.ค.–ธ.ค.).
    pub quarters: [f64; QUARTERS],
}

/// One per-drug plan line as stored in `BUYPLAN_C`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanLine {
    /// INVS `WORKING_CODE`.
    pub working_code: String,
    /// `DRUG_GN.DRUG_NAME` when available.
    pub drug_name: String,
    /// `BUYPLAN_C.TRIMESTER1..4`, index 0 = Q1 (ต.ค.–ธ.ค.).
    pub quarters: [f64; QUARTERS],
}

impl PlanLine {
    /// The full-year plan value of this line.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.quarters.iter().sum()
    }
}

/// Actual purchase values per quarter for one `WORKING_CODE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActualLine {
    /// `DRUG_GN.DRUG_NAME` when the join found the drug.
    pub drug_name: Option<String>,
    /// `MS_IVO_C.VALUE` summed per quarter, index 0 = Q1 (ต.ค.–ธ.ค.).
    pub quarters: [f64; QUARTERS],
}

/// One row of the tracking table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackingRow {
    /// INVS `WORKING_CODE`.
    pub working_code: String,
    /// Drug display name (plan name, actual name, or empty when unknown).
    pub drug_name: String,
    /// Full-year plan per quarter.
    pub plan: [f64; QUARTERS],
    /// Full-year actual per quarter.
    pub actual: [f64; QUARTERS],
    /// Plan summed through the selected quarter (`0` = full year).
    pub plan_sum: f64,
    /// Actual summed through the selected quarter (`0` = full year).
    pub actual_sum: f64,
    /// `actual_sum / plan_sum × 100`, `None` when there is no plan.
    pub achievement_pct: Option<f64>,
    /// The verdict for the selected scope.
    pub status: Status,
}

/// Headline numbers for the KPI strip.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackingSummary {
    /// Plan for the year (`BUYPLAN.VALUE_THIS_YEAR`, falling back to the
    /// summed per-drug plan when the year row is missing).
    pub plan_total: f64,
    /// Actual purchases summed through the selected quarter.
    pub actual_total: f64,
    /// `actual_total / plan_total × 100`, `None` when there is no plan.
    pub achievement_pct: Option<f64>,
    /// Number of drug lines in the table.
    pub drug_count: usize,
    /// Lines with the `ล่าช้า` verdict.
    pub behind_count: usize,
    /// Lines with the `นอกแผน` verdict.
    pub unplanned_count: usize,
}

/// Sum the first `through_quarter` quarters; `0` (or anything out of range)
/// sums the whole year.
#[must_use]
pub fn cumulative(quarters: &[f64; QUARTERS], through_quarter: u8) -> f64 {
    let n = if (1..=QUARTERS as u8).contains(&through_quarter) {
        usize::from(through_quarter)
    } else {
        QUARTERS
    };
    quarters[..n].iter().sum()
}

/// Classify one drug line from its scoped plan and actual sums.
#[must_use]
pub fn classify(plan: f64, actual: f64, thresholds: Thresholds) -> Status {
    if plan <= 0.0 {
        return if actual > 0.0 {
            Status::Unplanned
        } else {
            Status::NoData
        };
    }
    let ratio = actual / plan;
    if ratio > thresholds.over_max {
        Status::Over
    } else if ratio >= thresholds.ontrack_min {
        Status::OnTrack
    } else if ratio >= thresholds.watch_min {
        Status::Watch
    } else {
        Status::Behind
    }
}

/// Track every plan line against the actual purchases.
///
/// Both directions are visible: plan lines without purchases get a verdict
/// too, and purchases without a plan line are appended as `นอกแผน`.
#[must_use]
pub fn track(
    plan_lines: &[PlanLine],
    actual: &HashMap<String, ActualLine>,
    through_quarter: u8,
    thresholds: Thresholds,
) -> (Vec<TrackingRow>, TrackingSummary) {
    let mut rows: Vec<TrackingRow> = Vec::with_capacity(plan_lines.len() + actual.len());
    let mut planned: HashSet<&str> = HashSet::with_capacity(plan_lines.len());

    for line in plan_lines {
        planned.insert(line.working_code.as_str());
        let act = actual.get(&line.working_code);
        let actual_quarters = act.map_or([0.0; QUARTERS], |a| a.quarters);
        let plan_sum = cumulative(&line.quarters, through_quarter);
        let actual_sum = cumulative(&actual_quarters, through_quarter);
        rows.push(TrackingRow {
            working_code: line.working_code.clone(),
            drug_name: line.drug_name.clone(),
            plan: line.quarters,
            actual: actual_quarters,
            plan_sum,
            actual_sum,
            achievement_pct: percentage(plan_sum, actual_sum),
            status: classify(plan_sum, actual_sum, thresholds),
        });
    }

    for (code, act) in actual {
        if planned.contains(code.as_str()) {
            continue;
        }
        let actual_sum = cumulative(&act.quarters, through_quarter);
        rows.push(TrackingRow {
            working_code: code.clone(),
            drug_name: act.drug_name.clone().unwrap_or_default(),
            plan: [0.0; QUARTERS],
            actual: act.quarters,
            plan_sum: 0.0,
            actual_sum,
            achievement_pct: None,
            status: classify(0.0, actual_sum, thresholds),
        });
    }

    let plan_total: f64 = rows.iter().map(|r| r.plan_sum).sum();
    let actual_total: f64 = rows.iter().map(|r| r.actual_sum).sum();
    let summary = TrackingSummary {
        plan_total,
        actual_total,
        achievement_pct: percentage(plan_total, actual_total),
        drug_count: rows.len(),
        behind_count: rows.iter().filter(|r| r.status == Status::Behind).count(),
        unplanned_count: rows
            .iter()
            .filter(|r| r.status == Status::Unplanned)
            .count(),
    };

    (rows, summary)
}

/// `part / whole × 100`, or `None` when the whole is zero.
#[must_use]
fn percentage(whole: f64, part: f64) -> Option<f64> {
    if whole > 0.0 {
        Some(part / whole * 100.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_line(code: &str, quarters: [f64; 4]) -> PlanLine {
        PlanLine {
            working_code: code.to_string(),
            drug_name: format!("drug {code}"),
            quarters,
        }
    }

    #[test]
    fn cumulative_sums_through_the_selected_quarter() {
        let quarters = [10.0, 20.0, 30.0, 40.0];
        assert_eq!(cumulative(&quarters, 0), 100.0);
        assert_eq!(cumulative(&quarters, 2), 30.0);
        assert_eq!(cumulative(&quarters, 4), 100.0);
        assert_eq!(cumulative(&quarters, 9), 100.0, "out of range = full year");
    }

    #[test]
    fn classify_covers_every_verdict() {
        let t = Thresholds::default();
        assert_eq!(classify(0.0, 0.0, t), Status::NoData);
        assert_eq!(classify(0.0, 50.0, t), Status::Unplanned);
        assert_eq!(classify(100.0, 0.0, t), Status::Behind);
        assert_eq!(classify(100.0, 30.0, t), Status::Behind);
        assert_eq!(classify(100.0, 70.0, t), Status::Watch);
        assert_eq!(classify(100.0, 90.0, t), Status::OnTrack);
        assert_eq!(classify(100.0, 150.0, t), Status::Over);
    }

    #[test]
    fn classify_boundaries_are_inclusive_at_the_floor() {
        let t = Thresholds::default();
        assert_eq!(classify(100.0, 89.99, t), Status::Watch);
        assert_eq!(classify(100.0, 60.0, t), Status::Watch);
        assert_eq!(classify(100.0, 59.99, t), Status::Behind);
        assert_eq!(
            classify(100.0, 120.0, t),
            Status::OnTrack,
            "exactly 120 % is not over"
        );
        assert_eq!(classify(100.0, 120.01, t), Status::Over);
    }

    #[test]
    fn track_marks_purchases_without_a_plan_as_unplanned() {
        let plans = vec![plan_line("A", [25.0, 25.0, 25.0, 25.0])];
        let actual = HashMap::from([
            (
                "A".to_string(),
                ActualLine {
                    drug_name: Some("drug A".to_string()),
                    quarters: [20.0, 20.0, 0.0, 0.0],
                },
            ),
            (
                "C".to_string(),
                ActualLine {
                    drug_name: Some("drug C".to_string()),
                    quarters: [5.0, 0.0, 0.0, 0.0],
                },
            ),
        ]);

        let (rows, summary) = track(&plans, &actual, 2, Thresholds::default());

        let row_a = rows
            .iter()
            .find(|r| r.working_code == "A")
            .expect("plan row");
        assert_eq!(row_a.plan_sum, 50.0, "Q1+Q2 plan");
        assert_eq!(row_a.actual_sum, 40.0, "Q1+Q2 actual");
        assert_eq!(row_a.status, Status::Watch);
        assert!(
            row_a
                .achievement_pct
                .is_some_and(|p| (p - 80.0).abs() < 1e-9)
        );

        let row_c = rows
            .iter()
            .find(|r| r.working_code == "C")
            .expect("unplanned row");
        assert_eq!(row_c.plan_sum, 0.0);
        assert_eq!(row_c.status, Status::Unplanned);

        assert_eq!(summary.drug_count, 2);
        assert_eq!(summary.unplanned_count, 1);
        assert_eq!(summary.plan_total, 50.0);
        assert_eq!(summary.actual_total, 45.0);
    }

    #[test]
    fn track_flags_a_plan_line_without_purchases_as_behind() {
        let plans = vec![plan_line("A", [100.0, 0.0, 0.0, 0.0])];
        let (rows, summary) = track(&plans, &HashMap::new(), 1, Thresholds::default());

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Status::Behind);
        assert_eq!(rows[0].achievement_pct, Some(0.0));
        assert_eq!(summary.behind_count, 1);
        assert_eq!(summary.actual_total, 0.0);
    }

    #[test]
    fn track_without_a_plan_reports_none_percent() {
        let (rows, summary) = track(&[], &HashMap::new(), 0, Thresholds::default());
        assert!(rows.is_empty());
        assert_eq!(summary.achievement_pct, None);
    }

    #[test]
    fn full_year_scope_uses_all_four_quarters() {
        let plans = vec![plan_line("A", [25.0, 25.0, 25.0, 25.0])];
        let actual = HashMap::from([(
            "A".to_string(),
            ActualLine {
                drug_name: Some("drug A".to_string()),
                quarters: [25.0, 25.0, 25.0, 25.0],
            },
        )]);
        let (rows, _) = track(&plans, &actual, 0, Thresholds::default());
        assert_eq!(rows[0].actual_sum, 100.0);
        assert_eq!(rows[0].status, Status::OnTrack);
    }
}
