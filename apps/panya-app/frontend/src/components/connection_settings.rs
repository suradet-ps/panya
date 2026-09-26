//! The INVS connection settings modal.
//!
//! First launch opens it automatically. **Test** validates and connects,
//! reporting the round-trip latency; **Save** validates, connects, and then
//! persists the settings encrypted. Escape closes it from anywhere.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement};

use crate::components::icons::{Icon, IconKind};
use crate::contexts::db_config::DbConfigContext;

/// Props for [`ConnectionSettings`].
#[component]
pub fn ConnectionSettings(visible: RwSignal<bool>, on_close: Callback<()>) -> impl IntoView {
    let db = expect_context::<DbConfigContext>();

    let close = move || on_close.run(());

    // Escape closes from anywhere, matching the sibling apps.
    let escape_handle = window_event_listener(ev::keydown, move |ev| {
        if ev.key() == "Escape" && visible.get_untracked() {
            close();
        }
    });
    let _escape_handle = StoredValue::new(escape_handle);

    let text_handler = move |set: fn(&mut crate::models::InvsDbConfig, String)| {
        move |ev: Event| {
            let Some(target) = ev.target() else {
                return;
            };
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                db.config.update(|cfg| set(cfg, input.value()));
            }
        }
    };

    let on_host = text_handler(|cfg, value| cfg.host = value);
    let on_port = text_handler(|cfg, value| cfg.port = value);
    let on_database = text_handler(|cfg, value| cfg.database = value);
    let on_user = text_handler(|cfg, value| cfg.user = value);
    let on_password = text_handler(|cfg, value| cfg.password = value);
    let on_instance = text_handler(|cfg, value| cfg.instance = Some(value));

    let test = move |_| {
        spawn_local(async move {
            let _ = db.connect().await;
        });
    };

    let save = move |_| {
        spawn_local(async move {
            let _ = db.save_and_connect().await;
        });
    };

    let status_label = move || {
        if db.connected.get() {
            "เชื่อมต่อแล้ว"
        } else {
            "ยังไม่ได้เชื่อมต่อ"
        }
    };

    let busy = move || db.connecting.get() || db.saving.get();

    view! {
        <Show when=move || visible.get()>
            <div class="modal-backdrop" on:click=move |_| close()>
                <div
                    class="modal"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="settings-title"
                    on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                >
                    <div class="modal-header">
                        <div>
                            <div class="modal-title" id="settings-title">"ตั้งค่าการเชื่อมต่อ INVS"</div>
                            <div class="modal-sub">
                                "อ่านข้อมูลเท่านั้น (read-only) - รหัสผ่านถูกเข้ารหัสก่อนบันทึก"
                                <span class="modal-status">{status_label}</span>
                            </div>
                        </div>
                        <button class="btn btn-icon" title="ปิด" on:click=move |_| close()>
                            <Icon kind=IconKind::X size=14 />
                        </button>
                    </div>

                    <div class="form-grid">
                        <label class="field">
                            <span class="field-label">"Host"</span>
                            <input
                                class="input"
                                type="text"
                                autofocus=true
                                prop:value=move || db.config.get().host
                                on:input=on_host
                            />
                        </label>
                        <label class="field">
                            <span class="field-label">"Port"</span>
                            <input
                                class="input"
                                type="text"
                                prop:value=move || db.config.get().port
                                on:input=on_port
                            />
                        </label>
                        <label class="field">
                            <span class="field-label">"Database"</span>
                            <input
                                class="input"
                                type="text"
                                prop:value=move || db.config.get().database
                                on:input=on_database
                            />
                        </label>
                        <label class="field">
                            <span class="field-label">"Instance (ถ้ามี)"</span>
                            <input
                                class="input"
                                type="text"
                                prop:value=move || db.config.get().instance.clone().unwrap_or_default()
                                on:input=on_instance
                            />
                        </label>
                        <label class="field">
                            <span class="field-label">"User"</span>
                            <input
                                class="input"
                                type="text"
                                prop:value=move || db.config.get().user
                                on:input=on_user
                            />
                        </label>
                        <label class="field">
                            <span class="field-label">"Password"</span>
                            <input
                                class="input"
                                type="password"
                                prop:value=move || db.config.get().password
                                on:input=on_password
                            />
                        </label>
                    </div>

                    <Show when=move || db.error.get().is_some()>
                        <div class="message message-error" role="alert">
                            <Icon kind=IconKind::AlertTriangle size=14 />
                            <span>{move || db.error.get().unwrap_or_default()}</span>
                        </div>
                    </Show>

                    <Show when=move || db.connected.get() && db.latency_ms.get().is_some()>
                        <div class="message message-success" role="status">
                            {move || {
                                db.latency_ms
                                    .get()
                                    .map(|ms| format!("เชื่อมต่อได้ (ใช้เวลา {ms} ms)"))
                                    .unwrap_or_default()
                            }}
                        </div>
                    </Show>

                    <Show when=move || db.save_message.get().is_some()>
                        <div class="message message-success" role="status">
                            {move || db.save_message.get().unwrap_or_default()}
                        </div>
                    </Show>

                    <div class="modal-actions">
                        <button class="btn btn-secondary" disabled=busy on:click=test>
                            <Icon kind=IconKind::PlugZap size=14 />
                            {move || if db.connecting.get() { "กำลังทดสอบ…" } else { "ทดสอบการเชื่อมต่อ" }}
                        </button>
                        <button class="btn btn-primary" disabled=busy on:click=save>
                            <Icon kind=IconKind::Save size=14 />
                            {move || if db.saving.get() { "กำลังบันทึก…" } else { "บันทึกและเชื่อมต่อ" }}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
