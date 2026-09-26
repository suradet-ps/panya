//! INVS connection state and actions.
//!
//! Owns the editable connection settings, the connection state (connected /
//! connecting / lost), the save feedback, and the background health poll.
//! All backend communication goes through [`crate::services`]; this module
//! never touches `invoke` directly.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::models::InvsDbConfig;
use crate::services::commands;
use crate::services::timers::set_timeout_ms;

/// Shared connection state; every field is a plain `RwSignal`, so the
/// struct is `Copy` and can be passed to child components by value.
#[derive(Clone, Copy, Debug)]
pub struct DbConfigContext {
    /// Editable INVS connection settings.
    pub config: RwSignal<InvsDbConfig>,
    /// Whether an INVS connection is currently established.
    pub connected: RwSignal<bool>,
    /// Whether a connection attempt is in flight.
    pub connecting: RwSignal<bool>,
    /// Last connect error (displayed in the settings modal).
    pub error: RwSignal<Option<String>>,
    /// Connected at least once and the last health ping failed.
    pub lost: RwSignal<bool>,
    /// Whether a save operation is in flight.
    pub saving: RwSignal<bool>,
    /// Save feedback message, auto-cleared.
    pub save_message: RwSignal<Option<String>>,
    /// Whether the health-poll loop is running (guards double-start).
    polling: RwSignal<bool>,
    /// Whether a poll tick is still in flight.
    tick_running: RwSignal<bool>,
    /// Whether the connection was ever established (only then ping).
    ever_connected: RwSignal<bool>,
}

impl DbConfigContext {
    /// Create the signals, register them in context, and return the handle.
    #[must_use]
    pub fn provide() -> Self {
        let ctx = Self {
            config: RwSignal::new(InvsDbConfig::default()),
            connected: RwSignal::new(false),
            connecting: RwSignal::new(false),
            error: RwSignal::new(None),
            lost: RwSignal::new(false),
            saving: RwSignal::new(false),
            save_message: RwSignal::new(None),
            polling: RwSignal::new(false),
            tick_running: RwSignal::new(false),
            ever_connected: RwSignal::new(false),
        };
        provide_context(ctx);
        ctx
    }

    /// Load the persisted settings and auto-connect when they exist.
    pub async fn init_from_storage(self) {
        match commands::load_settings().await {
            Ok(settings) => {
                if let Some(cfg) = settings.invs.filter(|c| !c.host.trim().is_empty()) {
                    self.config.set(cfg);
                    let _ = self.connect().await;
                }
            }
            Err(e) => self.error.set(Some(e.message)),
        }
    }

    /// Test the connection with the current settings, storing the outcome.
    pub async fn connect(self) -> bool {
        self.connecting.set(true);
        self.error.set(None);
        let cfg = self.config.get_untracked();
        let ok = match commands::invs_connect(&cfg).await {
            Ok(()) => {
                self.ever_connected.set(true);
                self.lost.set(false);
                self.connected.set(true);
                true
            }
            Err(e) => {
                self.connected.set(false);
                self.error.set(Some(e.message));
                false
            }
        };
        self.connecting.set(false);
        ok
    }

    /// Persist the current settings (encrypted by the backend).
    pub async fn save(self) -> bool {
        self.saving.set(true);
        self.save_message.set(None);
        let cfg = self.config.get_untracked();
        let ok = match commands::save_settings(Some(&cfg)).await {
            Ok(()) => {
                self.save_message.set(Some("บันทึกการตั้งค่าสำเร็จ".to_string()));
                true
            }
            Err(e) => {
                self.save_message.set(Some(e.message));
                false
            }
        };
        self.saving.set(false);
        set_timeout_ms(move || self.save_message.set(None), 4_000);
        ok
    }

    /// Start the background health-poll loop (idempotent): a failed ping
    /// flips the badge to "lost" and reconnects from the saved settings; a
    /// successful one restores it.
    pub fn start_health_polling(self, interval_ms: i32) {
        if self.polling.get_untracked() {
            return;
        }
        self.polling.set(true);
        self.poll_tick(interval_ms);
    }

    fn poll_tick(self, interval_ms: i32) {
        set_timeout_ms(
            move || {
                spawn_local(async move {
                    if !self.tick_running.get_untracked() {
                        self.tick_running.set(true);
                        self.poll_once().await;
                        self.tick_running.set(false);
                    }
                    self.poll_tick(interval_ms);
                });
            },
            interval_ms,
        );
    }

    async fn poll_once(self) {
        if !self.ever_connected.get_untracked() {
            return;
        }
        match commands::invs_ping().await {
            Ok(()) => {
                if self.lost.get_untracked() {
                    self.lost.set(false);
                    self.connected.set(true);
                }
            }
            Err(e) => {
                if !self.lost.get_untracked() {
                    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                        "INVS connection lost: {}",
                        e.message
                    )));
                }
                self.lost.set(true);
                self.connected.set(false);
                // The single tiberius client is dead; reconnect from config.
                let cfg = self.config.get_untracked();
                if !cfg.host.trim().is_empty() && !self.connecting.get_untracked() {
                    let _ = self.connect().await;
                }
            }
        }
    }
}
