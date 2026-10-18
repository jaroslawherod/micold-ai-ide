# Implementation Plan: Overview Card Ripple

**Branch**: `claude/project-thread-kqz6b6` | **Date**: 2026-10-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/971-overview-card-ripple/spec.md`

## Summary

Wrap the activatable `OverviewCard` in the existing `material::Ripple` (feature 018) so a pointer
press ripples from the press point, add an opt-in keyboard origin to that wrapper so Enter or Space
on a focused card ripples from the centre, and hold the Overview open for the ripple's expand time
(300 ms) between an activation and its close, ignoring input meanwhile. No new crate, no wire or
storage change.

## Technical Context

**Language/Version**: Rust, stable (via `mise`)

**Primary Dependencies**: iced 0.14 (already in the workspace); `tokio::time::sleep` (already used
in `crates/micold-client/src/shell/pr_status.rs`); no new crate

**Storage**: N/A (the pending activation is transient window state, never persisted)

**Testing**: `cargo test` via `mise run gate`: client-crate unit tests in
`ui/material/ripple.rs`, `ui/material/overview_card.rs`, `features/overview.rs`; the rasterised
clipping check `ui/material/ripple_clipping.rs`; quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows desktop (iced)

**Project Type**: desktop-app (Rust workspace; change is in `micold-client` only)

**Performance Goals**: ripple frames only while it runs; idle draws no frames (SC-004)

**Constraints**: delay = `duration::MEDIUM_2` (300 ms, ±1 frame); outcome of an activation unchanged

**Scale/Scope**: one widget, one feature state, one root handler, one showcase sample, one doc page

## Constitution Check

- [x] **I. Test-First**: every change below has its failing test listed in the Test strategy; tasks order test before code.
- [x] **II. Multi-Session**: ripple state is per card instance (the tree state of each `Ripple`); no shared or global state. The pending activation is one value for the window's Overview, which is one view.
- [x] **III. Worktree Integration**: N/A — no file/VCS operation; the activation's outcome (select/reveal/offer) is unchanged (FR-008).
- [x] **IV. Local-First**: no storage, no network.
- [x] **V. Rust + iced**: iced only. The pending activation is `Option<Activation>`, so "two activations in flight" is unrepresentable.
- [x] **VI. Cross-Platform**: platform-neutral widget code and a timer; no `cfg`.
- [x] **VII. Documentation**: `docs/user-guide/overview.md` gains a line on the click feedback and the brief delay, in the same change.
- [x] **VIII. Reusable UI Foundation**: reuses `material::Ripple`; the keyboard origin is a builder method on that shared wrapper (not a card-local fork), terminating in `.into()`.

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/971-overview-card-ripple/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ui-contract.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-client/src/
├── features/overview.rs            # State.activating, Msg::ActivationDue, update gating (FR-007)
├── ui/material/ripple.rs           # Ripple::centred_on_keys builder; centred press on captured Enter/Space (FR-002, FR-005)
├── ui/material/overview_card.rs    # wrap the pressable in Ripple, tint palette.text, RADIUS (FR-001, FR-003, FR-006)
├── ui/overview.rs                  # card on_press emits ActivationRequested (FR-007)
├── showcase/sections/surfaces.rs   # sample card gets on_press (FR-009)
└── main.rs                         # overview_message: start the 300 ms timer, finish on ActivationDue (FR-007, FR-008)
docs/user-guide/overview.md
```

**Structure Decision**: edits in the existing `micold-client` modules; nothing new in `micold-core`
(the delay is UI timing, not a rule).

## Design

### D1 Pointer ripple (FR-001, FR-003, FR-006)
In `OverviewCard::into`, the activatable branch returns
`Ripple::new(focusable, p.text, RADIUS)` around the `TakesTheKeyboard`/`button` stack. The wrapper
already starts on `ButtonPressed(Left)` over its bounds with `cursor.position_in`, clamps the origin
(`cdk::ripple::Ripple::press`), clips to the rounded shape with `shape_bands`, and draws at the
pressed opacity. The plain-card branch returns `face` unwrapped, so it never ripples. Ripple is
outermost so it also bracket-captures the focus indicator's layer: `beneath_ripple` makes the ripple
the only state layer (FR-004, feature 018 FR-024h).

### D2 Keyboard origin (FR-002, FR-005)
`Ripple` gets `centred_on_keys(self) -> Self` (default off, so other controls are unchanged,
Assumptions). In `update`, after forwarding the event to the child: when enabled, the event is
`KeyPressed` with `Named::Enter` or `Named::Space`, the shell had not captured the event before the
child ran and has after, and the ripple is idle, call `state.press(None, bounds.size())` (None →
centre, FR-024b). Only a focused card publishes and captures (see `keyboard_focus.rs#update`), so an
unfocused card cannot ripple (US2-3); `!is_idle()` makes auto-repeat a no-op (FR-005). The ripple
starts on the key press itself (clarification 1). Pointer presses are unchanged (restart from the
new point).

### D3 Delay before close (FR-007, FR-008)
`CardActivated(a)` keeps today's meaning (close at once, `LocationRevealed`); the existing feature
813 tests that send it (`tests/overview.rs` activation, reveal and offer cases) stay unchanged
(SC-006). The card instead emits a new `Msg::ActivationRequested(a)` (`ui/overview.rs`, where it
now builds `CardActivated`).

`features::overview::State` gains `activating: Option<Activation>` and `fn request(&mut self, a)
-> bool` (pure): when `open` and `activating.is_none()` it stores `a` and returns true; otherwise
false. `update`:
- `ActivationRequested(a)`: `update` itself does nothing (no outcome, the Overview stays open); acceptance is the separate pure `request`, so the decision is unit-tested rather than left in `main.rs`.
- `ActivationRequested`, `Closed`, `Toggled` while `activating.is_some()`: ignored (US3-3). Accepted
  consequence: a `Closed` sent by another path inside the 300 ms (for example a session selected
  elsewhere) is dropped too; the pending activation then closes the view itself.
- `ProjectSwitched` clears `activating`.

`State` also gets the pure `fn due(&mut self) -> Option<Activation>` (`activating.take()`, so
`activating` is `None` before anything else runs and the follow-up's own `Closed` at `main.rs:916`
is not ignored). Both `request` and `due` are tested red-first; `main.rs::overview_message` keeps no
decision beyond two `if let`s: on `ActivationRequested(a)` it calls `request` and, when true, returns
`Task::perform(tokio::time::sleep(OVERVIEW_ACTIVATION_DELAY), |_| ActivationDue)`. On
`ActivationDue` it calls `due()` and, if `Some(a)`, handles
`Msg::CardActivated(a)` through the existing path unchanged (`follow_up`, `revealed_scroll`,
`OfferStart` chain), so the outcome is identical (FR-008). `ActivationDue` with nothing stored (`due()` is `None`) is a
no-op. `OVERVIEW_ACTIVATION_DELAY = Duration::from_millis(duration::MEDIUM_2)`, the token `ripple.rs`
uses for EXPAND, so they cannot drift.

Other cards may still ripple on a pointer press during the delay (feedback only; their
`ActivationRequested` is ignored, so no outcome). Enter, Space and Escape reach no outcome; focus
moves by arrow keys are unaffected.

### D4 Showcase (FR-009)
`overview_card` in `surfaces.rs` gives its sample cards `.on_press(Message::…)` with a no-op/existing
inert showcase message so pointer and keyboard ripples can be seen without closing anything.

### FR → design map
FR-001 D1 · FR-002 D2 · FR-003 D1 · FR-004 D1 (`beneath_ripple`) · FR-005 D1 + D2 (per-instance tree
state) · FR-006 D1 · FR-007 D3 · FR-008 D3 · FR-009 D4.

## Test strategy

| Requirement | Layer | Test |
|---|---|---|
| FR-001, SC-001 | client unit, `ui/material/overview_card.rs` (Mounted harness, `test_support`) | press at four corners + centre; assert ripple origin via `pulse`/state |
| FR-002, SC-002, US2-3 | client unit, same | Enter and Space on focused card → origin centre; unfocused → idle |
| FR-003, SC-003 | `ui/material/ripple_clipping.rs` (rasterised) | card-shaped, RADIUS 12 case |
| FR-004 | client unit in `ripple.rs` | existing `beneath_a_ripple…` plus a focused-card case |
| FR-005, SC-004 | client unit + `tests/ripple_state.rs` | two cards independent; repeat key no restart; idle after fade, no frames |
| FR-006 | client unit | plain card press/Enter leaves no ripple |
| FR-007 | `features/overview.rs` unit (pure `request`/`due`/`update`) | accept → pending; second request/Closed/Toggled ignored; `due` takes once, then `None`; `ProjectSwitched` clears; delay constant equals MEDIUM_2 |
| SC-005 (300 ms ±1 frame, ripple seen) | quickstart §B visual pass: only the wall-clock timing is manual; the constant and the gating are unit-tested | |
| FR-008, SC-006 | existing 813 `CardActivated` tests unchanged; `ui/overview.rs` test that a card emits `ActivationRequested` with its `activation` | |
| FR-009, SC-005 | quickstart §B visual pass (showcase and client) | `visual-pass` skill |

## Complexity Tracking

None.
