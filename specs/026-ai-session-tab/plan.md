# Implementation Plan: The AI Session as a Tab

**Branch**: `feat/026-ai-session-tab` | **Date**: 2026-08-19 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/026-ai-session-tab/spec.md`

## Summary

The session's AI CLI process joins the terminal tab strip as a right-anchored, unclosable,
icon-labelled tab, and the strip becomes always visible (FR-001–FR-003). Two consequences carry most
of the work. The strip must stay honest once it can overflow — the terminal tabs scroll at their
fixed width, the AI tab is pinned outside that scrolling region, and an edge says when something is
beyond it (FR-002a–FR-002f). And every tab gains a **stopped mark**, shown for exactly the states its
own menu can act on (FR-012–FR-012e), which is what makes the tab menu feature 012 shipped this
morning findable at all.

The technical approach has one organising idea: **derive both the mark and the menu from one
predicate**, generalised from the `attached_process_restartable` that already exists in
`ui/terminal.rs`. That function already encodes the AI-CLI-versus-shell split and already means "the
process is not running"; today it answers only for the *attached* process. Widened to answer for any
process the strip can show, it makes FR-012d's "the mark appears for exactly the states the menu can
act on" true by construction rather than by two matches agreeing — which is the same move that file
already made for `empty_terminal_message`, and for the same reason.

## Technical Context

**Language/Version**: Rust, `stable` (pinned for both entry points by `rust-toolchain.toml`)

**Primary Dependencies**: `iced` 0.14 (widgets, layout, overlay); `micold-core` for the render-free
session model (`SessionLifecycle`, `ShellLifecycle`, `TerminalMode`), design tokens and roles

**Storage**: none added. The strip is a **view** over state the client already holds; nothing is
persisted, including scroll position (spec Assumptions)

**Testing**: `cargo test --workspace` (via `mise run test`); render-free unit tests in
`ui/terminal.rs`'s `mod tests` and `micold-core`; the layout-snapshot gate and its six sub-gates in
`crates/micold-client/tests/`; appearance verified by the repo's `visual-pass` skill

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application — a three-crate Cargo workspace (`micold-core`,
`micold-client`, `micold-daemon`)

**Performance Goals**: no new per-frame work beyond one scrollable viewport; the strip is rebuilt
per render as it is today

**Constraints**:
- **The bar's child list must not vary** (feature 023 FR-008a). A conditional child shifts every
  sibling after it, and iced's positional `Tree::diff_children` then hands a pressed control its
  neighbour's node, so the press is dropped. This governs the stopped mark directly: it must occupy
  a **reserved slot** drawn empty, never a pushed-or-not child.
- **Every tab is one fixed width** (feature 012 FR-004c), derived rather than chosen, and no child
  of a tab may be laid out under `anatomy::button::MIN_TOUCH_TARGET` (012 SC-010). Both are held by
  gates that will run against the AI tab the moment it enters a covered state.
- No protocol change: `PROTOCOL_VERSION` and the daemon are untouched.

**Scale/Scope**: one strip per displayed session; N tabs where N = open instances + 1. The bar is
~1014dp at a 1280dp window and a tab is 136dp on a 144dp pitch, so overflow begins at about five
tabs — which is why FR-002a exists rather than being deferred.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every decision in this feature is a pure function of
  render-free state — which tab is marked, whether a process is stopped, what the menu holds — and
  each lands in `ui/terminal.rs`'s unit tests or a `tests/` gate before its call site. The GUI glue
  exception covers only the `src/ui/` assembly around them, and the geometry that assembly produces
  is itself asserted by the layout-snapshot gates rather than left to review.
- [x] **II. Multi-Session Support**: PASS. FR-011. The strip is derived per displayed session from
  that session's own record; no new state is introduced, so there is nothing to leak. The one piece
  of view state — the menu's target — follows the pattern feature 012 established, keyed by what it
  was opened on.
- [x] **III. Worktree Integration**: PASS (vacuous). No file or VCS operation; sessions keep the
  worktree mapping they already have.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is persisted and nothing leaves
  the device; the feature reads state already in memory.
- [x] **V. Rust + iced Stack**: PASS. The one type-level improvement available is naming the
  strip's members — a tab is *the AI process* or *an instance*, never "an instance id that might
  mean the AI", which is what keeps FR-005's "never zero, never two" representable (see
  `data-model.md`).
- [x] **VI. Cross-Platform Parity**: PASS. No platform-specific code. Wheel scrolling comes from the
  rendering stack's scrollable, which the sidebar already uses on all three; CI covers all three.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md` gains
  the AI tab, the stopped mark and the scrolling strip in the same change; it already documents the
  tab strip and its right-click menu.
- [x] **VIII. Reusable UI Component Foundation**: PASS, and it is the principle doing the most work
  here. Three shared primitives are **extended, not forked**: `material::Scrollable` gains a
  horizontal direction (it is vertical-only today), `material::ActivityBadge` gains a constructor
  that takes an emphasis directly so the stopped mark reuses the dot that already reserves its own
  slot, and the tab itself becomes one builder used for both kinds of tab rather than a second
  bespoke copy for the AI. Each keeps its chainable builder terminating in `.into()`.

### Post-design re-check (after Phase 1)

Re-evaluated against `research.md`, `data-model.md` and the contract. All eight still PASS, and two
are stronger than they were before the design existed:

- **V. Rust + iced** — the design introduced `StripTab`, a closed two-variant enum, so FR-005's
  "exactly one marked tab, never zero, never two" is a total function rather than a rule to keep.
  The alternative the design rejected — overloading `Option<ShellInstanceId>`, where `None` already
  means "no active instance" — is exactly the representable-invalid-state this principle forbids.
- **VIII. Reusable UI** — the design named the extensions concretely (`Scrollable` gains a
  direction, `ActivityBadge` gains an emphasis constructor) and found no case where a fork was
  needed.

**I. Test-First** is worth restating in the form the design gave it: `data-model.md` lists five
derived values, every one a pure function of a session record, so the whole of this feature's
decision-making is reachable from `cargo test` without a renderer. What is left for the GUI
exception is assembly — and even that is measured, since the tab geometry is under
`tests/gates/tab_children_fit.rs`.

One thing the design confirms cannot be gated, recorded here rather than discovered later: **the
edge fade and the stopped mark's legibility are appearance** (research R6). They are verified by the
`visual-pass` skill, and `quickstart.md` §8 names each check. A plan that implied a gate covered them
would be repeating what feature 012 learned twice.

## Project Structure

### Documentation (this feature)

```text
specs/026-ai-session-tab/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── ai-session-tab-ui.md
├── checklists/
│   └── requirements.md  # written by /speckit-specify, re-validated by /speckit-clarify
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── session.rs                     # SessionLifecycle, ShellLifecycle, TerminalMode — read, not changed
└── tokens/                        # roles + anatomy the mark and the tab measure against

crates/micold-client/src/
├── ui/
│   ├── terminal.rs                # the bottom bar, instance_switcher_row, the predicates.
│   │                              #   The feature's centre of gravity.
│   ├── mod.rs                     # overlay mounting for the tab menu
│   ├── material/
│   │   ├── scrollable.rs          # EXTEND: a horizontal direction (vertical-only today)
│   │   ├── activity_badge.rs      # EXTEND: build from a BadgeEmphasis, for the stopped mark
│   │   └── ...                    # Divider, IconButton, Text, Tooltip — reused as-is
│   └── cdk/
│       └── context_area.rs        # the secondary-press wrapper (012 BUG-005) — reused as-is
├── app.rs                         # the menu's target widened to name the AI process
├── features/session.rs            # the menu's floating-surface registration
└── icons.rs                       # Icon::AiCli already exists (FR-009 needs no new glyph)

crates/micold-client/tests/
├── terminal_tabs.rs               # the strip's call site
├── terminal_bar_stability.rs      # the bar's child list must not vary
├── layout_snapshot.rs             # + gates/tab_children_fit.rs — run against the AI tab
├── support/covered_states.rs      # the covered state the strip is rendered into
├── showcase_completeness.rs       # C3/C4: new variants must be posed
└── overlay_registration.rs        # the menu is a popover and must be in POPOVERS
```

**Structure Decision**: no new crate, no new module tree. The feature is concentrated in
`crates/micold-client/src/ui/terminal.rs`, which already owns the bar and the strip, with three
shared components in `ui/material/` extended in place per Principle VIII. The render-free decisions
(which tab is marked, whether a process is stopped, what its menu holds) stay as pure functions in
that file's testable surface, matching how `attached_process_restartable` and `restart_message`
already live there.

## Complexity Tracking

> No constitution violations to justify.

One scope note that is not a violation but should be visible: **FR-012 changes feature 012's
terminal tabs**, not only the new AI tab. That is deliberate and forced by FR-010 — the mark must be
the same on both kinds of tab or it is not a strip — but it means this feature edits a control
another feature owns, and its regression cover (`tests/terminal_tabs.rs`, the tab gates, 012's
`quickstart.md` §8) belongs to 012. The tasks must run 012's §8 as well as this feature's own pass.

A second, recorded in `research.md` R7: **FR-002c documents a defect that is live on `main` today**.
Past about five instances the bar's trailing controls are silently squeezed — the same failure mode
as 012 BUG-005, one level out. This feature does not cause it, meets it sooner, and fixes it.
