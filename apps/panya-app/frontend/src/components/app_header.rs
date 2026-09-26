//! App header: brand, connection badge, fiscal-year selector, refresh, settings.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlSelectElement};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::db_config::DbConfigContext;
use crate::contexts::tracking::TrackingContext;

/// Props for [`AppHeader`].
#[component]
pub fn AppHeader(on_open_settings: Callback<()>) -> impl IntoView {
    let db = expect_context::<DbConfigContext>();
    let tracking = expect_context::<TrackingContext>();

    let on_year_change = move |ev: Event| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(select) = target.dyn_into::<HtmlSelectElement>() else {
            return;
        };
        if let Ok(year) = select.value().parse::<i32>() {
            tracking.selected.set(None);
            tracking.year.set(year);
        }
    };

    let status_label = move || {
        if db.connecting.get() {
            "กำลังเชื่อมต่อ…"
        } else if db.connected.get() {
            "เชื่อมต่อแล้ว"
        } else if db.lost.get() {
            "การเชื่อมต่อหลุด"
        } else {
            "ยังไม่เชื่อมต่อ"
        }
    };

    view! {
        <header class="app-header">
            <div class="header-brand">
                <img class="brand-icon" src="/logo.svg" alt="PanYa" />
                <div class="brand-text">
                    <span class="brand-title">"PanYa"</span>
                    <span class="brand-sub">"ติดตามแผนจัดซื้อยา"</span>
                </div>
            </div>

            <div class="header-controls">
                // The whole label is the hit target: clicking the text opens
                // the select, not just the 32px control itself.
                <label class="year-selector">
                    <span class="year-label">"ปีงบประมาณ"</span>
                    <select
                        prop:value=move || tracking.year.get().to_string()
                        on:change=on_year_change
                    >
                        <For each=move || tracking.years.get() key=|y| *y let:year>
                            <option value=year.to_string()>{year}</option>
                        </For>
                    </select>
                </label>

                <span
                    class="badge"
                    class:badge-connected=move || db.connected.get()
                    class:badge-disconnected=move || !db.connected.get()
                >
                    <span
                        class="status-dot"
                        class:dot-green=move || db.connected.get()
                        class:dot-red=move || !db.connected.get()
                    />
                    {status_label}
                </span>

                <button
                    class="btn btn-icon"
                    title="โหลดข้อมูลใหม่"
                    aria-label="โหลดข้อมูลใหม่"
                    disabled=move || db.connecting.get() || tracking.loading.get()
                    on:click=move |_| {
                        spawn_local(async move {
                            tracking.load().await;
                        });
                    }
                >
                    <span class:spin=move || tracking.loading_visible.get()>
                        <Icon kind=IconKind::RefreshCw size=14 />
                    </span>
                </button>

                <button class="btn btn-ghost settings-btn" on:click=move |_| on_open_settings.run(())>
                    <Icon kind=IconKind::Settings size=14 />
                    "ตั้งค่า"
                </button>
            </div>
        </header>
    }
}
