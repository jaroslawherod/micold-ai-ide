# Implementation Plan: Reveal the current session in the sidebar

**Branch**: `024-reveal-current-session` | **Date**: 2026-08-09 | **Spec**: [spec.md](./spec.md)

**Bugfix**: 2026-08-19 — [BUG-001](./bugs/BUG-001.md) Updated from bugfix patch: the file tree
named `tree_view.rs` and `text.rs` as the whole of the 500-weight name, and left out the widget
that actually draws it. `ui/material/ellipsized.rs` is added below as the third file FR-003a
depends on.

**Input**: Feature specification from `/specs/024-reveal-current-session/spec.md`

## Summary

The mark already exists and is never seen. `TreeItem::selected` draws the `secondary_container`
pill on the session row whose id equals `State::active_session` (`ui/sidebar.rs:455`,
`ui/material/tree_view.rs:437`) — but on a project switch `restore_after_activation` clears
`default_expanded` and `set_worktrees` prunes `expanded` to the incoming project's directory names,
so the row carrying that pill is not in the tree at all. The feature is three changes over existing
machinery, not a new subsystem:

1. **Derive the open row.** `expanded` / `default_expanded` stay the *user's* set. One new field,
   `reveal_suppressed_for: Option<SessionId>`, is all the state the reveal needs; the location
   holding the current session reads as open unless the user has closed it for that session
   (FR-001b, FR-005). Ordering and pruning stop mattering, because nothing is stored to lose.
2. **Exempt one location from the filters**, carrying a chip that says why it is there
   (FR-011/FR-012/FR-012a) — `filtered_worktree_tree` gains an exemption predicate and
   `WorktreeNode` a `shown_for_current_session` flag.
3. **Scroll it into view** (FR-008/FR-009). Row heights are already deterministic
   (`density::height(base, step)` plus `spacing::XS`, asserted in `anatomy_size.rs`), so a pure
   function over the ordered rows decides whether and where to scroll; `main.rs` turns the answer
   into `iced::widget::operation::scroll_to`.

The non-colour half of the mark (FR-003a) is a fourth, smaller change: the current row's name
renders at the type scale's 500 weight while other session rows stay at 400.

> **BUG-001 (2026-08-19)**: this was built as a role selection and stopped there. `Ellipsized`,
> which draws every session name, takes a role's *size* and discards its font, so the two roles are
> indistinguishable on screen. The change is smaller than it looked, but it is one file wider.

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: iced 0.14 (`tokio`, `canvas`, `advanced`, `lazy`). No new dependency —
`iced::widget::operation::scroll_to` and `iced::widget::Sensor` both ship in 0.14.

**Storage**: None. Nothing about the reveal is persisted (spec Assumptions); the one new field is
in-memory view state, so Principle IV is satisfied by having nothing to store.

**Testing**: `cargo test --workspace` via `mise run test`; `mise run test-core` for the render-free
core. Reducer and projection tests are unit tests beside the code; the manual half is
[quickstart.md](./quickstart.md) §B, runnable by the repo's `visual-pass` skill.

**Target Platform**: Linux, macOS, Windows desktop (CI covers all three).

**Project Type**: Desktop application — existing three-crate workspace, no new crate.

**Performance Goals**: No frame-budget change. The reveal adds one `Option` comparison per row to a
projection the sidebar already rebuilds each view, and at most one `scroll_to` task per change of
current session.

**Constraints**: The scroll target is computed from row geometry, so the computed height must equal
the rendered height — the one real risk in the feature, addressed in [research.md](./research.md) R6
and gated by a test that shares `anatomy_size.rs`'s figures.

**Scale/Scope**: SC-003's figure — a project with 30 locations — is the sizing case. Six files in
`micold-client`, one user-guide page, no change to `micold-core`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every decision this feature makes lands in a
  render-free module and is driven by a failing test first: the effective-open predicate, the
  filter exemption and its flag, the row-metric/scroll-target function, and the suppression rules
  all live in `features/sidebar.rs` and `app.rs`. Only three things fall under the GUI-wiring
  exception, and none of them decides anything: the chip and the 500-weight name in
  `ui/sidebar.rs` / `ui/material/tree_view.rs`, the `Sensor` and `Id` on the `Scrollable` wrapper,
  and `main.rs` draining `pending_reveal_scroll` into a task. They are covered by quickstart §B.
- [x] **II. Multi-Session Support**: PASS. No session is created, stopped, or mutated; the reveal
  reads `Session::location` and nothing else. `reveal_suppressed_for` holds a `SessionId` but adds
  no per-session persisted state, so nothing new can leak between sessions.
- [x] **III. Worktree Integration**: PASS. No worktree is created, switched, or removed. Both
  sanctioned locations are handled symmetrically — the same predicate answers for a worktree and
  for Default, which is what keeps FR-001 from being a worktree-only feature.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is written or read; nothing
  leaves the device. The reveal is derived per run.
- [x] **V. Rust + iced Stack**: PASS. iced only. The types make the illegal states unreachable
  rather than guarded: "revealed for a session that is not current" cannot be expressed, because
  revealed-ness is not stored at all — it is derived from `Option<SessionId>` — and the location of
  the current session is expressed through the existing `SessionLocation` enum rather than a
  stringly-typed pair of a directory name and a bool.
- [x] **VI. Cross-Platform Parity**: PASS. No platform branch; row geometry and scroll offsets are
  the same arithmetic everywhere. CI covers all three.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md` gains
  the reveal, the filter exemption and its chip, and the fact that closing the row sticks — in the
  same change, in the two sections that already describe the behaviour being changed:
  `## Starting, switching, and closing sessions` and `### Filtering worktrees by tag`.
- [x] **VIII. Reusable UI Component Foundation**: PASS. Nothing is forked. Three shared primitives
  gain chainable builder methods terminating in the existing `.into()`: `Scrollable::id(...)` and
  `Scrollable::on_viewport_resize(...)`, and the current-row weight inside `TreeView`'s existing
  `selected` path. The FR-012a chip reuses `TreeItem::tags`, the slot worktree rows already use.
  No free functions with positional parameters are added.

Re-checked after Phase 1 design: unchanged, all PASS. The design added no new component, no new
crate, and no new stored state; the largest new surface is one pure function
(`scroll_target`) and it sits in a tested module.

## Project Structure

### Documentation (this feature)

```text
specs/024-reveal-current-session/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── sidebar-reveal.md      # The reveal's invariants and the effective-open predicate
│   └── scrollable-viewport.md # The Scrollable wrapper's new id/viewport surface
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-core/                 # UNCHANGED — see research.md R9
└── src/session.rs                  # `SessionLocation` is read, not modified

crates/micold-client/src/
├── app.rs                          # +reveal_suppressed_for, +sidebar_viewport_height,
│                                   # +pending_reveal_scroll; every app-initiated change of
│                                   # active_session commits the outgoing row, and one to Some
│                                   # arms (contract §3.0/§3.0a, research R12);
│                                   # commit-on-clear (FR-001c); suppression on user collapse
├── features/
│   └── sidebar.rs                  # effective-open predicate; filter exemption +
│                                   # shown_for_current_session; row metrics; scroll_target
├── ui/
│   ├── sidebar.rs                  # glue: the FR-012a chip, the Scrollable's id + viewport
│   └── material/
│       ├── scrollable.rs           # +.id(), +.on_viewport_resize() (Sensor-backed)
│       ├── tree_view.rs            # the current row's 500-weight name (FR-003a)
│       ├── ellipsized.rs           # draws that name; must carry the role's font, not only its
│                                   #   size (BUG-001)
│       └── text.rs                 # the 500-weight sidebar-session role
└── main.rs                         # drains pending_reveal_scroll into operation::scroll_to

crates/micold-client/tests/         # integration coverage per quickstart §A
docs/user-guide/worktrees-and-sessions.md
                                    # `## Starting, switching, and closing sessions` (:313) — the
                                    #   reveal, the mark, and that closing the row sticks
                                    # `### Filtering worktrees by tag` (:47) — the one exempt row
                                    #   and the chip that says why it is there
```

**Structure Decision**: The existing `micold-core` / `micold-client` / `micold-daemon` workspace,
unchanged. The feature is entirely client-side view logic: `micold-core` owns sessions and
worktrees and neither gains a field, because which row a panel draws open is not a fact about a
session (research.md R9). Within the client the split already in place is honoured — decisions in
`features/sidebar.rs` and `app.rs` (render-free, tested), rendering in `ui/` (quickstart-validated
per Principle I's exception).

## Complexity Tracking

> No constitution violations. The table is left in place, empty, because the two places this design
> was tempted into complexity are worth recording as *rejected*:

| Considered | Why it looked necessary | Why it was not taken |
|-----------|------------------------|---------------------|
| A per-project `BTreeMap<PathBuf, BTreeSet<String>>` of revealed rows | FR-007 asks that reveals not carry between projects, which reads like per-project storage | Deriving from the current session gives FR-007 for free — there is one current session, so there is one revealed location, and no map can drift from it (research.md R2) |
| `Tag::Current` in `micold-core::naming` | FR-012a's chip renders in the slot `Vec<Tag>` feeds | `Tag` is derived from branch naming; "holds the current session" is not a naming fact. A bool on `WorktreeNode` keeps core out of a view concern (research.md R5) |
