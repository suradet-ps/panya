# PanYa - Design System & Product Design

PanYa (แผนยา) is a **read-only purchase-plan tracking desktop application**.
It answers one question: **is actual drug purchasing following the fiscal-year
purchase plan?** All data is read from INVS (SQL Server) - the plan from
`BUYPLAN` / `BUYPLAN_C`, actual purchases from `MS_IVO` / `MS_IVO_C`, drug
names from `DRUG_GN`. PanYa never writes to INVS.

This document defines the product scope, UX flows, the tracking/status rules,
the data-model rationale, and the visual design system. Agent-facing
conventions (stack, build commands, schema reference) live in `AGENTS.md` -
do not duplicate them here.

---

## 1. Product Scope

- **Who uses it:** pharmacists and procurement staff tracking the annual drug
  purchase plan (แผนจัดซื้อยา) against real purchases.
- **What it does:** for a chosen fiscal year and quarter, compares the plan
  value per drug against the actual purchase value from INVS and labels
  every drug line: ตามแผน / เฝ้าระวัง / ล่าช้า / เกินแผน / นอกแผน / ไม่มีข้อมูล.
- **What it is not:** not an ERP, not a purchasing system, not a write path
  into INVS. It shows evidence - the buyer decides.
- **UI language:** Thai only. Amounts and codes use Arabic numerals with
  tabular alignment; INVS codes stay Latin.
- **Offline-first:** the app runs on a hospital LAN with no internet. Fonts
  and every asset are bundled locally - no CDN in the critical path.

### Core constraints

1. **Read-only against INVS** - every statement is a `SELECT`; the app holds
   one connection for the whole session.
2. **Thai fiscal year** - FY N = 1 Oct (N−1) → 30 Sep N; quarters are
   Q1 ต.ค.–ธ.ค., Q2 ม.ค.–มี.ค., Q3 เม.ย.–มิ.ย., Q4 ก.ค.–ก.ย.
3. **Verdicts are computed, never stored** - the tracking engine is pure
   Rust, unit-tested; the database only supplies numbers.
4. **No data is a real state** - a drug with no plan or no purchase renders
   as "ไม่มีข้อมูล" / "นอกแผน", never as a blank that looks comparable.

---

## 2. UX Flows

### 2.1 First launch → connection setup

1. App starts with no saved config → the **settings dialog opens
   automatically**.
2. INVS section: host, port, database, user, password, optional instance
   name. **Test** runs against the typed values before anything is saved
   (latency shown); **Save** connects first, then persists the config
   encrypted (AES-256-GCM, master key in the OS keychain).
3. Success updates the top-bar status dot; the main screen becomes usable.
4. The gear button reopens the dialog; on close, typed password buffers are
   zeroized. Escape closes from anywhere.

### 2.2 Main tracking screen

Top to bottom:

1. **Top bar** - wordmark แผนยา, INVS status dot, fiscal-year selector,
   settings button.
2. **KPI strip** - แผนรวมปี (`BUYPLAN.VALUE_THIS_YEAR`), ซื้อจริงสะสม
   (`MS_IVO_C.VALUE` inside the fiscal window), % ความสำเร็จ (featured card),
   จำนวนรายการล่าช้า, จำนวนรายการนอกแผน.
3. **Filter bar** - quarter pill group (ทั้งปี / Q1 / Q2 / Q3 / Q4), status
   filter pills, search by รหัสยา / ชื่อยา.
4. **Tracking table** - one row per drug: `WORKING_CODE`, `DRUG_NAME`, plan
   value (selected range), actual value, % achievement, mini quarter bars,
   status badge. Clicking a row opens the detail panel; keyboard Enter does
   the same.
5. **Drug detail panel** (right drawer) - plan vs actual per quarter, the
   cumulative curve, the raw numbers, secondary `QTY_ORDER`, and any
   unplanned purchases of that drug.

### 2.3 Status rules (defaults - pending product confirmation)

Cumulative actual vs cumulative plan through the selected quarter:

| Verdict | Thai label | Default rule |
|---|---|---|
| `ontrack` | ตามแผน | actual ≥ 90% of plan-through-quarter |
| `watch` | เฝ้าระวัง | 60–90% of plan-through-quarter |
| `behind` | ล่าช้า | < 60% of plan-through-quarter |
| `over` | เกินแผน | actual > plan-through-quarter + 20% |
| `unplanned` | นอกแผน | actual > 0 with no plan line |
| `nodata` | ไม่มีข้อมูล | no plan line and no purchase in the window |

- Status precedence: `nodata` → `unplanned` → `over` → `behind` → `watch` →
  `ontrack`.
- Mid-quarter, "plan-through-quarter" may be pro-rated by elapsed days; the
  choice is a product decision (see `AGENTS.md` open items). The UI shows a
  "pace" line when pro-rata is enabled so the operator can see both readings.
- Thresholds are defaults; if they become configurable they live in app
  settings, not in SQL.

### 2.4 Export (planned)

CSV of the current table view through the native save dialog. No PDF in v1.

---

## 3. Visual Design System

A **pop-neon pink language** rebuilt for a Thai desktop data application,
matching the app icon (`icon-master.svg`): white canvas, pink primary actions
with comic-black text, soft pink-tinted cards, and semantic status colors
reserved for verdicts. Thai UI needs a Thai-capable family, so the reference
system's display face is substituted with **IBM Plex Sans Thai** + **IBM Plex
Mono** for codes and numbers, both bundled with the app.

### 3.1 Color Tokens

Brand & action:

| Token | Value | Role |
|---|---|---|
| `--primary` | `#ff4081` | Primary CTAs, active pills, featured KPI (the app-icon pink) |
| `--primary-active` | `#e91e63` | Pressed state of primary |
| `--link` | `#c2185b` | Inline text links (AA on white) |
| `--badge-orange` | `#fb923c` | Badge pastel, `นอกแผน` status |
| `--badge-pink` | `#ec4899` | Badge pastel, used sparingly |
| `--badge-violet` | `#8b5cf6` | Badge pastel, used sparingly |
| `--badge-emerald` | `#34d399` | Badge pastel, used sparingly |

Surface:

| Token | Value | Role |
|---|---|---|
| `--canvas` | `#ffffff` | Page floor, cards on tinted layouts |
| `--surface-soft` | `#fff4f8` | Pink-tinted pill-group background, hover fills |
| `--surface-card` | `#fdeff5` | Pink-tinted cards, table zebra/selected row |
| `--surface-strong` | `#e5e7eb` | Chart track, disabled button background |
| `--hairline` | `#e5e7eb` | 1px borders, table dividers, input outlines |
| `--hairline-soft` | `#f3f4f6` | Barely-visible divider between sibling bands |

Text:

| Token | Value | Role |
|---|---|---|
| `--ink` | `#111111` | Headlines, primary text |
| `--body` | `#374151` | Running text |
| `--muted` | `#6b7280` | Secondary text, column headers, captions |
| `--muted-soft` | `#898989` | Tertiary text, placeholders, copyright |
| `--on-primary` | `#111111` | Text on the pink primary/featured surfaces (AA) |

Semantic & verdict (status = color + tint background):

| Token | Value | Role |
|---|---|---|
| `--success` | `#10b981` | Success states |
| `--warning` | `#f59e0b` | Warning callouts |
| `--warning-text` | `#92400e` | AA-readable text on warning tint |
| `--error` | `#ef4444` | Validation errors |
| `--status-ontrack` / `-bg` | `#10b981` / `#ecfdf5` | ตามแผน |
| `--status-watch` / `-bg` | `#f59e0b` / `#fffbeb` | เฝ้าระวัง |
| `--status-behind` / `-bg` | `#ff1744` / `#fef2f2` | ล่าช้า |
| `--status-over` / `-bg` | `#8b5cf6` / `#f5f3ff` | เกินแผน |
| `--status-unplanned` / `-bg` | `#fb923c` / `#fff7ed` | นอกแผน |
| `--status-nodata` / `-bg` | `#898989` / `#f5f5f5` | ไม่มีข้อมูล |

### 3.2 Typography

Families (bundled, offline):

- `--font-ui`: `"IBM Plex Sans Thai", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
- `--font-mono`: `"IBM Plex Mono", ui-monospace, "SF Mono", Consolas, monospace`

| Token | Size | Weight | Line height | Tracking | Use |
|---|---|---|---|---|---|
| `display-lg` | 32px | 600 | 1.15 | -0.5px | Page/app title |
| `display-md` | 24px | 600 | 1.2 | -0.4px | Section titles, hero numbers |
| `title-lg` | 20px | 600 | 1.3 | -0.3px | KPI values |
| `title-md` | 16px | 600 | 1.4 | 0 | Card titles, drawer headings |
| `title-sm` | 14px | 600 | 1.4 | 0 | Table headers, field labels |
| `body-md` | 14px | 400 | 1.5 | 0 | Default running text |
| `body-sm` | 13px | 400 | 1.5 | 0 | Secondary text |
| `caption` | 12px | 500 | 1.4 | 0 | Badges, footnote captions |
| `code` | 13px | 400 | 1.5 | 0 | Codes and amounts (`--font-mono`) |
| `button` | 14px | 600 | 1.0 | 0 | Button labels |
| `nav-link` | 14px | 500 | 1.4 | 0 | Top bar, tabs, filter pills |

Principles:

- Display sizes are weight 600 only - never 700, never 500.
- Negative tracking applies to Latin display type only; for Thai headlines
  keep 0 to -0.2px, because tight tracking harms Thai word shapes.
- Body copy never uses display tracking. Codes, drug codes, and amounts are
  always `--font-mono`, right-aligned in tables.

### 3.3 Spacing, Radii, Elevation

Spacing - base unit 4px:

| Token | Value | Typical use |
|---|---|---|
| `--space-xxs` | 4px | Icon gaps, pill inner padding |
| `--space-xs` | 8px | Tight stacks |
| `--space-sm` | 12px | Filter bar gaps, form rows |
| `--space-md` | 16px | KPI strip gaps, card padding (compact) |
| `--space-lg` | 24px | Page padding, card padding |
| `--space-xl` | 32px | Drawer padding |
| `--space-xxl` | 48px | Empty states, modal breathing room |

Radii:

| Token | Value | Use |
|---|---|---|
| `--radius-sm` | 4px | Checkbox, tiny accents |
| `--radius-md` | 6px | Buttons, inputs, tabs |
| `--radius-lg` | 8px | Cards, table wrapper |
| `--radius-xl` | 12px | KPI cards, drawer panel |
| `--radius-pill` | 9999px | Pills, badges, progress track |
| `--radius-full` | 50% | Status dot, icon buttons |

Elevation - soft and modern, no neumorphism, no glassmorphism:

| Level | Treatment | Use |
|---|---|---|
| Flat | none | Bands, table, top bar |
| Hairline | 1px `--hairline` | Inputs, cards, dividers |
| Soft | `0 1px 2px rgba(0,0,0,0.05)` | Pills, sticky header on scroll |
| Raised | `0 4px 12px rgba(0,0,0,0.08)` | Dropdowns, detail drawer |
| Overlay | `0 8px 24px rgba(0,0,0,0.18)` | Modal only |

The pink primary surface is the emphasis signal - it replaces shadow and
badge-based emphasis for the featured KPI card.

### 3.4 Layout

```
┌──────────────────────────── top bar (48px) ─────────────────────────────┐
│ แผนยา              ● INVS      [ปีงบประมาณ ▾]              [ตั้งค่า]     │
├─────────────────────────────────────────────────────────────────────────┤
│ KPI: [แผนรวมปี] [ซื้อจริงสะสม] [% ความสำเร็จ ▪pink] [ล่าช้า] [นอกแผน]   │
├─────────────────────────────────────────────────────────────────────────┤
│ [ ทั้งปี | Q1 | Q2 | Q3 | Q4 ]   [สถานะ ▾]   [ค้นหารหัส/ชื่อยา…]        │
├────────────────────────────────────────────┬────────────────────────────┤
│ tracking table (flex, scrolls)             │ detail panel (360px)       │
│  … rows …                                  │  plan vs actual / quarter  │
└────────────────────────────────────────────┴────────────────────────────┘
```

- **Window:** minimum 1000 × 600, default 1200 × 700. The app is a desktop
  tool, not a responsive website - no hamburger, no full-screen sheet.
- **Top bar:** 48px, flat canvas, hairline bottom border, always visible.
- **Bands:** KPI strip (padding `--space-md`), filter bar (hairline bottom),
  then the table fills the remaining height and scrolls independently.
- **Detail drawer:** 360px on the right; below 1120px width it narrows to
  320px so the table keeps room.
- `prefers-reduced-motion` disables transitions.

### 3.5 Components

**`top-bar`** - 48px, `--canvas`, hairline bottom. Wordmark "แผนยา" in IBM
Plex Sans Thai 16px / 600 at left; connection status dot (7px,
`--success` / `--error`) with a short Thai label; fiscal-year select; icon
button for settings.

**`nav-pill-group`** - the quarter selector and any grouped sub-nav. Wrapper
background `--surface-soft`, internal padding 3px, `--radius-pill`. Active
segment renders as a `--primary` pill with `--on-primary` text inside the
wrapper - the pill-in-pill treatment is the system's signature interactive
component.

**`kpi-card`** - `--canvas`, 1px `--hairline`, `--radius-xl`, padding
10 × 12px. Caption label in `--muted`, value in `title-lg` (`--ink`, mono for
amounts), optional delta line in `body-sm`.

**`kpi-card-featured`** - the same geometry with `--primary` (app-icon pink)
background and `--on-primary` (ink) text, echoing the icon's black-on-pink.
Used exactly once per screen: the % ความสำเร็จ card. The pink surface **is**
the emphasis - no border, no badge, no scale change.

**`filter-bar`** - hairline bottom, horizontal, gap `--space-xs`: quarter
`nav-pill-group`, status `nav-pill-group`, search `text-input` growing to the
right.

**`text-input` / `select`** - `--canvas`, `--ink`, `body-md`, `--radius-md`,
height 34px, padding 8 × 12, 1px `--hairline`. Focus: border `--ink` plus the
shared 2px `--primary` focus ring.

**`button-primary`** - `--primary` background, `--on-primary` text,
`typography.button`, height 34px, padding 8 × 14, `--radius-md`. Pressed:
`--primary-active`. Disabled: 0.4 opacity.

**`button-secondary`** - `--canvas` with `--hairline` border, `--ink` text,
same geometry as primary.

**`button-icon`** - 32 × 32px, `--radius-md`, hairline border, ink icon
(stroke 1.5px, Lucide-style, `aria-hidden`).

**`status-badge`** - pill, `caption` type, tint background + status text
color (e.g. ตามแผน = `--status-ontrack` on `--status-ontrack-bg`). This is
the per-row verdict chip and the only place status colors appear as fills.

**`progress-bar`** - 6px track in `--surface-strong`, `--radius-pill`; fill
uses the row's status color and caps at 100%. Over-plan rows keep the full
bar and print "+x%" beside it. No animation over 200ms.

**`tracking-table`** - sticky header (`title-sm`, `--muted`), rows 40px with
hairline dividers; zebra off, hover `--surface-soft`, selected
`--surface-card`. Columns: code (mono), name, plan, actual, %, quarter mini
bars, `status-badge`. Amounts right-aligned with thousands separators and
2 decimals. Empty and "ไม่มีข้อมูล" states are rendered, never omitted.

**`quarter-compare-chart`** - hand-rolled `<canvas>` (no chart library, no
Node): grouped bars per quarter - plan in `--surface-strong`, actual in the
row's status color - with Thai quarter labels (ต.ค.–ธ.ค. …). Tooltip shows
mono numbers; redraws on resize like the balance chart.

**`drug-detail-drawer`** - right panel, `--canvas`, hairline left border,
padding `--space-xl`, `--elevation-raised`. Header: drug name (`title-md`) +
code (mono) + `status-badge`; body: the quarter chart, a numbers list
(plan/actual/remaining/%), and secondary `QTY_ORDER` when useful.

**`warning-banner`** - `--status-watch-bg` fill, `--warning` border,
`--warning-text` text; used for schema-degraded data and connection loss.

**`empty-state`** - centered `--muted-soft` icon + one Thai sentence. A drug
with no data must be visible, not absent.

**`settings-modal`** - centered, max-width 460px, `--elevation-overlay`,
backdrop `rgba(17,17,17,0.4)`. INVS connection section with Test/Save;
typed password buffers zeroized on close.

### 3.6 Do's and Don'ts

**Do**

- Reserve `--primary` (#ff4081) for primary CTAs, active pills, and the
  featured KPI. The action layer is pink; text on it is comic-black (AA).
- Use semantic/status colors only for verdicts and alerts - never on
  buttons.
- Use the pink featured surface exactly once per screen (featured KPI).
  Scarcity is the signal.
- Keep amounts and codes in `--font-mono`, right-aligned.
- Show "ไม่มีข้อมูล" / "นอกแผน" as first-class states.
- Keep pills for grouped tabs and badges - the pill-in-pill pattern is the
  signature interaction.

**Don't**

- Don't use badge pastels or status colors on primary CTAs.
- Don't exceed weight 600 in display type.
- Don't use radius above `--radius-xl` (12px) on cards.
- Don't put the pink primary surface anywhere except the featured KPI card.
- Don't repeat the same surface mode in two consecutive bands.
- Don't fetch fonts, icons, or any asset from a CDN - offline-first is a law.
- Don't add hover-only information; the documented states are default,
  pressed, focus, disabled.

### 3.7 Accessibility

- Visible focus ring on every interactive element (2px `--primary` offset
  ring).
- WCAG AA contrast: status text on its tint, `--warning-text` (#92400e) on
  `--status-watch-bg`.
- Row keyboard support: Tab into the table, arrows move, Enter opens the
  drawer, Esc closes it.
- Icons decorative (`aria-hidden`); status is never conveyed by color alone -
  the Thai verdict word always accompanies the color.
- Reduced-motion support; checks for the connection poll use the same
  timing discipline as `balance` (banner on loss, app keeps composure).

---

## 4. Data Model Rationale

- **Plan:** `BUYPLAN` holds the year-level total (`VALUE_THIS_YEAR`) and the
  four quarter totals; `BUYPLAN_C` holds the per-drug lines
  (`WORKING_CODE`, `TRIMESTER1`..`4`) and is the **primary tracking table**.
- **Actual:** `MS_IVO` is the purchase/receive header (`RECEIVE_DATE`, an
  `INT` in `YYYYMMDD` form); `MS_IVO_C` carries the lines (`WORKING_CODE`,
  `VALUE`, `QTY_ORDER`); `DRUG_GN` supplies `DRUG_NAME`.
- **Assembly:** plan and actual are fetched as separate result sets and
  joined in Rust by `WORKING_CODE` (HashMap). No cross joins in SQL, no
  per-drug query loops for the table.
- **Two directions, both visible:** plan rows with no purchase (ยังไม่ซื้อ)
  and purchases with no plan row (นอกแผน) are equally part of the verdict.
- **One date truth:** calendar → fiscal year → quarter mapping lives in one
  pure module; SQL only filters an inclusive `YYYYMMDD` window.
- **Open items** (VAT/price basis, `BUYPLAN.YEAR` era, plan revisions) are
  tracked in `AGENTS.md` - resolve before implementation.

---

## 5. Iteration Guide

1. Work on ONE component at a time and reference its token directly.
2. Variants (`-pressed`, `-focus`, `-disabled`) are separate specs - never
   document hover as a state.
3. Use tokens everywhere; never inline a hex value in component code.
4. When emphasis is needed: larger display type before bolder display type;
   the pink featured surface before any accent color.
5. The trinity does not blur: display = 600 with tracking, body = 400, mono =
   codes and amounts.

## 6. Known Gaps

- The reference system's display face is licensed and unavailable; IBM Plex
  Sans Thai 600 is the Thai-capable substitute. It preserves the weight +
  tracking signature but not the exact geometric voice.
- The reference design documented a marketing site; hero bands, pricing
  tiers, testimonials, avatars, star ratings, and the marketing footer are
  intentionally dropped. The top bar, KPI strip, tracking table, and detail
  drawer are app-specific additions.
- Status thresholds, the pro-rata mid-quarter rule, and plan-revision
  handling are pending product confirmation (see `AGENTS.md`).
- Chart rendering is hand-rolled `<canvas>` like `balance`; performance on a
  full year of drugs is a Phase-2 concern.
- Form validation states beyond focus are not specified; error text uses
  `--error` on `--surface-card`.
- Badge pastel hexes are inherited from the reference palette and may shift
  if that palette is revised.
