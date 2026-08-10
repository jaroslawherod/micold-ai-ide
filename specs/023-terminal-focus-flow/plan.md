# Implementation Plan: Natural Terminal Focus Flow

**Branch**: `feat/improved-focuse-management` (spec dir `023-terminal-focus-flow`) | **Date**: 2026-08-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/023-terminal-focus-flow/spec.md`

## Summary

Terminal focus stops being a flag the application pushes around on every click and becomes a
question it answers. `State.terminal_focused` — a `bool` written from seven scattered places and
cleared by a widget whenever a press landed outside the pane — is replaced by
`State.terminal_released` (one deliberate user decision) plus a derived predicate
`State::terminal_focused()` that reads: *a session is displayed, the user has not released it, and
nothing that types has taken the keyboard.*

That single change delivers most of the spec. Presses on non-typing controls stop touching focus
because the release rule they tripped is deleted (FR-005/FR-006). A field or dialog taking the
keyboard already reports itself through `Message::FieldFocusChanged` / `State.overlay`, so the
terminal yields to it and takes it back when it finishes, with no restore stack (FR-004/FR-010/
FR-017). Window return needs no mechanism at all: nothing changed while the window was away, so
there is nothing to restore (FR-013–FR-015). A closed session unfocuses by derivation, not by
remembering to clear a flag (FR-012/FR-016).

Two defects survive that change and are fixed on their own terms: the pressed control whose click
is swallowed (root cause in [research.md](./research.md) R1 — a focus-conditional child in the
terminal's bottom bar shifts its siblings mid-click and iced's positional tree diff drops the
`is_pressed` state the release depends on), and the press into an unfocused pane that grants focus
but is not reported to a mouse-aware program (FR-008b).

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: `iced` 0.14 (`advanced` widget API — the feature lives in `Widget::update`
and the reducer), `alacritty_terminal` (term modes for mouse reporting). No new dependency.

**Storage**: None. Both focus facts are transient and deliberately unpersisted — `terminal_released`
is one moment's decision (spec Clarification Q1) and launch re-derives focus from what is displayed
(FR-012a).

**Testing**: `cargo test` via `mise run test` (workspace) and `mise run test-core`. New and extended
integration tests in `crates/micold-client/tests/`. The `src/ui/` edits fall under Principle I's
GUI-wiring exception and are validated by `quickstart.md` Part B, run headlessly with the repo's
`visual-pass` skill.

**Target Platform**: Linux, macOS, Windows desktop (parity required; no platform branch added).

**Project Type**: Desktop application — Rust + iced workspace (`micold-core`, `micold-client`,
`micold-daemon`). This feature touches `micold-client` only.

**Performance Goals**: No new work per frame. The derived predicate is a handful of field reads on
an existing `&State`; it replaces a stored bool, so view and event handling cost is unchanged.

**Constraints**: The keyboard holder must never pass through a state the user did not ask for
(FR-008a) — which rules out the obvious "release then re-assert" implementations, including the two
`Task::done(Message::TerminalFocused)` re-assertions in `main.rs` that BUG-001 added to win exactly
that race. Those are deleted rather than extended.

**Scale/Scope**: One reducer (`src/app.rs`), one shared widget (`src/ui/material/terminal_pane.rs`),
one screen (`src/ui/terminal.rs`), two call sites in `src/main.rs`, one predicate in `src/ui/mod.rs`,
one helper in `src/features/session.rs`. 26 functional requirements; ~7 files; no new module.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: Every decision in this feature — the derived predicate,
      which messages clear a release, how a press routes when it is the press that grants focus — is
      pure and lands in tested logic: the predicate and the navigation set in `app.rs`
      (`tests/terminal_focus.rs`), the press's own answer in `ui/material/terminal_pane.rs`'s pure
      `press_routing`, whose truth table is already unit-tested inline in that file's `mod tests`
      and gains the focus-granting case there — it is `pub(crate)`, so an integration test cannot
      reach it and inline is the only correct home. The question *whether* a press grants focus is
      not left as a branch in `Widget::update` either: it becomes `press_grants_focus(focused,
      is_left_press, over_bounds)`, pure and unit-tested beside `press_routing`, because the
      exception does not cover code with a rule of its own wherever it happens to sit. The same
      `mod tests` also asserts that no press outside the pane's bounds produces a
      `TerminalAction(Write(..))` — FR-003/SC-008's only mechanical evidence. What remains in
      `src/ui/` is wiring with no rule of its own
      (`.focused(state.terminal_focused())`, deleting a publish, pushing a button unconditionally) —
      the GUI-wiring exception, validated by `quickstart.md` Part B. The exception's precondition is
      itself checked: `tests/terminal_bar_stability.rs` fails if the bottom bar branches on focus
      again, following the precedent `tests/showcase_glue.rs` set for `src/showcase/`.
- [x] **II. Multi-Session Support**: No per-session state is added, and none is persisted. Focus is
      a property of *the displayed* terminal: `terminal_focused()` is false whenever
      `active_session` is `None`, so a background session can never hold the keyboard (FR-020) and
      no session leaks a focus state into another. The one global fact, `terminal_released`, is a
      user decision about the present moment — the spec settled this deliberately (Clarification Q1).
- [x] **III. Worktree Integration**: Untouched. No file or VCS operation, no worktree or session
      lifecycle change; sessions keep mapping to a worktree or the Default project root exactly as
      before.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Nothing is stored, read, or transmitted. Both
      focus facts live in memory for the lifetime of the process.
- [x] **V. Rust + iced Stack**: Rust + iced only. The type system does the enforcing: deriving
      `terminal_focused()` from `active_session`, the overlay registry, and `focused_field` makes "focused with
      no session displayed" and "focused while a text field is typing" unrepresentable, where today
      they are runtime rules seven assignments must all remember. That is a state machine deleted,
      not added.
- [x] **VI. Cross-Platform Parity**: No `cfg(target_os)` and no platform-conditional behaviour. The
      reserved release chord already resolves per platform inside `keymap`, unchanged here. CI runs
      the suite on all three.
- [x] **VII. Documentation First-Class**: `docs/user-guide/worktrees-and-sessions.md` documents the
      focus rules today and is updated in the same change — the default-holder model, what does and
      does not take the keyboard, and that the release chord persists until you give it back or
      navigate.
- [x] **VIII. Reusable UI Component Foundation**: No new widget and no fork. `TerminalPane` is an
      existing shared component and keeps its chainable builder shape (`.focused(...)` is amended in
      behaviour, not in signature); the bottom bar reuses `IconButton`/`Tooltip`; field focus reuses
      `TextField::track_focus`, which BUG-003 landed for exactly this purpose.

## Project Structure

### Documentation (this feature)

```text
specs/023-terminal-focus-flow/
├── plan.md              # This file
├── research.md          # Phase 0 — root cause, the derived-holder decision, alternatives
├── data-model.md        # Phase 1 — state shape, invariants, transition table
├── quickstart.md        # Phase 1 — Part A automated gates, Part B visual pass
├── contracts/
│   └── focus-model.md   # Phase 1 — supersedes specs/006-real-terminal-emulator/contracts/focus-model.md
├── checklists/
│   └── requirements.md  # From /speckit-specify
├── tasks.md             # /speckit-tasks output — not created here
├── visual-pass-baseline.md  # The pre-fix two-press behaviour, recorded before any code changes
└── visual-pass.md           # The recorded §B pass, one section per story
```

### Source Code (repository root)

```text
crates/micold-client/
├── src/
│   ├── app.rs                        # State.terminal_released (replaces terminal_focused),
│   │                                 # State::terminal_focused() predicate, pub(crate)
│   │                                 # focus_terminal()/release_terminal(), navigation arms,
│   │                                 # route_key (unchanged)
│   ├── main.rs                       # delete both Task::done(Message::TerminalFocused) re-asserts
│   ├── features/
│   │   └── session.rs                # restore_after_activation: project switch focuses (FR-011)
│   └── ui/
│       ├── mod.rs                    # subscription() gate reads the predicate
│       ├── terminal.rs               # .focused(predicate); bar child list independent of focus
│       └── material/
│           └── terminal_pane.rs      # delete click-outside release; new pure press_grants_focus();
│                                     # granting press acts (FR-008b); its inline `mod tests` gains
│                                     # the granting-press cases and the no-input-outside assertion
└── tests/
    ├── terminal_focus.rs             # extended: predicate truth table, navigation, bounds
    └── terminal_bar_stability.rs     # new: source gates — the bar must not branch on focus, and
                                      # `terminal_released` is written only by the two helpers

docs/user-guide/worktrees-and-sessions.md   # Principle VII deliverable
```

**Structure Decision**: The existing workspace layout is unchanged and no module is added. Decision
logic goes to `crates/micold-client/src/app.rs`, the crate's render-free reducer, which is where
Principle I requires it and where `tests/` can reach it. `micold-core` is deliberately not touched:
focus is a property of the client's window, not of the session model the daemon shares.

## Complexity Tracking

No constitutional violations to justify. Recorded for the reviewer instead: this plan **removes**
state rather than adding it — one stored bool and seven assignments become one stored bool and one
derived predicate — and deletes two `Task::done` workarounds. The one place it adds machinery is
`tests/terminal_bar_stability.rs`, a source-level gate that exists because Principle I's GUI-wiring
exception has a precondition ("no decision logic of its own") that nothing would otherwise check —
the same reasoning, and the same shape, as `tests/showcase_glue.rs` under constitution 1.5.0.
