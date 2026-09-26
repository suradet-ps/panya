//! Filter bar: quarter scope, status filter, search, and the result count.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::tracking::{StatusFilter, TrackingContext};

/// Props for [`FilterBar`] - none.
#[component]
pub fn FilterBar() -> impl IntoView {
    let tracking = expect_context::<TrackingContext>();

    let on_search = move |ev: Event| {
        let Some(target) = ev.target() else {
            return;
        };
        if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
            tracking.search_input.set(input.value());
        }
    };

    view! {
        <div class="filter-bar">
            <div class="pill-group" role="group" aria-label="ช่วงไตรมาส">
                <For each=|| [0u8, 1, 2, 3, 4] key=|q| *q let:q>
                    <button
                        type="button"
                        class="pill"
                        class:pill-active=move || tracking.quarter.get() == q
                        title=panya_core::fiscal::quarter_label(q)
                        on:click=move |_| {
                            tracking.selected.set(None);
                            tracking.quarter.set(q);
                        }
                    >
                        {if q == 0 { "ทั้งปี".to_string() } else { format!("Q{q}") }}
                    </button>
                </For>
            </div>

            <div class="pill-group pill-group--muted" role="group" aria-label="สถานะ">
                <For each=|| StatusFilter::OPTIONS key=|f| f.key() let:filter>
                    <button
                        type="button"
                        class="pill"
                        class:pill-active=move || tracking.status_filter.get() == filter
                        on:click=move |_| tracking.status_filter.set(filter)
                    >
                        {filter.label_th()}
                    </button>
                </For>
            </div>

            <Show when=move || tracking.loading_visible.get()>
                <span class="loading-hint">"กำลังโหลด…"</span>
            </Show>

            <Show when=move || {
                !tracking.loading_visible.get() && tracking.data.with(|data| data.is_some())
            }>
                <span class="loading-hint">
                    {move || {
                        let count = tracking.visible.with(Vec::len);
                        tracking
                            .data
                            .with(|data| {
                                data.as_ref().map(|d| format!("{count} รายการ · โหลด {} ms", d.elapsed_ms))
                            })
                            .unwrap_or_default()
                    }}
                </span>
            </Show>

            <div class="search-box">
                <Icon kind=IconKind::Search size=14 />
                <input
                    class="input search-input"
                    type="search"
                    placeholder="ค้นหารหัสยา / ชื่อยา…"
                    prop:value=move || tracking.search_input.get()
                    on:input=on_search
                />
            </div>
        </div>
    }
}
