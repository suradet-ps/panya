//! The INVS connection settings modal.
//!
//! Presentational: the form edits `DbConfigContext::config`; Test connects,
//! Save persists (encrypted). A missing config opens this modal
//! automatically on first launch.

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
            let _ = db.save().await;
        });
    };

    view! {
        <Show when=move || visible.get()>
            <div class="modal-backdrop" on:click=move |_| on_close.run(())>
                <div class="modal" on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()>
                    <div class="modal-header">
                        <div>
                            <div class="modal-title">"ตั้งค่าการเชื่อมต่อ INVS"</div>
                            <div class="modal-sub">"อ่านข้อมูลเท่านั้น (read-only) - รหัสผ่านถูกเข้ารหัสก่อนบันทึก"</div>
                        </div>
                        <button class="btn btn-icon" title="ปิด" on:click=move |_| on_close.run(())>
                            <Icon kind=IconKind::X size=14 />
                        </button>
                    </div>

                    <div class="form-grid">
                        <label class="field">
                            <span class="field-label">"Host"</span>
                            <input
                                class="input"
                                type="text"
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
                        <div class="message message-error">
                            <Icon kind=IconKind::AlertTriangle size=14 />
                            <span>{move || db.error.get().unwrap_or_default()}</span>
                        </div>
                    </Show>

                    <Show when=move || db.save_message.get().is_some()>
                        <div class="message message-success">{move || db.save_message.get().unwrap_or_default()}</div>
                    </Show>

                    <div class="modal-actions">
                        <button class="btn btn-secondary" disabled=move || db.connecting.get() on:click=test>
                            <Icon kind=IconKind::PlugZap size=14 />
                            {move || if db.connecting.get() { "กำลังทดสอบ…" } else { "ทดสอบการเชื่อมต่อ" }}
                        </button>
                        <button class="btn btn-primary" disabled=move || db.saving.get() on:click=save>
                            <Icon kind=IconKind::Save size=14 />
                            {move || if db.saving.get() { "กำลังบันทึก…" } else { "บันทึก" }}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
