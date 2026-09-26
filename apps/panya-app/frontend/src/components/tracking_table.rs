//! The tracking table: one row per drug with the scoped numbers and verdict.

use leptos::prelude::*;
use panya_core::tracking::TrackingRow;

use crate::components::status_badge::StatusBadge;
use crate::contexts::tracking::TrackingContext;
use crate::models::{format_baht, format_pct_opt};

/// Props for [`TrackingTable`] - none.
#[component]
pub fn TrackingTable() -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();
    let rows = tracking.rows();

    let empty_message = move || {
        if tracking.data.get().is_none() {
            "ยังไม่มีข้อมูล - ตรวจสอบการเชื่อมต่อ INVS ที่ปุ่ม \"ตั้งค่า\""
        } else {
            "ไม่พบรายการที่ตรงกับตัวกรอง"
        }
    };

    view! {
        <div class="table-card">
            <Show when=move || tracking.loading.get() && tracking.data.get().is_none()>
                <div class="table-message">"กำลังโหลดข้อมูล…"</div>
            </Show>

            <Show when=move || !tracking.loading.get() && rows.get().is_empty()>
                <div class="table-message">{empty_message}</div>
            </Show>

            <Show when=move || !rows.get().is_empty()>
                <div class="table-scroll">
                    <table class="tracking-table">
                        <thead>
                            <tr>
                                <th>"รหัสยา"</th>
                                <th>"ชื่อยา"</th>
                                <th class="cell-money">"แผน"</th>
                                <th class="cell-money">"ซื้อจริง"</th>
                                <th class="cell-pct">"%"</th>
                                <th class="cell-progress">"ความคืบหน้า"</th>
                                <th>"สถานะ"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For each=move || rows.get() key=|row| row.working_code.clone() let:row>
                                <TrackingRowItem
                                    row=row
                                    on_select=Callback::new(move |code: String| {
                                        tracking.selected.set(Some(code));
                                    })
                                />
                            </For>
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}

/// One table row; clicking it opens the detail drawer via `on_select`.
#[component]
fn TrackingRowItem(row: TrackingRow, on_select: Callback<String>) -> impl IntoView {
    let code = row.working_code.clone();
    let click_code = code.clone();
    let name = if row.drug_name.is_empty() {
        "-".to_string()
    } else {
        row.drug_name.clone()
    };
    let plan = row.plan_sum;
    let actual = row.actual_sum;
    let achievement = row.achievement_pct;
    let status = row.status;
    let fill_pct = achievement.unwrap_or(0.0).clamp(0.0, 100.0);
    let fill_class = format!("progress-fill status-{}", status.key());

    view! {
        <tr on:click=move |_| on_select.run(click_code.clone())>
            <td class="cell-code">{code}</td>
            <td class="cell-name">
                <span class="cell-name-text">{name}</span>
            </td>
            <td class="cell-money">{format_baht(plan, 0)}</td>
            <td class="cell-money">{format_baht(actual, 0)}</td>
            <td class="cell-pct">{format_pct_opt(achievement)}</td>
            <td class="cell-progress">
                <div class="progress-track">
                    <div class=fill_class style=format!("width:{fill_pct:.1}%")></div>
                </div>
            </td>
            <td>
                <StatusBadge status=status />
            </td>
        </tr>
    }
}
