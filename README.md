# PanYa (แผนยา)

PanYa is a read-only purchase-plan tracking desktop application. It connects
to INVS (SQL Server) and answers one question: **is actual drug purchasing
following the fiscal-year plan, by Thai fiscal year and quarter?**

Plan values come from `BUYPLAN` / `BUYPLAN_C`; actual purchases come from
`MS_IVO` / `MS_IVO_C`; drug names come from `DRUG_GN`. PanYa never writes to
INVS - every statement is a `SELECT`.

## Stack

- **Shell:** Tauri 2 (Rust)
- **Frontend:** Leptos 0.8 CSR, compiled to `wasm32-unknown-unknown` and
  bundled by Trunk - no Node toolchain anywhere in the build
- **Database:** SQL Server (INVS) via `tiberius` 0.13, read-only
- **Settings:** AES-256-GCM via `encryptman-keyring` (master key in the OS
  keychain)

## Layout

```
panya/
├─ crates/panya-core/     pure domain: Thai fiscal quarters + tracking engine
├─ crates/panya-invs/     SQL Server access: connection + read-only queries
└─ apps/panya-app/        Tauri 2 shell
   └─ frontend/           Leptos 0.8 CSR frontend (Trunk)
```

## Run

```
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --locked

cd apps/panya-app
cargo tauri dev
```

Release artifact: `cargo tauri build` (from `apps/panya-app`).
Frontend only: `trunk serve frontend/index.html --port 1420`.

## Checks

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --workspace --exclude panya-frontend
cargo check -p panya-frontend --target wasm32-unknown-unknown
```

## Docs

- `AGENTS.md` - agent-facing conventions and the INVS schema reference
- `DESIGN.md` - product scope, UX flows, status rules, visual design system
- `AGENTS-RUST.md` - the AgentForge-RUST constitution (ruleset 0.1.0)

## Status

Phase 1 walking skeleton: connection settings, fiscal-year/quarter scoping,
the plan-vs-actual tracking table, KPI strip, and the per-drug detail drawer.
Live INVS schema assumptions are tracked in `AGENTS.md` (open items).
