//! Service layer: Tauri IPC glue and typed command wrappers.
//!
//! [`tauri`] reaches `window.__TAURI__` (injected by the WebView), and
//! [`commands`] exposes one typed wrapper per backend command. No UI code
//! calls `invoke` directly.

pub mod commands;
pub mod tauri;
pub mod timers;
