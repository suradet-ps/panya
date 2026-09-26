# PanYa (แผนยา)

```
██████╗  █████╗ ███╗   ██╗██╗   ██╗ █████╗
██╔══██╗██╔══██╗████╗  ██║╚██╗ ██╔╝██╔══██╗
██████╔╝███████║██╔██╗ ██║ ╚████╔╝ ███████║
██╔═══╝ ██╔══██║██║╚██╗██║  ╚██╔╝  ██╔══██║
██║     ██║  ██║██║ ╚████║   ██║   ██║  ██║
╚═╝     ╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝   ╚═╝  ╚═╝
```

---

## ◆ PULSE

The plan is in the books. The question is whether the purchases follow
it. PanYa reads INVS read-only and answers one question per Thai fiscal
year and quarter: **is actual drug purchasing following the plan?** Per
drug: plan against actual, cumulative through the selected scope,
achievement, and a verdict - ตามแผน / เฝ้าระวัง / ล่าช้า / เกินแผน /
นอกแผน / ไม่มีข้อมูล. Plan lines with no purchase and purchases with no
plan line are both visible. The buyer decides; PanYa only shows the
evidence.

| Quarter scope ▣ | Verdict ramp ▣ | KPI strip ▣ | Detail drawer ▣ |
|---|---|---|---|

*v0.1.0 - the walking skeleton stands: connect, scope, track, drill down.*

> Built with Tauri 2 + Leptos 0.8, judged by `panya-core`, read from INVS
> by `panya-invs` - never a write, never a plaintext secret.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One toolchain, one launch.

```
⟫ rustup target add wasm32-unknown-unknown
⟫ cargo install trunk --locked
⟫ cargo install tauri-cli --locked

⟫ cd apps/panya-app
⟫ cargo tauri dev
```

The release artifact: `⟫ cargo tauri build` (from `apps/panya-app`).
Frontend only: `⟫ trunk serve frontend/index.html --port 1420`.

<details>
<summary>Prerequisites</summary>

- Rust stable with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev) - installed above
- Tauri 2 system dependencies for your platform
- An INVS SQL Server (a read-only account is recommended)

</details>

On first launch the settings modal asks for the INVS connection - the
credentials are encrypted (AES-256-GCM, master key in the OS keychain)
before they ever touch disk.

---

## ◆ ANATOMY

Three parts, one law that never bends: INVS is read, never written.

- **Judges** - `panya-core` is the pure domain: Thai fiscal quarters
  (Oct-Sep, Q1-Q4) and the plan-vs-actual tracking engine. No I/O, unit
  tested, so the same numbers always yield the same verdict.
- **Reads** - `panya-invs` is the `tiberius` SQL Server layer: plans
  from `BUYPLAN` / `BUYPLAN_C`, actuals from `MS_IVO` / `MS_IVO_C`,
  names from `DRUG_GN`. Every statement is a `SELECT`.
- **Seals** - the Tauri shell keeps the one connection behind a mutex
  and persists settings through `encryptman-keyring`; no plaintext
  credential reaches disk, logs, or error messages.
- **Speaks** - the Leptos frontend is Thai-only: the KPI strip, quarter
  and status pills, the tracking table, and the per-drug drawer. The
  pink comes from the icon; the verdict colors carry severity, never
  decoration.
- **Marks** - `icon-master.svg` is the single source for the app icon;
  `script/gen-icons.sh` forges every platform size with `cargo tauri
  icon`.

---

## ◆ RITUALS

**The core ceremony** - one fiscal year at a time:

1. Connect to INVS once; the keychain keeps the credentials sealed.
2. Pick the fiscal year (read from `BUYPLAN_C`) and the scope: ทั้งปี
   or Q1-Q4.
3. Read the table: plan, actual, achievement, progress, verdict. Filter
   by status, search by code or name.
4. Open a drug: the drawer shows the scoped numbers and the quarter
   breakdown, plan against actual, with the remaining budget.

**The ceremony of honesty** - "ไม่มีข้อมูล" and "นอกแผน" are legitimate,
visible verdicts, never blanks that look comparable. The Thai label
always accompanies the color, so hue is never the only signal.

**The ceremony of silence** - PanYa reads from INVS and never writes a
row into it. The system of record stays the system of record.

---

## ◆ ECHOES

**Where this artifact is heading**

```
P1   ▸ workspace, core engine, INVS queries, IPC, tracking UI ──────── ▸ sealed
P2   ▸ canvas quarter chart, CSV export ────────────────────────────── ▸ ahead
P3   ▸ fonts bundled locally (offline-first is a law) ──────────────── ▸ ahead
P4   ▸ live INVS schema verification (AGENTS.md open items) ────────── ▸ forging
P5   ▸ watchlist, notes, plan snapshots (local store) ──────────────── ▸ open
```

**Raising the artifact** - read `AGENTS.md` first: it holds the INVS
schema reference, the read-only law, and the open items that need a
live connection. The design language and the verdict ramp live in
`DESIGN.md`; the Rust constitution in `AGENTS-RUST.md`.

**Status** - the checks run before every commit: `cargo fmt --check`,
`cargo check --all-targets`, `cargo clippy` (host and wasm) with
`-D warnings`, the `panya-core` tests, and the wasm build of the
frontend. CI gates are not forged yet.

---

```
  ─────────────────────────────────────────
   The plan is a promise.
   The purchases are the record.
   The gap is what PanYa exists to show.
  ─────────────────────────────────────────
```

PanYa is distributed under the MIT License.
