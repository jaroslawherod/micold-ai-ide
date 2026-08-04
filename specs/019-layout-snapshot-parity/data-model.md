# Data Model: Layout Snapshot Parity Gate

**Feature**: `specs/019-layout-snapshot-parity` | **Date**: 2026-07-28

Nothing here is persisted application state. These are the test-only structures the gate builds in
memory and the one artefact it commits. No type in this feature is reachable from
`micold_client`'s public API or from `micold-core`; the application is not modified (FR-019).

---

## CoveredState

A named, reproducible configuration of the application from which a layout can be resolved. This is
the unit of coverage and the unit a failure names first (FR-004).

| Field | Type | Notes |
|-------|------|-------|
| `name` | `&'static str` | Stable identifier, used as the fixture section header and in failure messages. Renaming one is a fixture diff. |
| `build` | `fn() -> State` | Constructs the application state from fixed data. Never reads the developer's workspace, config or session store (FR-007). |
| `anchors` | `&[Anchor]` | The elements this state cares about by name (see below). May be empty. |

**Window size and colour scheme are not fields.** Both are properties of the run, not of the state:
every covered state resolves at one canonical window size (FR-008b), and the fixture records one
scheme while the other is asserted byte-identical (FR-008a). They are declared once in the fixture
header. Putting them on the state would invite states to disagree about conditions that are, by
requirement, uniform.

**Validation rules**

- `name` MUST be unique across the registry. A duplicate is a registration error, not a silent
  overwrite.
- `build` MUST be a pure constructor over in-memory fixtures — `State::default()`, or the
  `FakeScanner`-backed workspaces in `tests/support/mod.rs` (FR-007).
- `build` MUST be deterministic: two calls produce states that lay out identically (FR-005).
- The registry MUST include the reduced parity set feature 017's T001b named (FR-008): main shell
  with sidebar expanded, main shell with sidebar collapsed, the add-worktree dialog in each of its
  two branch-source modes, and one open menu.
- The registry MUST also include the empty and error layouts (FR-008c): no project open, an
  unavailable project, and a disconnected daemon.
- Layout behaviour that depends on content exceeding its container MUST be reachable through a
  state's fixed data — a constrained panel width, a deliberately over-long label — since window
  size is uniform and cannot be varied to produce it (FR-008b).

**Lifecycle**. Constructed, laid out, recorded, dropped — within a single test run. Nothing
survives the process.

---

## LayoutRecord

One element's resolved geometry within one covered state. The leaf datum of the whole feature.

| Field | Type | Notes |
|-------|------|-------|
| `path` | `Vec<usize>` | Depth-first child indices from the root, e.g. `[0, 2, 1, 0]`. The element's identity (R3). |
| `depth` | `usize` | Derived from `path.len()`; carried explicitly so the fixture can be indented and read as a tree. |
| `x`, `y` | `f32` | Position, normalised per FR-012. |
| `width`, `height` | `f32` | Size, normalised per FR-012. |
| `layer` | `Layer` | Which pass produced it — `Base` or `Overlay` (R5, FR-009). |

**Validation rules**

- All four geometry values are rounded to one decimal place and formatted at fixed precision before
  they enter the fixture (FR-012, R4).
- `-0.0` is normalised to `0.0` before formatting.
- Emission order is the tree's own depth-first order, which is deterministic by construction
  (FR-002). No sorting is applied — sorting would hide a structural reordering, which is precisely
  a change the gate should report.

---

## Anchor

A name bound to a path, for the elements a failure should be able to talk about (R3). Anchors are
what FR-018's demonstration asserts against.

| Field | Type | Notes |
|-------|------|-------|
| `name` | `&'static str` | e.g. `"sidebar.row.label"`, `"sidebar.row.close_button"`. |
| `path` | `&'static [usize]` | The path this name refers to within its covered state. |

**Validation rules**

- An anchor whose path does not resolve in its covered state MUST fail the check, naming the
  anchor. This is one of the ways FR-014 is satisfied: an element disappearing is a visible event,
  not a silent narrowing of coverage.
- Anchors are advisory for *recording* — every element is recorded whether anchored or not — and
  load-bearing for *reporting*.

---

## Fixture

The committed artefact: the collection of `LayoutRecord`s across all covered states, in a
human-readable text form. This is the reference the check asserts against (FR-003) and the thing a
reviewer reads instead of running the application (SC-005).

**Properties**

- One file, committed, at `crates/micold-client/tests/fixtures/layout_snapshot.txt`.
- Text, line-oriented, one record per line, indented by depth so the tree is legible.
- Sectioned by covered state. The window size and scheme are declared once in the file header
  rather than per section, because both are uniform by requirement (FR-008a, FR-008b) — so a record
  is never separated from the conditions that produced it, and those conditions cannot drift
  between sections.
- Asserted **byte-for-byte**. Any difference fails (FR-003).
- Regenerated only by explicit opt-in, never as a side effect of a normal run (FR-013).

**Format contract**: [`contracts/layout-fixture.md`](./contracts/layout-fixture.md).

---

## ReferenceFont

The typeface the fixture's measurements are taken against (R2, resolving D1).

| Property | Value |
|----------|-------|
| Face | Roboto Regular (400) |
| Location | `crates/micold-client/tests/fixtures/` |
| Role | Passed as `default_font` when the headless renderer is constructed |
| Licence | Apache-2.0, matching the workspace licence; provenance recorded alongside the file |

**Validation rules**

- A guard assertion MUST pin a known measurement of a known string, so a same-named font installed
  on the host winning the family lookup fails loudly and specifically rather than presenting as a
  mass layout regression (R2, residual risk).
- This file is the same asset feature 018's T015 should register as the shipped application font.
  It MUST NOT be duplicated when 018 lands.

---

## Relationships

```text
Fixture
  ├── window + scheme            (declared once, file header — uniform by FR-008a/FR-008b)
  └── 1..n  CoveredState        (one section per state, keyed by name)
             ├── 0..n  Anchor       (name → path; reporting only)
             └── 1..n  LayoutRecord (path, depth, x, y, w, h, layer)

ReferenceFont ──used by──▶ headless renderer ──measures──▶ every LayoutRecord
```

## What this feature deliberately does not model

- **Colour, border, radius, shadow.** Already pinned by `style_snapshot` (feature 017). Recording
  them again would create two fixtures that must agree, and a second place to update.
- **Widget type or tree shape as such.** Spec Acceptance Scenario 1.3 is explicit: restructuring a
  tree without moving anything must pass. The record is of geometry, not of composition.
- **Pixels.** See research R1 — available via `Headless::screenshot`, out of scope here.
