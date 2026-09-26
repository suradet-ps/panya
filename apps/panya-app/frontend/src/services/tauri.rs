//! Low-level Tauri global IPC bindings.
//!
//! The frontend is built by Trunk with no JS bundler, so it cannot `import`
//! the `@tauri-apps/api` npm module. Instead `withGlobalTauri` is enabled in
//! `tauri.conf.json`, which makes the WebView inject `window.__TAURI__`
//! synchronously - no CDN, no network dependency, the app boots offline on a
//! hospital LAN. These helpers wrap the raw `web-sys` calls so the rest of
//! the app never touches the DOM.

use js_sys::{Function, Promise, Reflect};
use serde::de::DeserializeOwned;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

/// Error returned when the Tauri global API cannot be located.
fn no_tauri() -> JsValue {
    JsValue::from_str("Tauri global API is not available")
}

/// Return the `window` object once `window.__TAURI__` is present (injected
/// synchronously by `withGlobalTauri`).
async fn get_tauri() -> Result<JsValue, JsValue> {
    let win = window().ok_or_else(no_tauri)?;
    let existing = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
    if existing.is_undefined() || existing.is_null() {
        return Err(no_tauri());
    }
    Ok(JsValue::from(win))
}

/// Resolve `window.__TAURI__.core` (the namespace that hosts `invoke`).
async fn core_namespace() -> Result<JsValue, JsValue> {
    let win = get_tauri().await?;
    let tauri = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
    Reflect::get(&tauri, &JsValue::from_str("core"))
}

/// Invoke a Tauri command and decode its JSON result.
///
/// `args` is the serialised argument object (`{}` for argument-less commands).
/// On a rejected promise the underlying [`JsValue`] is returned so the caller
/// can translate it into a domain error.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: &JsValue) -> Result<T, JsValue> {
    let core = core_namespace().await?;
    let invoke_fn = Reflect::get(&core, &JsValue::from_str("invoke"))?
        .dyn_into::<Function>()
        .map_err(|_| JsValue::from_str("invoke is not a function"))?;

    let promise = invoke_fn
        .call2(&core, &JsValue::from_str(cmd), args)
        .map_err(|e| JsValue::from_str(&format!("failed to call {cmd}: {e:?}")))?;

    let result = JsFuture::from(promise.unchecked_into::<Promise>())
        .await
        .map_err(|e| JsValue::from_str(&format!("{cmd} rejected: {e:?}")))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| JsValue::from_str(&format!("failed to decode {cmd} result: {e:?}")))
}

/// Build a JSON argument object from `(name, value)` pairs.
#[must_use]
pub fn build_args(pairs: &[(&str, &JsValue)]) -> JsValue {
    let obj = js_sys::Object::new();
    for (name, value) in pairs {
        let _ = Reflect::set(&obj, &JsValue::from_str(name), value);
    }
    obj.into()
}
