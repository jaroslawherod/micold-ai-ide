# Implementation Plan: Daemon settings in an overlay with their own Save

**Branch**: `feat/1092_daemon-settings-should-open-in-an-overlay-with-its` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1092-daemon-settings-overlay/spec.md`

## Summary

Move the Settings > Daemons add/edit form, and the per-row Details block, out of the page and into
two modal dialogs on the existing floating-surface registry (`overlay::registry`). The daemon
reducer (`features::daemons`) already holds the form as `State::draft`, already saves it
independently of the page save (`Msg::Saved`, `save_checked`) and already has Cancel (`Msg::DraftDiscarded`).
So this is a presentation and dispatch change: the draft being `Some` *is* "the overlay is open".

Technical approach (details in [research.md](research.md)):

1. Register `DaemonFormDialog` (open while `daemon_list.draft` is `Some`) and `DaemonDetailsDialog`
   (open while the new `daemon_list.details` is `Some`) in `overlay/registry.rs`, each drawn by a
   `ui::` function. Escape and the scrim click come free from the registry (FR-005, FR-006).
2. Add one shared builder component, `material::ScrollingDialog`, for a dialog whose body scrolls
   and whose action row stays pinned (FR-011); the existing `material::dialog::confirm` cannot
   hold a large form (Principle VIII).
3. Render the counted confirmation for a running daemon's runtime edit (`Pending::Edit`) *inside*
   the form dialog instead of as the separate `ConfirmDaemonRemovalDialog` (FR-008). The form
   dialog's cancellation is state-dependent: Escape discards while editing, answers no while
   confirming.
4. Gate system-raised prompts (Kubernetes drift) behind the open overlay, queued until it closes (spec edge case).
5. Return focus to the opening control on close (FR-009); verify and, if needed, scope Tab
   traversal to the overlay.
6. Replace `details_shown: BTreeSet<DaemonId>` / `Msg::DetailsToggled` with one open `details`
   target (`DetailsOpened(id)` / `DetailsClosed`) (FR-012).
7. Update `docs/user-guide/settings.md` (Principle VII).

No new storage, no new crate, no `micold-core` change.

## Technical Context

**Language/Version**: Rust stable (workspace toolchain via `mise`), iced.

**Primary Dependencies**: existing only: `iced`, `micold-core`, `micold-client` internals
(`overlay::registry`, `ui::material`, `ui::cdk::overlay`, `features::daemons`).

**Storage**: unchanged. `Msg::Saved` already writes the daemon registry through
`Outcome::DaemonRegistryChanged`; the overlay draft is in-memory only (`State::draft`).

**Testing**: `cargo test` via `mise run test-core` is not enough here (no core change); client
tests via `mise run test`, gate via `mise run gate`. Layers in "Test strategy" below.

**Target Platform**: Linux, macOS, Windows, identical behaviour (no platform branch).

**Project Type**: desktop app (Rust workspace: `micold-core`, `micold-client`, `micold-daemon`).

**Performance Goals**: none beyond the existing dialog transition; the Details dialog re-renders on
the existing daemon log/error messages.

**Constraints**: the form dialog must fit the smallest supported window with Save daemon and Cancel visible (SC-005).

**Scale/Scope**: one settings section, two dialogs, about 6 files of production change.

## Constitution Check

*Pre-research gate; re-checked after Phase 1 design: unchanged, all PASS.*

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Test-First | PASS | Decision logic (what is open, confirmation placement, escape target, prompt gating, details target, stale-daemon save) lives in `features::daemons` / `overlay::registry`, both render-free and under `crates/micold-client/tests/`; each gets a failing test first. Only the `ui/` layout glue falls under the GUI exception and is covered by geometry gates plus [quickstart.md](quickstart.md) Part B. |
| II | Multi-Session | PASS | No session state touched. Cross-window registry change is covered by FR-014 (draft untouched; stale save refused). |
| III | Worktree | PASS | N/A: no file or VCS operation. |
| IV | Local-First | PASS | Same local registry write as today; nothing remote. |
| V | Rust + iced | PASS | iced only. "Overlay open" is derived from `draft: Option<Draft>` / `details: Option<DaemonId>` rather than a parallel flag, so open-with-no-draft is unrepresentable. |
| VI | Cross-Platform | PASS | No OS branch; Escape and pointer paths are the shared ones. CI already runs all three. |
| VII | Documentation | PASS | `docs/user-guide/settings.md` Daemons section updated in the same change (task in tasks.md). |
| VIII | Reusable UI | PASS | New `material::ScrollingDialog` builder (struct, chainable, `From<..> for Element`, takes `Roles`) instead of a feature-local layout; `Surface`/`Modal`/`dialog::fields`/`actions` reused. |

## Requirements coverage

| Requirement | Where |
|---|---|
| FR-001, FR-006 | `DaemonFormDialog` registration; `ui::settings::daemons::view` no longer pushes `form(..)`; `Modal` blocks the page |
| FR-002, FR-004 | `ScrollingDialog` actions row (Save daemon / Cancel); `Msg::Saved` unchanged, `form_error` shown by `form()` |
| FR-003 | `save` / `save_checked` unchanged; overlay closes because `draft` becomes `None` |
| FR-005 | registry dismissal: Escape via `on_escape`, scrim via `Modal::on_dismiss`; Cancel button sends `DraftDiscarded` |
| FR-007 | no change to `features::settings`; regression tests on page-save dirtiness |
| FR-008 | `State::form_confirmation`, state-dependent `DaemonFormDialog::dismissal`, `ConfirmDaemonRemovalDialog` excludes `Pending::Edit` |
| FR-009 | `Opener` remembered in `daemons::State`; focus task from `shell/daemons.rs`; traversal check in R5 |
| FR-010 | `form()` reused as the dialog body; per-kind sections untouched |
| FR-011 | `ScrollingDialog` (scrollable body, pinned actions, window-fitting height) |
| FR-012 | `DaemonDetailsDialog`, `details_block` reused, `DetailsOpened`/`DetailsClosed`; row loses Hide details |
| FR-013 | row actions otherwise unchanged; removal flow unchanged |
| FR-014 | no reducer path replaces `draft` on `DaemonRegistryChanged`; stale-id save refused with a message (R6) |
| SC-001..SC-006 | tests listed below; see [quickstart.md](quickstart.md) |

## Project Structure

### Documentation (this feature)

```text
specs/1092-daemon-settings-overlay/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── ui-contract.md
└── tasks.md            # next unit
```

### Source code (existing paths; "new" marked)

```text
crates/micold-client/src/
├── features/daemons.rs          # State: details target, opener; form_confirmation; gating; DaemonFormDialog, DaemonDetailsDialog (new)
├── overlay/registry.rs          # two register! lines; surface-count comments
├── ui/
│   ├── mod.rs                   # (re-exports)
│   ├── daemon_form_dialog.rs    # new: dialog view for the form (+ confirmation panel)
│   ├── daemon_details_dialog.rs # new: dialog view for Details
│   ├── confirm_daemon_removal.rs# dialog() skips the in-form confirmation
│   ├── material/scrolling_dialog.rs  # new: ScrollingDialog builder
│   └── settings/daemons.rs      # drop inline form/details_block placement; Details button opens dialog
├── shell/daemons.rs             # focus-return task after close
crates/micold-client/tests/      # see Test strategy
docs/user-guide/settings.md      # Daemons section
```

**Structure Decision**: single existing crate pair; no new crate. `form()` and `details_block()`
stay in `ui/settings/daemons.rs` (made `pub(crate)`) and the two new dialog modules call them, so the
form is not forked.

## Test strategy

| Layer | Covers |
|---|---|
| Client reducer/registry tests (`tests/features_daemons.rs`, `tests/overlay_registry.rs`, `tests/popover_displacement.rs`, `tests/surface_registration_cost.rs`) | US1-US5 logic: edit/add open and save/cancel, validation keeps it open, confirmation in-form and Escape = No, drift prompt queued while open, details open/close, stale-daemon save, draft survives `DaemonRegistryChanged`, page-save independence (FR-007, SC-004), registry lists the two new surfaces (existing counts 20 -> 22) |
| Geometry gates (`tests/daemons_section_geometry.rs` + a new dialog case) | no inline form or Details block in the page (SC-006); Save daemon and Cancel inside the window at the smallest supported size (SC-005); `ScrollingDialog` pins actions |
| Widget tests (`ui/material` test support) | `ScrollingDialog` scrolls its body, keeps actions, honours width |
| Client reducer tests (FR-009 part) | `Opener` set on open and taken on close; the focus move itself is §B |
| Quickstart §B visual pass | look, focus ring, Tab cycle, focus return, narrow window |

## Complexity Tracking

No constitution violations.
