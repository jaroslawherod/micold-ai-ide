# Implementation Plan: Material Component Architecture

**Branch**: `feat/improve-material-design` | **Date**: 2026-07-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/017-material-component-architecture/spec.md`

## Summary

Close the boundary between the shared component library and the feature modules that use it, so
that appearance is decided in one place. Wrap every styled rendering widget, split the library into
a behavior layer and an appearance layer, consolidate five overlay implementations into one, and
move presentation state out of application state into the components that own it.

Nothing looks different afterwards. That is the point: this is the foundation the visual system
([`018`](../018-material3-visual-system/plan.md)) is applied to, and keeping it visually inert makes
its large diff reviewable as a single yes/no question.

Phase 0 established that the rendering stack supports what this feature requires — notably
per-instance widget state, which is what lets a component own its own transients — so no new
dependency and no forked widget is needed.

## Technical Context

**Language/Version**: Rust, stable, MSRV 1.97

**Primary Dependencies**: `iced 0.13.1` (features: tokio, canvas, advanced, lazy). **No new
dependency.** The `advanced` feature — required to implement custom widgets with per-instance
state — is already enabled.

**Storage**: N/A. This feature changes no persisted data. The presentation state being moved is not
serialized.

**Testing**: `cargo test --workspace` (`mise run test`). Baseline before this feature: 781 passing.

**Target Platform**: Linux, macOS, Windows — parity required

**Project Type**: Desktop application, three-crate Cargo workspace

**Performance Goals**: At rest, zero frames requested and no measurable CPU. Every animation
settles and releases its state.

**Constraints**: Zero visual change. One sanctioned behavior change (floating-surface dismissal).
Tokens must be nameable from a crate that cannot see the renderer.

**Scale/Scope**: 7 wrapper components, 1 behavior primitive, 5 overlay implementations collapsed
to 1, 13 feature modules migrated, 119 style applications and 135 raw text-size references removed,
1 global enumeration and 2 central animators deleted.

## Constitution Check

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. The boundary test, the builder-API conformance test,
  and the pure decision logic (the overlay dismissal rules) are all written failing first. Wrapper bodies are thin GUI wiring validated by `quickstart.md` under
  the Principle I exception; anything with branching lands in the tested core first (FR-017).
- [x] **II. Multi-Session Support**: PASS. No session-scoped state is added or moved.
- [x] **III. Worktree Integration**: PASS. No file or VCS operation is touched.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing stored, nothing transmitted.
- [x] **V. Rust + iced Stack**: PASS. No widget is forked; wrappers compose the stack's own widgets
  and use its documented per-instance state mechanism.
- [x] **VI. Cross-Platform Parity**: PASS. No platform branch introduced; CI covers all three.
- [x] **VII. Documentation First-Class**: PASS. Developer documentation of the two layers and the
  composition rule ships in the same change (FR-026).
- [x] **VIII. Reusable UI Component Foundation**: PASS — this feature *is* Principle VIII enforced.
  It moves component reuse from convention to a build-failing test, and every wrapper exposes the
  mandated chainable builder terminating in `.into()`.

**Post-Phase-1 re-check**: PASS. The design adds no dependency, no persisted state and no platform
branch. Two items are recorded in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/017-material-component-architecture/
├── plan.md              # This file
├── research.md          # Phase 0 — rendering-stack capability findings
├── data-model.md        # Phase 1 — state ownership model
├── quickstart.md        # Phase 1 — parity validation procedure
├── contracts/
│   └── component-api.md # The durable architecture contract
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code

```text
crates/micold-core/
├── src/
│   ├── tokens/                 # MOVED from the client — same values, relocated (FR-020, FR-021)
│   ├── overlay.rs              # NEW — dismissal rules as pure logic (FR-009, FR-017)
└── tests/
    ├── tokens_move.rs          # values survive the move unchanged
    └── overlay_dismissal.rs    # unified dismissal rules

crates/micold-client/src/ui/
├── cdk/                        # NEW behavior layer — no appearance (FR-006)
│   └── overlay.rs              # ONE overlay: position, backdrop, dismissal, stacking (FR-008)
├── material/                   # appearance layer — with cdk, the only modules naming a widget
│   ├── style.rs                # MOVED here + made internal (FR-002)
│   ├── button.rs               # NEW wrapper
│   ├── text.rs                 # NEW wrapper
│   ├── text_field.rs           # NEW wrapper
│   ├── checkbox.rs             # NEW wrapper
│   ├── scrollable.rs           # NEW wrapper
│   ├── surface.rs              # NEW wrapper
│   └── …                       # existing components, rebased onto cdk
└── {sidebar,shell,about,*_form,confirm_*,project_selector,terminal,mod}.rs
                                # 13 feature modules — compose components only

crates/micold-client/tests/
├── material_boundary.rs        # NEW — build fails if a feature module styles anything (FR-004)
├── material_builder_api.rs     # NEW — every component exposes the mandated builder
├── overlay_stacking.rs         # NEW — deterministic stacking order (FR-010)
├── cdk_no_appearance.rs        # NEW — the behavior layer carries no appearance (FR-007)
├── component_state_isolation.rs # NEW — instances animate independently (FR-011)
└── component_api_opacity.rs    # NEW — no internals in a public API (FR-013)
```

**Structure Decision**: The three-crate workspace is unchanged. Two additions: a `tokens/` module
in the render-free core (a move, not a rewrite), and a `cdk/` behavior layer beneath `material/`.
The token move is friction-free — the current token file already imports only from the core.

## Phase Ordering

| Phase | Delivers | Story |
|-------|----------|-------|
| 1 | Setup; **baseline capture** | — |
| 2 | Tokens moved to the render-free core; dismissal rules as pure logic | — |
| 3 | The `cdk` behavior layer: five overlays become one | US3 |
| 4 | Wrapper components at parity; styling layer made internal | US1 |
| 5 | 13 feature modules migrated onto wrappers | US1 |
| 6 | Presentation state extracted; global enumeration and animators deleted | US2 |
| 7 | Contract check, docs, parity gate, performance, three platforms | — |

Phases 1 → 2 → 4 → 5 are the MVP path. Phase 3 is sequenced early because the overlay
consolidation is self-contained, but nothing in Phase 4 depends on it. Phase 6 can overlap Phase 5
per component.

## Complexity Tracking

| Item | Why needed | Simpler alternative rejected because |
|------|------------|--------------------------------------|
| A separate `cdk` behavior layer for a single primitive | The overlay is used by five components today, each with its own copy — the exact failure this feature exists to fix. Establishing the layer now also gives 018's ripple and density resolver a defined home rather than inventing one under pressure. | Behavior inside each component was rejected: it is the status quo and it produced the divergence. |
| Decision logic (dismissal rules, density arithmetic, ripple geometry) in the render-free core rather than the widget | Principle I requires branching logic to be testable, and the GUI-wiring exception explicitly does not cover it. Keeping *storage* in the widget while *decisions* stay pure satisfies both rules. | Putting it in the widget was rejected: it would be unreachable from tests. Putting the state in the core was rejected separately: it would force callers to allocate identities, which FR-013 forbids. |

## Risks

| Risk | Mitigation |
|------|------------|
| A large diff across 13 modules is hard to review | Zero visual change plus a parity checkpoint turns review into a yes/no question. The boundary test flips to blocking only after the last module migrates, so intermediate states stay buildable. |
| A wrapper subtly changes appearance | FR-005 makes parity the requirement, and `quickstart.md` walks every surface in both schemes against the pre-change build. |
| Unified dismissal surprises users on a surface that behaved differently | Sanctioned and scoped by FR-024 to dismissal only. Called out explicitly in the quickstart so it is confirmed rather than discovered. |
| Extracting presentation state changes persistence | None of the moved fields is serialized — application state is not a persisted type. Verified before planning; SC-006 asserts it anyway. |
