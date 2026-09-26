//! Root application view.
//!
//! Provides the shared contexts, boots the connection from persisted
//! settings, keeps the year/quarter scope in sync with the backend, and
//! composes the header, KPI strip, filter bar, table, and detail drawer.

use std::cell::Cell;
use std::rc::Rc;

use leptos::html::Div;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::app_header::AppHeader;
use crate::components::connection_settings::ConnectionSettings;
use crate::components::drug_detail_drawer::DrugDetailDrawer;
use crate::components::filter_bar::FilterBar;
use crate::components::icons::{Icon, IconKind};
use crate::components::summary_kpi_bar::SummaryKpiBar;
use crate::components::tracking_table::TrackingTable;
use crate::contexts::db_config::DbConfigContext;
use crate::contexts::tracking::TrackingContext;
use crate::models::current_fiscal_year;

/// The root component of the frontend.
#[component]
pub fn App() -> impl IntoView {
    let db = DbConfigContext::provide();
    let tracking = TrackingContext::provide();

    // Boot: load persisted settings (auto-connects) and start the health-poll
    // loop. The connection watcher below owns the year list and the first
    // load, so nothing is fetched twice. The first launch has no settings, so
    // the modal opens immediately.
    let root_ref = NodeRef::<Div>::new();
    root_ref.on_load(move |_| {
        spawn_local(async move {
            // The window starts hidden; reveal it now that the shell is up.
            let _ = crate::services::commands::show_main_window().await;
            db.init_from_storage().await;
            if !db.connected.get_untracked() {
                db.settings_open.set(true);
            }
            db.start_health_polling(15_000);
        });
    });

    // First successful connection (boot auto-connect or a Test in the modal)
    // → load the year list and the first payload. Setting the year triggers
    // the scope effect below; otherwise this loads directly - exactly one
    // load either way.
    let was_connected = Rc::new(Cell::new(false));
    Effect::new(move |_| {
        let connected = db.connected.get();
        if connected && !was_connected.replace(connected) {
            spawn_local(async move {
                let years = tracking.fetch_years().await;
                let current = tracking.year.get_untracked();
                if years.is_empty() {
                    // No plan years in INVS yet: fall back to the clock.
                    tracking.year.set(current_fiscal_year());
                } else if !years.contains(&current) {
                    tracking.year.set(years[0]);
                } else {
                    tracking.load().await;
                }
            });
        }
    });

    // Year change → reload (only while connected; a disconnected app already
    // shows its own banner and error). Quarter changes do not reload: the
    // shared engine recomputes the verdicts from the same payload instantly.
    Effect::new(move |_| {
        let _ = tracking.year.get();
        if db.connected.get_untracked() {
            spawn_local(async move {
                tracking.load().await;
            });
        }
    });

    let reconnect = move |_| {
        spawn_local(async move {
            let _ = db.connect().await;
        });
    };

    let retry_load = move |_| {
        spawn_local(async move {
            tracking.load().await;
        });
    };

    view! {
        <div class="app-shell" node_ref=root_ref>
            <AppHeader on_open_settings=Callback::new(move |_| db.settings_open.set(true)) />

            <Show when=move || tracking.error.get().is_some()>
                <div class="error-banner" role="alert">
                    <Icon kind=IconKind::AlertTriangle size=14 />
                    <span>{move || tracking.error.get().unwrap_or_default()}</span>
                    <Show when=move || db.connected.get()>
                        <button class="btn btn-ghost" on:click=retry_load>
                            "ลองใหม่"
                        </button>
                    </Show>
                    <button
                        class="btn-dismiss"
                        title="ปิดข้อความ"
                        aria-label="ปิดข้อความ"
                        on:click=move |_| tracking.error.set(None)
                    >
                        <Icon kind=IconKind::X size=12 />
                    </button>
                </div>
            </Show>

            <Show when=move || db.lost.get()>
                <div class="lost-banner">
                    <Icon kind=IconKind::AlertTriangle size=14 />
                    <span>"การเชื่อมต่อ INVS หลุด - ระบบจะพยายามเชื่อมต่อใหม่โดยอัตโนมัติ"</span>
                    <button class="btn btn-ghost" on:click=reconnect>
                        "เชื่อมต่อใหม่"
                    </button>
                </div>
            </Show>

            <Show when=move || !db.connected.get() && !db.connecting.get() && !db.lost.get()>
                <div class="no-conn-banner">
                    <Icon kind=IconKind::PlugZap size=14 />
                    "ยังไม่ได้เชื่อมต่อฐานข้อมูล INVS -"
                    <button class="link-btn" on:click=move |_| db.settings_open.set(true)>
                        "คลิกเพื่อตั้งค่าการเชื่อมต่อ"
                    </button>
                </div>
            </Show>

            <main class="main-content">
                <SummaryKpiBar />
                <FilterBar />
                <div class="workspace">
                    <TrackingTable />
                    <DrugDetailDrawer />
                </div>
            </main>

            <ConnectionSettings
                visible=db.settings_open
                on_close=Callback::new(move |_| db.settings_open.set(false))
            />

            // Screen-reader narration for the async states (WCAG 4.1.3).
            <div class="sr-only" role="status" aria-live="polite">
                {move || {
                    if tracking.loading_visible.get() {
                        "กำลังโหลดข้อมูล".to_string()
                    } else if tracking.data.with(|data| data.is_some()) {
                        format!("โหลดข้อมูลแล้ว {} รายการ", tracking.visible.with(Vec::len))
                    } else {
                        String::new()
                    }
                }}
            </div>
        </div>
    }
}
