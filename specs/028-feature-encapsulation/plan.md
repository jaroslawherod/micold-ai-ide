# Implementation Plan: Feature Encapsulation — Own Your Messages, Own Your State

**Branch**: `feat/feature-encapsulation` | **Date**: 2026-08-25 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/028-feature-encapsulation/spec.md`

## Summary

Finish 021's Tier 3 by making two things mandatory that 021 left optional or unattempted: every
feature module declares its own message vocabulary and one reducer entry point, and every feature's
state is named in that feature's module rather than scattered across a 44-field root struct. Three
new guards make the pattern non-optional, and they run on all three platforms.

The technical approach, in three parts:

1. **Nest ten vocabularies.** All 119 root variants were attributed to exactly one owner during
   planning ([data-model.md](./data-model.md) §2): 114 to a feature, 5 genuinely cross-cutting. The
   root `Message` goes to **15 variants** — ten feature wrappers and five cross-cutting — with one
   root arm per feature. `worktree_form` is the worked precedent and needs no conversion.
2. **Give each feature its own state struct.** Each feature's fields collapse into
   `features::<n>::State`, held as one field of `app::State`. Root state goes from 44 flat public
   fields to ten feature structs plus one declared shared member (`workspace`).
3. **Guard all of it.** G1 (no single-feature variant at the root), G2 (no single-owner path at the
   root), G3 (every feature with a `Msg` has an entry point) — each with an allowlist carrying a
   written reason per entry, each observed failing its own violation before it is trusted, and all
   three added to the CI step that runs on macOS and Windows.

**Where Story 2's two tracks come from.** Planning found that FR-007 as originally written — move
qualifying state into the iced widget that renders it — has an empty result set: applying its rule
mechanically to all 44 fields yields **5** candidates, and all five are individually pinned to the
application by `tests/logical_state_ownership.rs`, a feature-017 guard that FR-021 forbids relaxing.
The `/speckit-clarify` session of 2026-08-25 settled this: FR-007 now asks for the feature-owned
grouping (part 2 above, the shape `worktree_form` already has), and the widget rule survives as
**FR-007a**, implemented as a guard whose allowlist names the pinning test as each entry's reason. It
moves nothing today and reports the first field that genuinely qualifies. Full reasoning and the
alternatives rejected: [research.md](./research.md) §R4.

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: iced (GUI, client only); no dependency is added or removed by this feature

**Storage**: N/A — local files via `shell/persist.rs`, untouched

**Testing**: `cargo test` via `mise run test` (whole workspace) and `mise run test-core`
(render-free core). Guards are integration tests under `crates/micold-client/tests/` that read
source text and open no window.

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: Desktop application — a three-crate Cargo workspace (`micold-core`,
`micold-client`, `micold-daemon`). This feature touches `micold-client` only.

**Performance Goals**: No regression. Specifically: no additional frames drawn while idle
(SC-008), held by the existing `tests/idle_requests_no_frames.rs`.

**Constraints**: No user-visible behaviour change of any kind (FR-019). The pre-existing suite is
the behaviour specification and no assertion may be removed (FR-021). Every commit builds, runs and
is green (FR-006). Feature modules stay render-free (`tests/features_are_render_free.rs`).

**Scale/Scope**: `crates/micold-client` only — `app.rs` (1,427 lines), `main.rs` (1,808),
`src/features/` (10 modules, 3,591 lines), `src/shell/` (11 modules), `src/ui/`. Roughly 100
existing test files read the state paths that move, so the change is wide and shallow. `showcase/`,
`micold-core`, `micold-daemon` and the wire protocol are out of scope.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Each of the three guards is written and observed
      failing its own injected violation before it is relied upon (FR-017, quickstart §B) — the
      Red step, recorded per task the way 021 recorded its probes. The conversions themselves are
      covered by the pre-existing suite, which FR-021 freezes as the behaviour specification; no
      production code in this feature carries a rule that suite does not already exercise. The
      GUI-glue exception is not invoked.
- [x] **II. Multi-Session Support**: PASS. No session state changes shape. `workspace.sessions`
      stays a shared member on the root ([data-model.md](./data-model.md) §3.2) and session
      isolation continues to be held by `tests/session_isolation.rs`, unmodified.
- [x] **III. Worktree Integration**: PASS. No file or VCS operation changes. Worktree lifecycle
      stays in `shell/`; the Default-session exception is untouched.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. `shell/persist.rs` reads the same values
      through new paths. Nothing is added to or removed from what is written to disk, and nothing
      leaves the device.
- [x] **V. Rust + iced Stack**: PASS, and this feature strengthens it. Ten `Msg` types and ten
      feature `State` structs make "which feature owns this" a fact the compiler checks rather than
      a `const` in a test file — the ownership map shrinks from 51 hand-maintained entries to the
      declared shared members. No alternative framework is introduced; the spec records the analysis
      that rejected one.
- [x] **VI. Cross-Platform Parity**: PASS, and improved. No platform-specific code is added. The
      three new guards **and the four they extend** join `ci.yml`'s all-platforms step
      ([contracts/guards.md](./contracts/guards.md)), closing an open item 021's T058 and T077 both
      recorded: none of 021's guards runs on macOS or Windows today.
- [x] **VII. Documentation First-Class**: PASS. Nothing user-facing changes, so the user guide needs
      no edit and the Documentation gate's trigger is not met. The maintainer-facing documentation
      *is* the deliverable and ships in the same change: each feature module's header states what it
      owns, and each guard states its rule, its exceptions and its non-vacuity probe.
- [x] **VIII. Reusable UI Component Foundation**: PASS. No component is added, forked or changed.
      Contract S4 binds any future component-owned state to the chainable builder API, which
      `tests/material_builder_api.rs` already holds; today it binds nothing, because the qualifying
      set is empty ([research.md](./research.md) §R4).

**Re-check after Phase 1 design**: all eight still PASS. Phase 1 added no new dependency, no new
component, no new platform branch and no new persisted value. The only design decision with
constitutional weight is the FR-007 departure above, and it exists *because* Principle I forbids
removing the covering test that blocks it — recorded in Complexity Tracking rather than waived.

## Project Structure

### Documentation (this feature)

```text
specs/028-feature-encapsulation/
├── plan.md                          # This file
├── spec.md                          # Input
├── research.md                      # Phase 0 — R0..R9, incl. the FR-007 finding
├── data-model.md                    # Phase 1 — all 119 variants, all 44 fields, classified
├── contracts/
│   ├── feature-boundary.md          # M1-M5, S1-S4, B1-B2
│   └── guards.md                    # G1, G2, G3 + CI and freeze enforcement
├── quickstart.md                    # Phase 1 — measurements, probes, the manual pass
├── checklists/requirements.md       # Pre-existing
└── tasks.md                         # Phase 2 (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-client/
├── src/
│   ├── app.rs                       # Message 119 -> 15; State 44 fields -> 10 structs + shared
│   ├── main.rs                      # update_inner: 52 arms -> one per feature with effects
│   ├── features/                    # the ten feature modules — each gains `Msg`, `update`, `State`
│   │   ├── mod.rs                   # Outcome vocabulary (extended only where a conversion needs it)
│   │   ├── connection.rs  help.rs  notifications.rs  project.rs  session.rs
│   │   ├── settings.rs  sidebar.rs  window.rs  worktree.rs
│   │   └── worktree_form.rs         # the precedent — Msg + update already present
│   ├── shell/                       # effectful entry points (shape B), by external system
│   ├── ui/                          # views — read feature state through the new paths
│   └── overlay/                     # registry — unchanged; owns the cross-cutting dispatch
└── tests/
    ├── root_vocabulary_is_cross_cutting.rs   # NEW — G1 (FR-013)
    ├── root_state_is_shared.rs               # NEW — G2 (FR-014)
    ├── feature_registration_cost.rs          # EXTENDED — G3 (FR-015)
    ├── feature_write_isolation.rs            # OWNERS shrinks to the shared members
    ├── logical_state_ownership.rs            # UNCHANGED — and it is what bounds FR-007
    └── ...                                   # ~100 files whose state paths change spelling only

.github/workflows/ci.yml             # all-platforms step gains 3 new + 4 existing guards (FR-018)
scripts/check-assertions-frozen.sh   # scope_reason() gains 028 (FR-021)
specs/028-feature-encapsulation/assertion-adjudications.md   # NEW — spelling changes, with reasons
```

**Structure Decision**: The existing three-crate workspace is kept and only `micold-client` is
touched. No new crate, module directory or layer is introduced: the ten feature modules, the eleven
shell modules and the overlay registry all already exist, and this feature moves declarations into
them rather than creating homes. That is the whole reason the spec calls its inherited machinery
"scope reducers, not scope".

## Implementation phases

Ordered by [research.md](./research.md) §R9. Each conversion is one commit, buildable and green.

| Phase | Work | Story | Exit |
|---|---|---|---|
| **P1** | Nest `help` (3), `window` (2), `notifications` (2) | US1 | root `Message` 119 → 112; the pattern proven on the cheapest three |
| **P2** | Nest `settings` (10), `sidebar` (10) | US1 | 112 → 94 |
| **P3** | Nest `connection` (12) — first shape-B-only feature | US1 | 94 → 83; the two-entry-shape rule exercised |
| **P4** | Nest `worktree` (18) incl. `TextCopyRequested`, `project` (19) | US1 | 83 → 48 |
| **P5** | Nest `session` (37) | US1 | 48 → **15**. **US1 independently shippable here** |
| **P6** | G1 + G3 land, with probes; `ScrolledBeneathOverlay` decided and pinned (FR-020) | US3 | SC-002, SC-004, SC-005 (2 of 3) |
| **P7** | Feature state structs, one feature per commit (Track 2A) | US2 | root `State` 44 flat fields → 10 structs + `workspace` |
| **P8** | G2 lands with its probe; `OWNERS` shrinks to the shared members; FR-007's component rule lands with its allowlist | US2/US3 | SC-003, SC-007, SC-005 (3 of 3) |
| **P9** | CI all-platforms step gains the seven guards; freeze scope gains 028 | — | FR-018, FR-021 enforced rather than assumed |

Guards land **after** the conversions they describe (P6 after P5, P8 after P7): a guard that has to
be relaxed to let its own migration through is not holding anything, an argument
`feature_registration_cost.rs` already makes about itself.

**SC-009** is satisfied at P5: Stories 1 and 3's first two guards deliver standalone value if the
work stops there.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| FR-007a's component move is implemented as a guard that moves nothing today, rather than as a migration | All 5 fields meeting the rule are pinned to the application by `tests/logical_state_ownership.rs` (feature 017). FR-021 forbids removing an existing assertion to accommodate this restructuring, and the 017 guard's reasoning survives scrutiny: `expanded` and `sidebar_filters` plainly still mean something with the screen switched off. Ratified by clarification on 2026-08-25. | *Relax the 017 guard so the five can move*: rejected on FR-021 and on merit — a widget owning `expanded` puts a reveal's target inside the renderer. *Drop the rule as unimplementable*: rejected — it is what stops the next transient interaction field from landing in the root by default, and `SelectState` in `ui/material/select.rs:213` shows such fields do exist here. |
| CI and the assertion-freeze script are edited by a feature that claims to change no behaviour | FR-018 requires the guards to run where there is no window, and `ci.yml`'s all-platforms list is the only place that happens; FR-021 requires the freeze, which is currently scoped to feature 021 and reports without failing for 028. | *Rely on the Linux full-workspace run*: rejected — it is what left 021's guards single-platform, an open item T058 and T077 both recorded. *Rely on the freeze's report-only output*: rejected — FR-021 would then be a sentence with nothing enforcing it, which is the failure mode this whole feature exists to correct. |

## The spec's baseline, as verified

Three figures the spec inherited from feature 021 were stale when measured against the code. The
clarification session of 2026-08-26 corrected all three in the spec itself, so this plan and the
spec now agree; they are listed here as the record of what was measured.

- **10** feature modules, not eleven (`mod.rs` is not a feature). SC-004 accordingly reads **ten of
  ten, up from one of ten** — as a ratio it was previously unmeetable.
- **12** `Outcome` variants, not seven. Does not affect scope.
- **13** files carrying a `Widget` impl in `src/ui/`, not 18. Does not affect scope.

Everything else in the spec's baseline table verified exactly: 119 variants, 44 fields, 300-line
`State::update`, 52 `update_inner` arms, 51 `OWNERS` entries, 1 of 10 features nested.
