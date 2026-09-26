//! The drug detail drawer: scoped numbers plus the quarter breakdown.

use leptos::prelude::*;
use panya_core::fiscal::quarter_label;

use crate::components::icons::{Icon, IconKind};
use crate::components::status_badge::StatusBadge;
use crate::contexts::tracking::TrackingContext;
use crate::models::{format_baht, format_pct_opt};

/// Props for [`DrugDetailDrawer`] - none (reads the selection from context).
#[component]
pub fn DrugDetailDrawer() -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();
    let selected = tracking.selected_row();

    let close = move |_| tracking.selected.set(None);

    view! {
        <Show when=move || selected.get().is_some()>
            {move || {
                selected
                    .get()
                    .map(|row| {
                        let name = if row.drug_name.is_empty() {
                            row.working_code.clone()
                        } else {
                            row.drug_name.clone()
                        };
                        let code = row.working_code.clone();
                        let status = row.status;
                        let plan_sum = row.plan_sum;
                        let actual_sum = row.actual_sum;
                        let remaining = (plan_sum - actual_sum).max(0.0);
                        let achievement = row.achievement_pct;
                        let max = row
                            .plan
                            .iter()
                            .chain(row.actual.iter())
                            .fold(0.0_f64, |acc, value| acc.max(*value));
                        let quarter_data: Vec<(u8, f64, f64)> = (1u8..=4)
                            .map(|q| {
                                let idx = usize::from(q - 1);
                                (q, row.plan[idx], row.actual[idx])
                            })
                            .collect();

                        view! {
                            <aside class="drawer">
                                <div class="drawer-header">
                                    <div class="drawer-heading">
                                        <div class="drawer-title">{name}</div>
                                        <div class="drawer-code">{code}</div>
                                    </div>
                                    <div class="drawer-actions">
                                        <StatusBadge status=status />
                                        <button class="btn btn-icon" title="ปิด" on:click=close>
                                            <Icon kind=IconKind::X size=14 />
                                        </button>
                                    </div>
                                </div>

                                <div class="drawer-section">
                                    <div class="drawer-label">"สรุปตามช่วงที่เลือก"</div>
                                    <div class="drawer-numbers">
                                        <div class="drawer-number">
                                            <span class="dn-label">"แผน"</span>
                                            <span class="dn-value">{format_baht(plan_sum, 0)}</span>
                                        </div>
                                        <div class="drawer-number">
                                            <span class="dn-label">"ซื้อจริง"</span>
                                            <span class="dn-value">{format_baht(actual_sum, 0)}</span>
                                        </div>
                                        <div class="drawer-number">
                                            <span class="dn-label">"คงเหลือ"</span>
                                            <span class="dn-value">{format_baht(remaining, 0)}</span>
                                        </div>
                                        <div class="drawer-number">
                                            <span class="dn-label">"% ความสำเร็จ"</span>
                                            <span class="dn-value">{format_pct_opt(achievement)}</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="drawer-section">
                                    <div class="drawer-label">"รายไตรมาส (แผน vs ซื้อจริง)"</div>
                                    <div class="quarter-legend">
                                        <span class="legend-swatch legend-plan"></span>
                                        <span>"แผน"</span>
                                        <span class="legend-swatch legend-actual"></span>
                                        <span>"ซื้อจริง"</span>
                                    </div>
                                    <For each=move || quarter_data.clone() key=|(q, _, _)| *q let:item>
                                        <QuarterBars q=item.0 plan=item.1 actual=item.2 max=max />
                                    </For>
                                </div>
                            </aside>
                        }
                    })
            }}
        </Show>
    }
}

/// One quarter's plan/actual pair as two horizontal bars.
#[component]
fn QuarterBars(q: u8, plan: f64, actual: f64, max: f64) -> impl IntoView {
    let width = |value: f64| {
        if max > 0.0 {
            (value / max * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    };
    let plan_width = width(plan);
    let actual_width = width(actual);

    view! {
        <div class="quarter-row">
            <div class="quarter-label">
                <span class="quarter-q">{format!("Q{q}")}</span>
                <span class="quarter-range">{quarter_label(q)}</span>
            </div>
            <div class="quarter-bars">
                <div class="bar-track">
                    <div class="bar-fill bar-plan" style=format!("width:{plan_width:.1}%")></div>
                </div>
                <div class="bar-track">
                    <div class="bar-fill bar-actual" style=format!("width:{actual_width:.1}%")></div>
                </div>
            </div>
            <div class="quarter-values">
                <span class="qv">{format_baht(plan, 0)}</span>
                <span class="qv">{format_baht(actual, 0)}</span>
            </div>
        </div>
    }
}
