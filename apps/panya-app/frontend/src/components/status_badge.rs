//! The per-row verdict chip.

use leptos::prelude::*;
use panya_core::tracking::Status;

/// Render a verdict as a tinted pill; the Thai label always accompanies the
/// color (the color is never the only signal).
#[component]
pub fn StatusBadge(status: Status) -> impl IntoView {
    view! {
        <span class=format!("status-badge status-{}", status.key())>{status.label_th()}</span>
    }
}
