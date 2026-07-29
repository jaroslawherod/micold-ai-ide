# Implementation Plan: Layout Snapshot Parity Gate

**Branch**: `feat/snapshot-parity` | **Date**: 2026-07-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/019-layout-snapshot-parity/spec.md`

**Depends on**: [`017-material-component-architecture`](../017-material-component-architecture/plan.md),
which has landed. **Does not depend on** [`018-material3-visual-system`](../018-material3-visual-system/plan.md)
— the spec's D1 left that ordering to planning, and research R2 removes the dependency entirely
rather than choosing between waiting and narrowing. **Does not depend on**
[`020-component-showcase-gallery`](../020-component-showcase-gallery/spec.md) either (spec D2): its
gallery would be an additional subject, never a substitute for the application screens FR-008
mandates, and this feature's determinism already comes from FR-007's fixed test data rather than
from the gallery. Neither feature blocks the other and they share no files.

## Summary

Record the resolved geometry of every element in a curated set of application states, commit it as
a text fixture, and assert it byte-for-byte on every build — the same shape as feature 017's
`style_snapshot`, applied to the half of parity that feature could only close by eye.

The approach is settled by four findings from Phase 0, each verified rather than assumed.

**First, the renderer is not a free choice, and that turns out to be fine.** `ui::view` returns
`Element` typed to the *concrete* `iced::Renderer`, so no custom measuring renderer can be
substituted. But this workspace takes iced's default features, so that concrete type is
`fallback::Renderer<wgpu, tiny-skia>` — a public enum whose `Headless::new`, given the backend hint
`Some("tiny-skia")`, makes the wgpu implementation decline on its first line before touching a
`wgpu::Instance`. The result constructs with no GPU, no window and no display. Measured: a 50-node
layout tree over the real `ui::view` at 1280×800.

**Second, the obvious shortcut is a trap.** iced's built-in null renderer `()` measures every
string as 0×0 — the same text that shapes to 244.5×20.8 under real shaping — so a fixture built on
it would record fiction, and it is `#[cfg(debug_assertions)]`-only besides.

**Third, D1 dissolves.** The threat was real and measured: cosmic-text loads the host's system
fonts unconditionally (391 faces here) and resolves `Family::SansSerif` through a per-platform
table. But `material/text.rs` sets no font on body text, so it falls back to the renderer's
default — and the snapshot owns that renderer. Pinning a committed Roboto as the measuring basis
makes text metrics identical everywhere today, with text-derived geometry fully in scope, without
waiting on 018's 76 open tasks. 018 later promotes the same file to the shipped application font.

**Fourth, the scaffolding already exists.** `State::default()` is the established idiom, and
`tests/support/mod.rs` already builds workspaces from in-memory fixtures behind a `FakeScanner`
that touches no filesystem — so FR-007 is satisfied by what is there rather than by new work.

The honest limit, which FR-015 requires stating loudly: this gate pins what *is*, not what is
*correct*. A layout defect present when the fixture is generated is baked in until someone notices
it by eye, and FR-019 forbids quietly fixing anything found along the way.

## Technical Context

**Language/Version**: Rust, stable, MSRV 1.97 (pinned in workspace `Cargo.toml`)

**Primary Dependencies**: `iced 0.14.0` (features: tokio, canvas, advanced, lazy) — already
present. **No new runtime dependency and no new dev-dependency.** The headless renderer, the
`Headless` trait and real text shaping all come from features already enabled; `ttf-parser 0.25.1`
is already a workspace dev-dependency and covers the font guard assertion. One binary asset is
added: Roboto Regular, under `tests/fixtures/`.

**Storage**: N/A — no persisted state, no application state touched. The fixture is a committed
test artefact, not runtime data.

**Testing**: `cargo test --workspace` (`mise run test`). The gate is an ordinary integration test
at `crates/micold-client/tests/layout_snapshot.rs`. **This feature does not invoke the constitution's
Principle I GUI-wiring exception** — it is the exception's replacement for the layout dimension.

**Target Platform**: Linux, macOS, Windows desktop — parity required (Principle VI), and here it is
the central technical risk rather than a checkbox (FR-006, research R2).

**Project Type**: Desktop application, three-crate Cargo workspace

**Performance Goals**: `mise run test` under 60 seconds locally, and no more than 3 seconds added
per covered state (SC-006, SC-006a — both amended after measurement; the original budget named a
test binary and a share of the suite, and is discussed in the spec). The cost is dominated by
shaping real text across every covered screen in both schemes, **not** by the one-time font-system
construction this plan originally predicted: about 12s of the suite's 35s, and irreducible without
narrowing coverage. Measured 2026-07-29 at 35.1s total and 2.21s per covered state.

**Constraints**: The application's appearance and behaviour must not change (FR-019) — mechanically
proved by `style_snapshot` still passing with no regeneration. Covered states must never read the
developer's workspace, config or session store (FR-007). Output must be identical on all three
platforms, with any category that cannot be excluded rather than tolerated (FR-006).

**Scale/Scope**: The reduced parity set feature 017's T001b named — main shell with sidebar
expanded and collapsed, the add-worktree dialog in both branch-source modes, one open menu
(FR-008) — plus the empty and error layouts FR-008c mandates: no project open, an unavailable
project, a disconnected daemon. All resolve at **one canonical window size** (FR-008b); the fixture
records **one scheme**, with the other asserted byte-identical rather than duplicated (FR-008a).
Roughly 50 layout records per state on current evidence. One fixture, one test file, one font
asset. **No application source file is modified.**

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS, and unusually directly — the deliverable *is* a
  test. Red-Green-Refactor still applies and is not vacuous: the assertions (fixture mismatch names
  the element; a missing state fails; an unresolvable anchor fails; the motivating overlap defect
  fails the check) are written and observed failing before the walker and the fixture exist. FR-018
  is the sharpest form of this — the gate must be seen catching the real defect, not merely be
  believed to.
- [x] **II. Multi-Session Support**: PASS. No session-scoped state is added, read or persisted.
  Covered states are constructed in memory and dropped.
- [x] **III. Worktree Integration**: PASS. No file or VCS operation is performed. FR-007 makes the
  stronger commitment: the check must not touch the developer's real worktrees at all, satisfied by
  the existing `FakeScanner` scaffolding.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is stored, nothing leaves the
  device. The font is vendored in-repo, so no network fetch is introduced at build or test time.
- [x] **V. Rust + iced Stack**: PASS. iced only, no widget forked, no renderer reimplemented — the
  gate drives iced's own `Headless` and `Widget::layout`. `CoveredState` makes an unregistered or
  unnamed state unrepresentable rather than checked at runtime.
- [x] **VI. Cross-Platform Parity**: PASS — and this feature is largely *about* it. FR-006 requires
  byte-identical output on all three platforms; research R2 removes the one mechanism that would
  have broken it, and a guard assertion fails loudly if a host font subverts the fix. CI already
  builds and tests all three (FR-017).
- [x] **VII. Documentation First-Class**: PASS. Not user-facing, so the obligation lands on the
  developer documentation feature 017 established at `docs/development/` — FR-015 requires the
  covered/not-covered boundary to be written down explicitly, and SC-007 makes it testable by
  asking whether a reader can answer "would this catch X?" from the docs alone.
- [x] **VIII. Reusable UI Component Foundation**: PASS, vacuously and deliberately. No widget, no
  component and no UI of any kind is added. A change to this feature that adds one is out of scope.

**Post-Phase-1 re-check**: PASS, with one thing worth naming rather than ticking. The design adds a
font asset whose *family name* is resolved by a database that also contains the host's fonts
(research R2, residual risk). That is a genuine, permanent soft spot — it survives feature 018,
because Roboto is a common system font name — and it is handled by a guard assertion pinning a
known measurement, not by hoping. No other decision moved between the pre- and post-design checks;
no new dependency was introduced; no application file is edited.

## Project Structure

### Documentation (this feature)

```text
specs/019-layout-snapshot-parity/
├── plan.md              # This file
├── spec.md              # Merged in b0fee61
├── research.md          # Phase 0 output — R1..R9
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output — validation guide, Parts A..G
├── contracts/
│   └── layout-fixture.md    # Phase 1 output — the fixture format contract
├── checklists/
│   └── requirements.md  # From /speckit-specify
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-client/
├── src/
│   └── ui/
│       ├── mod.rs                     # `pub fn view` — the entry point laid out. UNCHANGED.
│       └── material/
│           └── style_snapshot.rs      # Feature 017's colour gate. UNCHANGED, and its
│                                      #   continued passing is this feature's FR-019 proof.
└── tests/
    ├── layout_snapshot.rs             # NEW — the gate: registry, walker, emitter, assertions
    ├── support/
    │   └── mod.rs                     # EXISTING — FakeScanner + in-memory workspaces (FR-007)
    └── fixtures/
        ├── style_snapshot.txt         # EXISTING — feature 017
        ├── layout_snapshot.txt        # NEW — the committed fixture
        ├── Roboto-Regular.ttf         # NEW — the measuring basis (research R2)
        └── FONT-PROVENANCE.md         # NEW — source and Apache-2.0 licence for the above

docs/development/
└── layout-snapshot.md                 # NEW — covered/not-covered boundary (FR-015, SC-007)
```

**Structure Decision**: The gate is an ordinary integration test in `crates/micold-client/tests/`,
not a crate-internal module. This differs deliberately from `style_snapshot.rs`, which feature
017's T036 had to move *inside* the crate because it asserts against `material::style` after that
module became `pub(crate)`. No such constraint applies here: this feature asserts against
`micold_client::ui::view`, which is public. Keeping it in `tests/` leaves the crate free of
test-only code and puts the fixture next to the one feature 017 already established.

No application source file appears in the tree above as modified, and that is a requirement rather
than an observation (FR-019).

## Complexity Tracking

> No constitution violation requires justification. The table below records the two design
> decisions that add cost, so a reviewer can weigh them deliberately rather than discover them.

| Decision | Why needed | Simpler alternative rejected because |
|----------|------------|--------------------------------------|
| Commit a Roboto binary as the measuring basis | FR-006 requires byte-identical output on three platforms; the host's fonts otherwise decide every text width (measured: 391 faces loaded, `Family::SansSerif` resolved per-platform) | Using the platform default produces a fixture that passes only where generated — the one option the spec explicitly rules out. Excluding text-derived geometry instead does not stay local: text width propagates into every ancestor that hugs its content, gutting the tree rather than one leaf category. |
| Named anchors alongside index paths | FR-004 requires a failure to identify *the element*; `layout::Node` carries no name, type or id, so a bare path is the only identity available | Paths alone satisfy FR-002 but make FR-004's message weak and FR-018's demonstration unassertable. iced's `Operation` traversal can reach widget `Id`s but only for widgets implementing `operate` — most do not, so coverage would be partial *and silently so*, which is the failure FR-015 exists to prevent. |
