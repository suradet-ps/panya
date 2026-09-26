//! The tracking table: one row per drug with the scoped numbers and verdict.
//!
//! Interaction contract: the header is sortable (`aria-sort` on the `th`),
//! rows are one roving tab stop (arrow keys move, Enter opens the drawer),
//! and every state - loading, empty (three variants), refreshing - has a
//! deliberate presentation.

use leptos::prelude::*;
use panya_core::tracking::TrackingRow;
use wasm_bindgen::JsCast;

use crate::components::icons::{Icon, IconKind};
use crate::contexts::db_config::DbConfigContext;
use crate::contexts::tracking::{SortKey, TrackingContext};
use crate::models::{format_baht, format_pct_opt};
use crate::services::timers::set_timeout_ms;

/// Rows rendered per event-loop batch when streaming a large list.
const RENDER_CHUNK: usize = 150;

/// Lists at or below this size render in one go: streaming them would only
/// add visible chunking to a list that was never heavy to render.
const RENDER_STREAM_THRESHOLD: usize = 300;

/// Skeleton rows shown while the first payload is in flight.
const SKELETON_ROWS: usize = 8;

/// Props for [`TrackingTable`] - none.
#[component]
pub fn TrackingTable() -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();
    let db = expect_context::<DbConfigContext>();
    let codes = tracking.visible;
    let rendered = RwSignal::new(RENDER_CHUNK);
    let generation = RwSignal::new(0_u64);
    let focused = RwSignal::new(Option::<String>::None);

    // Grow the rendered window in event-loop batches: the first paint shows
    // one chunk immediately and the rest streams in, so a large plan never
    // blocks the UI in a single synchronous render.
    Effect::new(move |_| {
        let total = codes.with(Vec::len);
        let current_generation = generation.get_untracked().wrapping_add(1);
        generation.set(current_generation);
        let initial = if total > RENDER_STREAM_THRESHOLD {
            RENDER_CHUNK
        } else {
            total
        };
        rendered.set(initial);
        grow_rows(rendered, generation, current_generation, total);
    });

    // Keep the roving tab stop on a row that still exists.
    Effect::new(move |_| {
        let list = codes.get();
        let current = focused.get_untracked();
        if current.as_ref().is_none_or(|code| !list.contains(code)) {
            focused.set(list.first().cloned());
        }
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

    // Arrow keys move the roving focus; Enter/Space opens the drawer.
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(row) = target.dyn_into::<web_sys::HtmlElement>() else {
            return;
        };
        let Some(code) = row.get_attribute("data-code") else {
            return;
        };
        let list = codes.get_untracked();
        let Some(index) = list.iter().position(|candidate| *candidate == code) else {
            return;
        };
        let next = match ev.key().as_str() {
            "ArrowDown" => list.get(index + 1).cloned(),
            "ArrowUp" => index.checked_sub(1).and_then(|i| list.get(i)).cloned(),
            "Home" => list.first().cloned(),
            "End" => list.last().cloned(),
            "Enter" | " " => {
                ev.prevent_default();
                tracking.selected.set(Some(code.clone()));
                None
            }
            _ => None,
        };
        if let Some(next_code) = next {
            ev.prevent_default();
            focus_row(&next_code);
        }
    };

    let empty_state = move || {
        if !db.connected.get() {
            view! {
                <div class="table-empty">
                    <Icon kind=IconKind::PlugZap size=32 />
                    <div class="table-empty-title">"ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS"</div>
                    <div class="table-empty-sub">"ตั้งค่าการเชื่อมต่อเพื่อเริ่มติดตามแผนจัดซื้อยา"</div>
                    <button
                        class="btn btn-primary"
                        on:click=move |_| db.settings_open.set(true)
                    >
                        "ตั้งค่าการเชื่อมต่อ"
                    </button>
                </div>
            }
            .into_any()
        } else if tracking.filters_active().get() {
            view! {
                <div class="table-empty">
                    <Icon kind=IconKind::Search size=32 />
                    <div class="table-empty-title">"ไม่พบรายการที่ตรงกับตัวกรอง"</div>
                    <div class="table-empty-sub">"ลองล้างตัวกรองหรือแก้คำค้นหา"</div>
                    <button class="btn btn-secondary" on:click=move |_| tracking.clear_filters()>
                        "ล้างตัวกรอง"
                    </button>
                </div>
            }
            .into_any()
        } else {
            view! {
                <div class="table-empty">
                    <Icon kind=IconKind::Inbox size=32 />
                    <div class="table-empty-title">"ไม่พบแผนจัดซื้อในปีงบประมาณนี้"</div>
                    <div class="table-empty-sub">
                        "ตรวจสอบปีงบประมาณที่เลือก หรือข้อมูล BUYPLAN_C ใน INVS"
                    </div>
                </div>
            }
            .into_any()
        }
    };

    view! {
        <div class="table-card">
            <Show when=move || {
                tracking.loading_visible.get() && tracking.data.with(|data| data.is_some())
            }>
                <div class="loadbar" aria-hidden="true"></div>
            </Show>

            <Show when=move || {
                tracking.loading_visible.get() && tracking.data.with(|data| data.is_none())
            }>
                <div class="table-skeleton" aria-busy="true" aria-label="กำลังโหลดข้อมูล">
                    <table class="tracking-table">
                        <thead>
                            <tr>
                                <th scope="col">"รหัสยา"</th>
                                <th scope="col">"ชื่อยา"</th>
                                <th scope="col" class="cell-money">"แผน"</th>
                                <th scope="col" class="cell-money">"ซื้อจริง"</th>
                                <th scope="col" class="cell-pct">"%"</th>
                                <th scope="col" class="cell-progress">"ความคืบหน้า"</th>
                                <th scope="col">"สถานะ"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {(0..SKELETON_ROWS)
                                .map(|_| {
                                    view! {
                                        <tr>
                                            {(0..7)
                                                .map(|_| {
                                                    view! {
                                                        <td>
                                                            <span class="skeleton-line"></span>
                                                        </td>
                                                    }
                                                })
                                                .collect_view()}
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
            </Show>

            <Show when=move || {
                codes.get().is_empty()
                    && !(tracking.loading_visible.get()
                        && tracking.data.with(|data| data.is_none()))
            }>
                <div class="table-message">{empty_state}</div>
            </Show>

            <Show when=move || !codes.get().is_empty()>
                <div
                    class="table-scroll"
                    class:table-scroll--stale=move || tracking.loading_visible.get()
                >
                    <table class="tracking-table">
                        <thead>
                            <tr>
                                <th scope="col">"รหัสยา"</th>
                                <SortHeader label="ชื่อยา" key=SortKey::Name class="cell-name" />
                                <SortHeader label="แผน" key=SortKey::Plan class="cell-money" />
                                <SortHeader label="ซื้อจริง" key=SortKey::Actual class="cell-money" />
                                <SortHeader label="%" key=SortKey::Pct class="cell-pct" />
                                <th scope="col" class="cell-progress">"ความคืบหน้า"</th>
                                <SortHeader label="สถานะ" key=SortKey::Status />
                            </tr>
                        </thead>
                        <tbody on:keydown=on_keydown>
                            <For each=visible key=|code| code.clone() let:code>
                                <TrackingRowItem
                                    code=code
                                    focused=focused
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

/// A sortable column header: a real button, with `aria-sort` on the `th`.
#[component]
fn SortHeader(
    label: &'static str,
    key: SortKey,
    #[prop(into, default = String::new())] class: String,
) -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();
    let active = Memo::new(move |_| tracking.sort.get() == key);
    let ascending = move || tracking.sort_asc.get();

    let on_click = move |_| {
        if active.get_untracked() {
            tracking.sort_asc.update(|asc| *asc = !*asc);
        } else {
            tracking.sort.set(key);
            // Names start ascending; numbers and severity start descending.
            tracking.sort_asc.set(matches!(key, SortKey::Name));
        }
    };

    view! {
        <th
            scope="col"
            class=class
            aria-sort=move || {
                if active.get() {
                    if ascending() { "ascending" } else { "descending" }
                } else {
                    "none"
                }
            }
        >
            <button class="th-sort" class:th-sort-active=move || active.get() on:click=on_click>
                {label}
                <span class="th-sort-icon" aria-hidden="true">
                    {move || if active.get() { if ascending() { "▲" } else { "▼" } } else { "↕" }}
                </span>
            </button>
        </th>
    }
}

/// One table row; clicking it opens the detail drawer via `on_select`.
///
/// The row is keyed by `working_code` and every cell reads its field through
/// [`TrackingContext::with_row`], so a reload updates the text nodes in place
/// instead of rebuilding the table. The row is the table's single roving tab
/// stop (`focused`).
#[component]
fn TrackingRowItem(
    code: String,
    focused: RwSignal<Option<String>>,
    on_select: Callback<String>,
) -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();

    let data_code = code.clone();
    let click_code = code.clone();
    let focus_code = code.clone();
    let tab_code = code.clone();
    let code_text = code.clone();
    let tabindex = move || {
        if focused.get().as_deref() == Some(tab_code.as_str()) {
            "0"
        } else {
            "-1"
        }
    };

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
        <tr
            data-code=data_code
            tabindex=tabindex
            on:click=move |_| on_select.run(click_code.clone())
            on:focus=move |_| focused.set(Some(focus_code.clone()))
        >
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

/// Move DOM focus to the row for `code` (roving tabindex).
fn focus_row(code: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let selector = format!("tr[data-code=\"{code}\"]");
    if let Ok(Some(element)) = document.query_selector(&selector)
        && let Ok(row) = element.dyn_into::<web_sys::HtmlElement>()
    {
        let _ = row.focus();
    }
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
