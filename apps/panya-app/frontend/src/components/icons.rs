//! Inline SVG icons (lucide-style, MIT path data).
//!
//! Re-implemented as inline SVGs with no JS dependency: 24×24 view box,
//! `currentColor` strokes. Keep the set minimal - one icon per real need.

use leptos::prelude::*;

/// A single shape inside an icon.
enum Shape {
    Path(&'static str),
    Circle(f64, f64, f64),
}

/// Which icon to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    AlertTriangle,
    PlugZap,
    Save,
    Search,
    Settings,
    X,
}

fn shapes(kind: IconKind) -> &'static [Shape] {
    match kind {
        IconKind::AlertTriangle => &[
            Shape::Path(
                "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z",
            ),
            Shape::Path("M12 9v4"),
            Shape::Path("M12 17h.01"),
        ],
        IconKind::PlugZap => &[
            Shape::Path("M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18"),
            Shape::Path("M13.5 2.5 4 12h6l-1.5 8 9.5-9.5H12l1.5-8z"),
        ],
        IconKind::Save => &[
            Shape::Path(
                "M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z",
            ),
            Shape::Path("M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7"),
            Shape::Path("M7 3v4a1 1 0 0 0 1 1h7"),
        ],
        IconKind::Search => &[
            Shape::Path("m21 21-4.3-4.3"),
            Shape::Circle(11.0, 11.0, 8.0),
        ],
        IconKind::Settings => &[
            Shape::Path(
                "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z",
            ),
            Shape::Circle(12.0, 12.0, 3.0),
        ],
        IconKind::X => &[Shape::Path("M18 6 6 18"), Shape::Path("m6 6 12 12")],
    }
}

/// Render a lucide-style icon at `size` pixels, stroked with `currentColor`.
#[component]
pub fn Icon(
    kind: IconKind,
    #[prop(into, default = String::new())] class: String,
    #[prop(default = 14)] size: u16,
) -> impl IntoView {
    let icon_shapes = shapes(kind);
    view! {
        // Note: SVG-element attributes are passed verbatim by the Leptos
        // view! macro, so the exact DOM spellings (`viewBox`, `stroke-width`)
        // must be used here.
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=class
            aria-hidden="true"
        >
            {icon_shapes
                .iter()
                .map(|s| match s {
                    Shape::Path(d) => view! { <path d=*d></path> }.into_any(),
                    Shape::Circle(cx, cy, r) => {
                        view! { <circle cx=*cx cy=*cy r=*r></circle> }.into_any()
                    }
                })
                .collect_view()}
        </svg>
    }
}
