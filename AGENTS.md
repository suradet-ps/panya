# AGENTS.md - PanYa

## Project Overview

PanYa (แผนยา) is a read-only purchase-plan tracking desktop application. It
connects to INVS (SQL Server)
and answers one question: **is actual drug purchasing following the purchase
plan, by Thai fiscal year and quarter?**

Plan values come from `BUYPLAN` / `BUYPLAN_C`; actual purchases come from
`MS_IVO` / `MS_IVO_C` (the same tables the `balance` project reads); drug
names come from `DRUG_GN`. PanYa never writes to INVS.

Product scope, UX flows, tracking/status rules, the data model, and the
visual design system (a monochrome SaaS palette adapted for PanYa) live in
**DESIGN.md**. This file covers agent-facing conventions: stack, build
commands, coding rules, and the INVS schema reference needed to write
correct queries. Do not duplicate DESIGN.md content here - link to it.

## Tech Stack

- **Shell:** Tauri 2
- **Frontend:** Leptos 0.8, CSR (compiled to `wasm32-unknown-unknown`,
  bundled by Trunk - no Node toolchain anywhere in the build)
- **Database access:** SQL Server (INVS) via `tiberius` 0.13, read-only; one
  client behind a `tokio::sync::Mutex` in Tauri managed state
- **Credential/config encryption:** `encryptman` / `encryptman-keyring`
  (AES-256-GCM, master key in the OS keychain). Never store plaintext
  credentials on disk, in logs, or in error messages.
- **Reference implementation:** `../balance` (same Tauri 2 + Leptos 0.8 +
  tiberius stack, reading the same INVS tables) - consult it before
  inventing new wiring.
- **Documentation convention:** `DESIGN.md` (product design + visual
  design system), `AGENTS-RUST.md` (the AgentForge-RUST constitution -
  mandatory for all Rust work), `AGENTS.md` (this file).

## Core Constraints

- **Read-only against INVS.** No `INSERT`/`UPDATE`/`DELETE`/DDL against any
  system of record under any circumstance. All queries are `SELECT` only,
  ideally executed via a read-only DB user/role.
- **Thai fiscal year (ปีงบประมาณ).** FY N = 1 Oct (N−1) → 30 Sep N.
  Quarters (confirmed): **Q1 = ต.ค.–ธ.ค., Q2 = ม.ค.–มี.ค.,
  Q3 = เม.ย.–มิ.ย., Q4 = ก.ค.–ก.ย.**
- **One source of date truth.** All calendar ↔ fiscal year ↔ quarter mapping
  lives in one pure, unit-tested module. Never duplicate date math in SQL,
  IPC handlers, or the UI.
- **The tracking engine is pure.** Plan vs. actual comparison takes numbers
  in and returns per-drug/per-quarter verdicts; no I/O, unit-tested. Tauri
  command handlers stay thin adapters: parse args → call the engine →
  serialize.
- **Thai UI.** All user-facing strings are Thai. "No data" is a visible,
  legitimate state - never a blank that looks comparable.
- **Schema assumptions must be verified against the live read-only INVS**
  before writing a query that touches a new table/column. Do not guess
  column names or types.

## INVS Schema Reference

Use this as the source of truth for table/column names when writing queries.
Columns marked (balance) are already used by the `balance` project; tables
marked (stated) come from the product owner and must be confirmed against the
live schema before relying on types, nullability, or uniqueness.

| Table | Key fields | Notes |
|---|---|---|
| `BUYPLAN` | `YEAR`, `VALUE_THIS_YEAR`, `TRIMESTER1`..`TRIMESTER4` | Year-level plan value (stated). `VALUE_THIS_YEAR` = plan for the whole year; `TRIMESTERn` = plan value per quarter. |
| `BUYPLAN_C` | `YEAR`, `WORKING_CODE`, `TRIMESTER1`..`TRIMESTER4` | Per-drug plan line - the **primary tracking table** (stated). Uniqueness of (`YEAR`, `WORKING_CODE`) unconfirmed. |
| `MS_IVO` | `INVOICE_NO`, `RECEIVE_DATE` | Purchase/receive header (balance). `RECEIVE_DATE` is an `INT` in `YYYYMMDD` form; join key to `MS_IVO_C` is `INVOICE_NO`. |
| `MS_IVO_C` | `INVOICE_NO`, `WORKING_CODE`, `QTY_ORDER`, `VALUE` | Purchase/receive lines (balance). `VALUE` = money, `QTY_ORDER` = quantity. |
| `DRUG_GN` | `WORKING_CODE`, `DRUG_NAME` | Drug master for names (balance). |

Query shape (from balance, reuse the pattern): aggregate actuals with
`MS_IVO_C JOIN MS_IVO ON c.INVOICE_NO = h.INVOICE_NO`, windowed on
`h.RECEIVE_DATE` between the fiscal-year start/end as `YYYYMMDD` integers,
`LEFT JOIN DRUG_GN` for names, then assemble plan × actual in Rust (HashMap
by `WORKING_CODE`) - do not cross-join in SQL.

## Repository Layout

- `crates/panya-core/` - pure domain logic: Thai fiscal quarters
  (`fiscal.rs`) and the plan-vs-actual tracking engine (`tracking.rs`);
  no I/O, serde types shared with the frontend.
- `crates/panya-invs/` - SQL Server access: connection (`client.rs`),
  settings value type (`config.rs`), and every read-only query
  (`queries.rs`).
- `apps/panya-app/` - the Tauri 2 shell: encrypted settings, IPC commands
  (`src/commands.rs`), state, and the `tauri.conf.json` / `Trunk.toml` that
  drive the frontend build.
- `apps/panya-app/frontend/` - the Leptos 0.8 CSR frontend (`src/` Rust,
  `assets/` styles and logo); it consumes the `panya-core` types, so verdict
  labels and quarter labels have one definition.

`../balance` is the reference for the Tauri/tiberius wiring; `../med-recon`
is the reference for this crate layout.

## Development Workflow

- Confirm schema assumptions against a live read-only INVS connection before
  writing a query against a new table/column. This document may drift by
  site/version.
- Never hardcode connection strings; all INVS settings go through the
  encrypted settings path.
- Read `AGENTS-RUST.md` first; it governs all Rust work (ruleset 0.1.0,
  templates `wasm,tauri`). Never hand-edit it - use its `[OVERRIDE §X]`
  section, and keep it fresh with `cargo agentforge check`.
- Build/run (from `apps/panya-app`): `cargo tauri dev`, `cargo tauri build`;
  frontend only (from `apps/panya-app`): `trunk serve frontend/index.html
  --port 1420`.
- Mandatory checks before claiming completion:
  `cargo fmt --check`, `cargo check --all-targets`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --all-features`.
  The Leptos frontend verifies with
  `cargo check -p panya-frontend --target wasm32-unknown-unknown`.
- `.github/workflows/` is audited with `zizmor` and must report no findings
  (default and `--pedantic`). Keep actions pinned by full commit SHA and
  every `permissions` block minimal and inline-commented.
- See `DESIGN.md` for the tracking rules, screen flows, and the visual
  design system.

## Open Items to Resolve Before Implementation

- [ ] Confirm `BUYPLAN.YEAR` era: is `2027` ค.ศ. or พ.ศ.? Confirm the plan
      window is Oct–Sep (fiscal year) and not Sep–Aug.
- [ ] Confirm `BUYPLAN_C` uniqueness on (`YEAR`, `WORKING_CODE`) or whether
      plan revisions stack rows (then decide SUM vs. latest).
- [ ] Confirm the "actual purchase" basis: `MS_IVO.RECEIVE_DATE` (receive
      date, as balance uses) vs. an order-date column/table.
- [ ] Confirm whether plan and actual `VALUE` share the same VAT/price basis.
- [ ] Decide status thresholds (on-track / watch / behind / over / no-plan /
      no-data) and whether mid-quarter pace is pro-rata or full-quarter.
- [ ] Bundle the IBM Plex Sans Thai/Mono woff2 files locally: `index.html`
      currently uses Google Fonts as a progressive enhancement, which
      contradicts the offline-first law in `DESIGN.md`.
- [ ] Decide whether PanYa needs a local SQLite store (watchlist, notes,
      plan snapshots) or stays stateless.
