//! The KPI strip: plan, actual purchases, achievement, and alert counts.
//!
//! All values come from the tracking payload; the `% ความสำเร็จ` card is the
//! featured (pink) card - the system's single emphasis signal.

use leptos::prelude::*;

use crate::contexts::tracking::TrackingContext;
use crate::models::{format_baht, format_number, format_pct_opt};

/// Props for [`SummaryKpiBar`] - none.
#[component]
pub fn SummaryKpiBar() -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();

    let scope_label = move || {
        let quarter = tracking.quarter.get();
        if quarter == 0 {
            "ทั้งปี".to_string()
        } else {
            format!("สะสมถึง Q{quarter}")
        }
    };

    view! {
        <div class="kpi-bar">
            <div class="kpi-card">
                <div class="kpi-label">"แผนรวมปี"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .data
                            .get()
                            .map(|d| {
                                let planned = if d.year_plan.value_this_year > 0.0 {
                                    d.year_plan.value_this_year
                                } else {
                                    d.summary.plan_total
                                };
                                format_baht(planned, 0)
                            })
                            .unwrap_or_else(|| "-".to_string())
                    }}
                </div>
                <div class="kpi-sub">{scope_label}</div>
            </div>

            <div class="kpi-card">
                <div class="kpi-label">"ซื้อจริงตามช่วง"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .data
                            .get()
                            .map(|d| format_baht(d.summary.actual_total, 0))
                            .unwrap_or_else(|| "-".to_string())
                    }}
                </div>
                <div class="kpi-sub">
                    {move || {
                        tracking
                            .data
                            .get()
                            .map(|d| format!("{} รายการ", format_number(d.summary.drug_count as f64, 0)))
                            .unwrap_or_default()
                    }}
                </div>
            </div>

            <div class="kpi-card kpi-card-featured">
                <div class="kpi-label">"% ความสำเร็จ"</div>
                <div class="kpi-value kpi-value-featured">
                    {move || {
                        tracking
                            .data
                            .get()
                            .map(|d| format_pct_opt(d.summary.achievement_pct))
                            .unwrap_or_else(|| "-".to_string())
                    }}
                </div>
                <div class="kpi-sub">{scope_label}</div>
            </div>

            <div class="kpi-card">
                <div class="kpi-label">"ล่าช้า"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .data
                            .get()
                            .map(|d| d.summary.behind_count.to_string())
                            .unwrap_or_else(|| "-".to_string())
                    }}
                </div>
                <div class="kpi-sub">"รายการ"</div>
            </div>

            <div class="kpi-card">
                <div class="kpi-label">"นอกแผน"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .data
                            .get()
                            .map(|d| d.summary.unplanned_count.to_string())
                            .unwrap_or_else(|| "-".to_string())
                    }}
                </div>
                <div class="kpi-sub">"รายการ"</div>
            </div>
        </div>
    }
}
