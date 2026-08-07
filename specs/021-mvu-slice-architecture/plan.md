# Implementation Plan: Feature-Module MVU Architecture

**Branch**: `feat/021-mvu-feature-modules` | **Date**: 2026-08-07 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/021-mvu-slice-architecture/spec.md`

## Summary

Restructure the client's monolithic model-view-update core into feature modules, a uniform
floating-surface registry, and a shell split by external system — with no user-visible change and
no assertion in the existing suite modified.

The approach is **type-first extraction before message nesting**, in three tiers plus an orthogonal
shell split, exactly as the spec's structural stance sets out. Twenty migration steps are sequenced
in [research.md](./research.md) §6, each one buildable, runnable and green on its own (FR-028,
SC-009).

**Two discoveries during planning change the shape of the work** and are carried through every
artifact below:

1. **There are two reducers, not one.** The spec's Tier 3 speaks of "the single long reducer",
   meaning `app.rs::update` (778 lines). But `main.rs::update_inner` is **1,253 lines** — larger —
   and holds the effectful half of every feature's message handling. Splitting only the pure
   reducer would leave the bigger one intact and SC-003's shell target unreachable. Tier 3 and the
   shell split are therefore planned as *paired* splits over the same feature boundaries.
2. **`main.rs` is 851 lines of inline tests** (lines 2715–3567), so its production body is 2,715
   lines. Those tests move with the code they cover; SC-003's "below 500 lines" is measured against
   the file as a whole, which the split satisfies by relocating tests alongside their subjects
   rather than by deleting them (FR-027 forbids deletion).

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise` (workspace `Cargo.toml`)

**Primary Dependencies**: `iced` 0.13 (GUI, client only), `micold-core` (render-free domain + ports),
`micold-daemon` (session host, **out of scope** per Q1)

**Storage**: Local JSON files — `JsonFileStore` (projects), `JsonFileSettingsStore` (settings).
Format frozen for this feature (FR-026).

**Testing**: `cargo test --workspace` via `mise run test`; `mise run test-core` for the render-free
core. 71 client integration-test files, plus inline `#[cfg(test)]` modules.

**Target Platform**: Linux, macOS, Windows desktop — all three required green (SC-006)

**Project Type**: Desktop GUI application, three-crate Rust workspace

**Performance Goals**: **None set, deliberately** (FR-019b, clarified 2026-08-07). Capabilities are
supplied by dynamic dispatch; the vtable indirection is not measured and not budgeted, because every
capability call sits behind real I/O costing orders of magnitude more, and no capability is
reachable from the rendering path. FR-025 constrains user-visible behavior — including animation
timing — not runtime cost.

**Constraints**: No observable behavior change (FR-025); persisted format unchanged (FR-026); zero
existing assertions modified (FR-027); every step independently green (FR-028).

**Scale/Scope**: `app.rs` 2,434 lines → target < 500; `main.rs` 3,567 lines → target < 500;
37 state fields, 130 message variants, 10 + 9 overlay variants, 7 loose popover fields, 7 service
ports + 3 to declare.

No NEEDS CLARIFICATION items remain. The spec's two open questions were resolved before merge
(spec §Resolved Decisions); the two planning obligations it deferred — the per-feature nesting
record (FR-003, SC-004a) and the migration sequence (FR-028) — are discharged in research.md §5
and §6 respectively.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: Every new invariant this feature introduces — the overlay
      registration guard (FR-010), the cross-feature-write guard (FR-024a), the
      no-concrete-implementation guard (FR-017) — is written as a failing test before the structure
      that satisfies it. For the *extraction* steps, the pre-existing suite is the specification
      (spec assumption "Test suite is the behavior specification"): FR-027 freezes its assertions,
      so each step is Red-Green against a net that already exists. Per-feature unit tests required
      by SC-004 are written before the module they exercise.
- [x] **II. Multi-Session Support**: No session state is added, removed or re-scoped. Session
      isolation behavior is frozen by FR-025 and held by the existing `session_isolation.rs`,
      `session_archive.rs`, `session_default_no_worktree.rs` suites.
- [x] **III. Worktree Integration**: No worktree operation changes. `worktree_delete.rs` must pass
      unchanged (FR-023's outcome refactor is behavior-preserving by construction).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: FR-026 freezes the persisted format; SC-008
      requires pre-change state to load identically. Nothing gains a network dependency.
- [x] **V. Rust + iced Stack**: FR-029 keeps the MVU shape; FR-030 keeps the builder API. **One
      deviation is recorded in Complexity Tracking** — the principle's "make invalid states
      unrepresentable via the type system" is deliberately *not* applied to cross-feature writes.
- [x] **VI. Cross-Platform Parity**: No platform-specific code is added. The OS-theme capability
      (FR-015) *improves* parity by putting the one existing platform branch behind a port. SC-006
      requires all three platforms green.
- [x] **VII. Documentation First-Class**: Not user-facing, so the user-guide obligation is
      satisfied by architectural documentation (spec assumption "Documentation"). This plan adds
      `docs/development/architecture.md` as a required deliverable, verified by the existing docs
      check in CI.
- [x] **VIII. Reusable UI Component Foundation**: Tier 2's uniform floating surface is a shared
      primitive built on feature 017's existing `Layer`/`Surface`/`Trigger` vocabulary (FR-014), not
      a second parallel one, and exposes the mandated chainable builder terminating in `.into()`
      (FR-030).

**Gate result: PASS**, with one recorded deviation (Principle V, below).

### Post-design re-check (after Phase 1)

Re-evaluated against research.md, data-model.md, the three contracts and quickstart.md. **Still
PASS.** The design surfaced two things worth recording:

- **Principle VI got stronger, not weaker.** The OS-theme capability (contracts/service-capabilities.md)
  puts `dark_light::detect()` — the codebase's only direct operating-system branch — behind a port.
  That is precisely what "platform-specific behavior MUST be isolated behind clear abstractions"
  asks for, so this feature improves cross-platform posture rather than merely preserving it.
- **Principle I needed the exception, and it applies cleanly.** Two validations cannot be automated:
  the persisted-state round-trip (M1) and the overlay exit-animation behaviors (M2). Both are
  recorded as `quickstart.md` procedures, which is exactly the GUI/process-spawn carve-out the
  constitution grants — thin glue with no decision logic of its own. The decision logic they sit
  over (dismissal rules, snapshot lifecycle) lands in render-free modules with automated tests, as
  the exception requires.

No new deviation. The Principle V deviation below is unchanged in scope by the design work.

## Project Structure

### Documentation (this feature)

```text
specs/021-mvu-slice-architecture/
├── spec.md              # Merged (PR #47)
├── checklists/
│   └── requirements.md  # Merged (PR #47)
├── plan.md              # This file
├── research.md          # Phase 0 — decisions, per-feature nesting record, migration sequence
├── data-model.md        # Phase 1 — entities and their relationships
├── quickstart.md        # Phase 1 — validation guide
├── contracts/           # Phase 1 — internal contracts
│   ├── overlay-registry.md
│   ├── service-capabilities.md
│   └── feature-outcomes.md
└── tasks.md             # Phase 2 — NOT created by /speckit-plan
```

### Source Code (repository root)

Target layout. `micold-daemon` is untouched (Q1); `micold-core` gains only capability declarations
and their fakes (Q2 keeps feature modules in the client).

```text
crates/micold-core/src/
├── git.rs, store.rs, settings.rs, fs_scan.rs      # 7 existing ports, unchanged
├── terminal.rs, provider.rs
├── clipboard.rs                                    # NEW capability (FR-015)
├── os_theme.rs                                     # NEW capability (FR-015)
├── env_include.rs                                  # NEW capability (FR-015)
└── notify.rs, worktree.rs, session.rs, workspace.rs, overlay.rs, tokens.rs

crates/micold-client/src/
├── app.rs                                          # → routing + composition only (< 500 lines)
├── features/                                       # Tier 1 — one module per feature
│   ├── worktree.rs                                 #   types + helpers + reducer module
│   ├── worktree_form.rs                            #   NESTED UNIT (own message type)
│   ├── session.rs
│   ├── project.rs
│   ├── sidebar.rs
│   ├── settings.rs
│   ├── notifications.rs
│   └── connection.rs                               #   see research.md §4 (eighth concern)
├── overlay/                                        # Tier 2
│   ├── mod.rs                                      #   uniform surface type
│   └── registry.rs                                 #   the single registration point
├── shell/                                          # FR-019a — split by external system
│   ├── startup.rs, persist.rs, daemon_sync.rs
│   ├── subscriptions.rs, env_include.rs, os_theme.rs
│   └── capabilities.rs                             #   the single assembly point (FR-018)
├── ui/                                             # unchanged; views stay beside their features
└── main.rs                                         # → capability assembly + iced wiring (< 500)

crates/micold-client/tests/                         # 71 files; assertions frozen (FR-027)
├── overlay_registration.rs                         # NEW guard (FR-010)
├── feature_write_isolation.rs                      # NEW guard (FR-024a)
└── no_concrete_implementations.rs                  # NEW guard (FR-017)
```

**Structure Decision**: Feature modules live in `crates/micold-client/src/features/`, alongside
their views in `src/ui/`, per the spec's resolved Q2. The core keeps the domain model and the
declared capabilities only. `features/` is a flat directory of one file per feature — deliberately
*not* `features/<name>/{state,update,view}.rs`, which FR-001a forbids. A feature grows a directory
only if it becomes a nested unit with genuinely separable parts, and today exactly one does
(`worktree_form`, research.md §5).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| **Principle V** — cross-feature write isolation is enforced by a guard test (FR-024a), not by making the invalid state unrepresentable in the type system, which the principle prefers | The spec's Edge Cases require a view to read across features to render (the sidebar reads session data today, spec §Edge Cases). Partitioning `State` into mutually invisible halves would make those legitimate reads impossible or force a projection layer for every view. FR-003a states the constraint directly: isolation on writes, not on reads. | Two alternatives were considered and rejected. (a) *Split `State` into per-feature structs with a read-only view projection*: makes cross-feature reads expensive and re-introduces the view-model layer the structural stance rejects, for isolation a single screen does not need. (b) *Interior-mutability tokens granting per-feature write capability*: pushes the check to runtime, which is strictly worse than a compile-time-adjacent guard test, and is far more machinery than the invariant is worth. The guard-test mechanism is already proven in this codebase — `showcase_isolation.rs` holds exactly this kind of line for feature 020 — and the spec names that precedent explicitly (spec §The second binary). |
