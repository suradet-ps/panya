//! The KPI strip: plan, actual purchases, achievement, and alert counts.
//!
//! The summary comes from the shared engine's computed verdicts (instant on
//! quarter change); the year plan comes from the raw payload. The
//! `% ความสำเร็จ` card is the featured (sky-blue) card - the system's single
//! emphasis signal. Every read goes through `with`, so showing the numbers
//! never clones the whole payload.

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
                            .with(|data| {
                                data.as_ref()
                                    .map(|d| {
                                        let planned = if d.year_plan.value_this_year > 0.0 {
                                            d.year_plan.value_this_year
                                        } else {
                                            tracking
                                                .computed
                                                .with(|computed| {
                                                    computed
                                                        .as_ref()
                                                        .map_or(0.0, |c| c.summary.plan_total)
                                                })
                                        };
                                        format_baht(planned, 0)
                                    })
                            })
                            .unwrap_or_else(|| "—".to_string())
                    }}
                </div>
                <div class="kpi-sub">{scope_label}</div>
            </div>

            <div class="kpi-card">
                <div class="kpi-label">"ซื้อจริงตามช่วง"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .computed
                            .with(|computed| {
                                computed
                                    .as_ref()
                                    .map(|c| format_baht(c.summary.actual_total, 0))
                            })
                            .unwrap_or_else(|| "—".to_string())
                    }}
                </div>
                <div class="kpi-sub">
                    {move || {
                        tracking
                            .computed
                            .with(|computed| {
                                computed.as_ref().map(|c| {
                                    format!("{} รายการ", format_number(c.summary.drug_count as f64, 0))
                                })
                            })
                            .unwrap_or_default()
                    }}
                </div>
            </div>

            <div class="kpi-card kpi-card-featured">
                <div class="kpi-label">"% ความสำเร็จ"</div>
                <div class="kpi-value kpi-value-featured">
                    {move || {
                        tracking
                            .computed
                            .with(|computed| {
                                computed
                                    .as_ref()
                                    .map(|c| format_pct_opt(c.summary.achievement_pct))
                            })
                            .unwrap_or_else(|| "—".to_string())
                    }}
                </div>
                <div class="kpi-sub">{scope_label}</div>
            </div>

            <div class="kpi-card">
                <div class="kpi-label">"ล่าช้า"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .computed
                            .with(|computed| {
                                computed
                                    .as_ref()
                                    .map(|c| c.summary.behind_count.to_string())
                            })
                            .unwrap_or_else(|| "—".to_string())
                    }}
                </div>
                <div class="kpi-sub">"รายการ"</div>
            </div>

            <div class="kpi-card">
                <div class="kpi-label">"นอกแผน"</div>
                <div class="kpi-value">
                    {move || {
                        tracking
                            .computed
                            .with(|computed| {
                                computed
                                    .as_ref()
                                    .map(|c| c.summary.unplanned_count.to_string())
                            })
                            .unwrap_or_else(|| "—".to_string())
                    }}
                </div>
                <div class="kpi-sub">"รายการ"</div>
            </div>
        </div>
    }
}
