//! PanYa core - the pure domain layer.
//!
//! No I/O, no database, no Tauri: [`fiscal`] owns every calendar ↔ Thai
//! fiscal-year ↔ quarter mapping and [`tracking`] turns plan values and
//! actual purchase values into per-drug verdicts. Both modules are pure
//! functions with unit tests, so the verdicts are reproducible and the SQL
//! layer (or the UI) can never disagree on a date boundary.

pub mod fiscal;
pub mod tracking;
