//! The tracking table: one row per drug with the scoped numbers and verdict.

use leptos::prelude::*;
use panya_core::tracking::TrackingRow;

use crate::contexts::tracking::TrackingContext;
use crate::models::{format_baht, format_pct_opt};
use crate::services::timers::set_timeout_ms;

/// Rows rendered per event-loop batch on the first paint.
const RENDER_CHUNK: usize = 150;

/// Props for [`TrackingTable`] - none.
#[component]
pub fn TrackingTable() -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();
    let codes = tracking.visible_codes();
    let rendered = RwSignal::new(RENDER_CHUNK);
    let generation = RwSignal::new(0_u64);

    // Grow the rendered window in event-loop batches: the first paint shows
    // one chunk immediately and the rest streams in, so a large plan never
    // blocks the UI in a single synchronous render.
    Effect::new(move |_| {
        let total = codes.with(Vec::len);
        let current_generation = generation.get_untracked().wrapping_add(1);
        generation.set(current_generation);
        rendered.set(RENDER_CHUNK.min(total));
        grow_rows(rendered, generation, current_generation, total);
    });

    // The slice of codes currently mounted in the table.
    let visible = move || {
        let limit = rendered.get();
        codes.with(|codes| {
            if limit >= codes.len() {
                codes.clone()
            } else {
                codes[..limit].to_vec()
            }
        })
    };

    let empty_message = move || {
        if tracking.data.with(|data| data.is_none()) {
            "ยังไม่มีข้อมูล - ตรวจสอบการเชื่อมต่อ INVS ที่ปุ่ม \"ตั้งค่า\""
        } else {
            "ไม่พบรายการที่ตรงกับตัวกรอง"
        }
    };

    view! {
        <div class="table-card">
            <Show when=move || tracking.loading.get() && tracking.data.with(|data| data.is_none())>
                <div class="table-message">"กำลังโหลดข้อมูล…"</div>
            </Show>

            <Show when=move || !tracking.loading.get() && codes.get().is_empty()>
                <div class="table-message">{empty_message}</div>
            </Show>

            <Show when=move || !codes.get().is_empty()>
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
                            <For each=visible key=|code| code.clone() let:code>
                                <TrackingRowItem
                                    code=code
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
///
/// The row is keyed by `working_code` and every cell reads its field through
/// [`TrackingContext::with_row`], so a reload updates the text nodes in place
/// instead of rebuilding the table.
#[component]
fn TrackingRowItem(code: String, on_select: Callback<String>) -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();

    let click_code = code.clone();
    let code_text = code.clone();
    let name = cell(tracking, code.clone(), |row| {
        if row.drug_name.is_empty() {
            "—".to_string()
        } else {
            row.drug_name.clone()
        }
    });
    let plan = cell(tracking, code.clone(), |row| format_baht(row.plan_sum, 0));
    let actual = cell(tracking, code.clone(), |row| format_baht(row.actual_sum, 0));
    let pct = cell(tracking, code.clone(), |row| {
        format_pct_opt(row.achievement_pct)
    });

    let progress_class = {
        let code = code.clone();
        move || {
            tracking.with_row(&code, |row| {
                row.map_or_else(String::new, |row| {
                    format!("progress-fill status-{}", row.status.key())
                })
            })
        }
    };
    let progress_width = {
        let code = code.clone();
        move || {
            tracking.with_row(&code, |row| {
                row.map_or(0.0, |row| {
                    row.achievement_pct.unwrap_or(0.0).clamp(0.0, 100.0)
                })
            })
        }
    };

    let badge_class = {
        let code = code.clone();
        move || {
            tracking.with_row(&code, |row| {
                row.map_or_else(String::new, |row| {
                    format!("status-badge status-{}", row.status.key())
                })
            })
        }
    };
    let badge_label = {
        let code = code.clone();
        move || {
            tracking.with_row(&code, |row| {
                row.map_or_else(|| "—".to_string(), |row| row.status.label_th().to_string())
            })
        }
    };

    view! {
        <tr on:click=move |_| on_select.run(click_code.clone())>
            <td class="cell-code">{code_text}</td>
            <td class="cell-name">
                <span class="cell-name-text">{name}</span>
            </td>
            <td class="cell-money">{plan}</td>
            <td class="cell-money">{actual}</td>
            <td class="cell-pct">{pct}</td>
            <td class="cell-progress">
                <div class="progress-track">
                    <div
                        class=progress_class
                        style=move || format!("width:{:.1}%", progress_width())
                    ></div>
                </div>
            </td>
            <td>
                <span class=badge_class>{badge_label}</span>
            </td>
        </tr>
    }
}

/// A reactive cell: formats one field of the row for `code` (or `—`).
fn cell(
    tracking: TrackingContext,
    code: String,
    format: fn(&TrackingRow) -> String,
) -> impl Fn() -> String {
    move || tracking.with_row(&code, |row| row.map_or_else(|| "—".to_string(), format))
}

/// Schedule the next batch of rows for the current generation; stale chains
/// (a newer filter/scope change) stop at the generation check.
fn grow_rows(
    rendered: RwSignal<usize>,
    generation: RwSignal<u64>,
    current_generation: u64,
    total: usize,
) {
    if rendered.get_untracked() >= total {
        return;
    }
    set_timeout_ms(
        move || {
            if generation.get_untracked() != current_generation {
                return;
            }
            rendered.update(|limit| *limit = (*limit + RENDER_CHUNK).min(total));
            grow_rows(rendered, generation, current_generation, total);
        },
        0,
    );
}
