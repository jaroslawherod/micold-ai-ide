# Implementation Plan: Hide Agent Worktrees

**Branch**: `fix/hide-agent-worktrees` | **Date**: 2026-07-23 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/014-hide-agent-worktrees/spec.md`

**Revision**: re-run after the `/speckit-clarify` session of 2026-07-23 (4 answers). See
[Clarifications incorporated](#clarifications-incorporated) for what moved.

## Summary

An AI assistant working in the same repository creates throwaway worktrees under the project's own
`.claude/worktrees/` directory, so they land in the sidebar next to the user's worktrees and are
indistinguishable from them. This feature classifies each discovered worktree as user-owned or
agent-owned purely from its naming, hides the agent-owned ones from the sidebar and everything
derived from it, and adds a "Show agent worktrees" chip to the existing filter accordion that
brings them back — badged, and fully actionable — for the current project in the current run.

Technical approach: classification is a pure predicate on `Worktree` (`dir_name` + `branch`) in the
render-free core, so `Vec<Worktree>` keeps carrying every discovered worktree and nothing about
discovery, git, or the filesystem changes. A single new `State::visible_worktrees()` accessor,
gated on a transient `show_agent_worktrees: bool`, becomes the source for `worktree_tree()`,
`available_tag_filters()`, and the sidebar's empty-state hint — so hiding, counting, filtering, and
action targets stay consistent by construction rather than by three separate filters. Two UI
touches: a new `Tag::Agent` badge on revealed rows, and a reveal chip in the filter accordion built
on a `ToggleChip` primitive promoted out of the sidebar's existing bespoke `filter_chip()`.

## Clarifications incorporated

The four answers from the clarify session, and what each one changed in this plan:

| Clarification | Effect on the design |
|---|---|
| **FR-013** — a revealed row keeps full actions (start session, rename, delete) | `row_actions_cluster()` stays unchanged: no branch, no disabled state, no extra confirmation. Was previously only a spec assumption; now a requirement with its own quickstart scenario |
| **FR-010e** — the reveal control resets on every project switch | `restore_after_activation()` (`src/app.rs:1369`) gains `show_agent_worktrees = false`, beside the existing `default_expanded = false`. This is the one place the control deliberately diverges from `sidebar_filters`, which is sticky across switches |
| **FR-005/FR-006** — identifier is ≥ 16 characters, all hexadecimal | Was already the plan's working assumption (research R2); now normative, so the 16/15 boundary rows in the classification truth table are required tests rather than defensive ones |
| **Terminology** — user-visible copy says "agent", spec prose says "assistant-owned" | Pins the UI strings (`"Show agent worktrees"`, badge `agent`) and the FR-012 docs section. No user-facing string may say "assistant" |

Only FR-010e changed the design; the other three confirmed or tightened decisions Phase 1 had
already made.

## Technical Context

**Language/Version**: Rust, stable toolchain (managed by `mise`), edition 2021

**Primary Dependencies**: `iced` 0.13 (GUI-only, `gui` feature). No new dependency — classification
is `str` prefix + `is_ascii_hexdigit` checks in `std`

**Storage**: N/A — no new persisted state. The reveal control is transient *and* project-scoped
(FR-010a, FR-010e): reset on app start and on every project switch, never written to the store.
Ownership is recomputed from names on every call

**Testing**: `mise run test` (`cargo test --no-default-features --all-targets`) — the render-free
core. New unit tests in `tests/worktree_owner.rs`; extensions to `tests/sidebar_tree.rs` and
`tests/app_state.rs`. GUI wiring is validated by `quickstart.md` (Principle I GUI-wiring exception)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: Desktop application — single Rust crate with a render-free `lib` core plus a
`gui`-feature binary

**Performance Goals**: Classification is O(worktrees) with a short constant per entry; no
user-perceptible change to project-open or sidebar-render time (SC-004)

**Constraints**: Presentation-only — no git command, no filesystem write, no branch or directory
mutation may be added by this feature (FR-008, SC-005). FR-013's delete path is the *existing*
lifecycle code, reached by explicit user action, not new behavior introduced here

**Scale/Scope**: Tens of worktrees per project. ~5 core functions touched, 1 new shared UI
primitive, 1 new `Tag` variant, 1 new `Message`, 1 new transient state field

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every unit of new logic — `is_agent_owned`,
      `visible_worktrees`, the tree/filter/empty-state consequences, the reveal toggle reducer, and
      the FR-010e project-switch reset — is pure core code reachable from `tests/`, and each lands
      Red-first. The only code relying on the GUI-wiring exception is the sidebar render of the
      reveal chip and the agent badge (`src/ui/sidebar.rs`, `src/ui/material/toggle_chip.rs`),
      which carry no decision logic of their own and are validated by `quickstart.md`.
- [x] **II. Multi-Session Support**: PASS. No session state is added, removed, or re-scoped.
      FR-011 explicitly reuses the app's existing behavior for a session whose worktree is not in
      the visible list — sessions are joined to worktrees in `worktree_tree()` and are neither
      pruned nor terminated by this feature. FR-013 keeps session-start available on a revealed
      row, which is the ordinary path and adds no new session semantics.
- [x] **III. Worktree Integration**: PASS, and worth stating precisely now that FR-013 is
      normative. *Hiding* is presentation-only: it adds no git call and no lifecycle step, and the
      app never adopts, prunes, or claims ownership of an agent worktree (FR-008). *Deleting* a
      revealed agent worktree is a different thing — a user-initiated action running the app's
      existing, already-worktree-aware lifecycle path, which is exactly what Principle III asks the
      app to own. FR-008 is not weakened: it forbids modification *as a consequence of hiding*, not
      modification the user explicitly requests. The `Default` project-root entry is unaffected; it
      is already exempt from tag filtering and stays exempt from the reveal control.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. No new persistence, no network. The
      reveal control lives in memory, scoped to the active project for the current run.
- [x] **V. Rust + iced Stack**: PASS. Ownership is modeled as an `enum WorktreeOwner { User, Agent }`
      rather than a bool, so a third classification is representable without a boolean-blindness
      refactor, and `Tag::Agent` makes "this row is agent-owned" a value the renderer matches on
      exhaustively rather than a string convention.
- [x] **VI. Cross-Platform Parity**: PASS. Name-based classification is byte-identical on all three
      platforms; nothing branches on the host OS. Paths are only compared by their final component,
      which `reconcile()` already normalizes.
- [x] **VII. Documentation First-Class**: PASS. FR-012 is delivered in the same change —
      `docs/user-guide/worktrees-and-sessions.md` gains an "Agent worktrees" section covering their
      existence, the default-hidden behavior, how to reveal them, and that the app never cleans
      them up. Per the spec's Terminology section it says "agent", not "assistant".
- [x] **VIII. Reusable UI Component Foundation**: PASS. No bespoke widget is forked. The agent
      badge reuses the existing shared `Tag` chip primitive
      (`src/ui/material/tag.rs`). The reveal chip does **not** get a private copy of the sidebar's
      local `filter_chip()`; instead that bespoke helper is promoted into a shared
      `ToggleChip` builder (`ToggleChip::new(label, on_press, roles).active(b).accent(fill, on)`,
      terminating in `.into()`), which `filter_chip()` then delegates to — one primitive, two call
      sites, builder API as mandated.

**Post-design re-check (after Phase 1, re-confirmed post-clarify)**: still PASS. Two notes.
Principle VIII got *stronger* during Phase 1: the reveal chip needs the sidebar's private
`filter_chip()` look, so the design promotes it to a shared `ToggleChip` instead of copying it
(contracts/sidebar-reveal-control.md), which also removes an existing private-widget wart.
Principle III is the one the clarify session made sharper rather than riskier — FR-013 was already
the spec's assumption, and making it a requirement forced the hiding-vs-deleting distinction above
to be written down instead of left implicit. Principle I is unchanged in scope: the GUI-wiring
exception covers only the accordion placement and the badge render, both recorded as manual steps
in `quickstart.md`, while the placement *rule* (unconditional, above `filter_bar()`'s early return)
is a contract invariant a reviewer can check without running the app. No violations, no Complexity
Tracking entries.

## Project Structure

### Documentation (this feature)

```text
specs/014-hide-agent-worktrees/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── agent-worktree-classification.md
│   └── sidebar-reveal-control.md
├── checklists/
│   └── requirements.md  # /speckit-specify output
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── worktree.rs          # + WorktreeOwner enum, Worktree::owner()/is_agent_owned() (pure core)
├── naming.rs            # + Tag::Agent variant (badge only — never a TagFilter)
├── app.rs               # + show_agent_worktrees flag, ShowAgentWorktreesToggled message,
│                        #   visible_worktrees(); worktree_tree()/available_tag_filters() rebased;
│                        #   restore_after_activation() resets the flag on project switch (FR-010e)
├── main.rs              # unchanged — discovery keeps returning every worktree
└── ui/
    ├── sidebar.rs       # reveal chip in the filter accordion; empty-state hint rebased;
    │                    #   filter_chip() delegates to the shared ToggleChip;
    │                    #   row_actions_cluster() UNCHANGED (FR-013)
    └── material/
        ├── mod.rs       # + toggle_chip module export
        ├── toggle_chip.rs  # NEW shared primitive (promoted from sidebar::filter_chip)
        └── tag.rs       # unchanged — reused as-is for the agent badge

tests/
├── worktree_owner.rs    # NEW — classification truth table incl. the 16/15 boundary and case
│                        #   rules (US1, US2, FR-005, FR-006)
├── sidebar_tree.rs      # + hidden/revealed tree, empty-state, orphan/missing, tag-filter
│                        #   interaction, Tag::Agent on revealed rows (US1, US3, US4)
└── app_state.rs         # + reveal-toggle reducer, default-off, filters-untouched,
│                        #   project-switch reset (FR-010a, FR-010d, FR-010e)

docs/user-guide/
└── worktrees-and-sessions.md  # + "Agent worktrees" section (FR-012)
```

**Structure Decision**: The existing single-crate layout is kept unchanged — the render-free
`lib` core (`src/worktree.rs`, `src/naming.rs`, `src/app.rs`) holds every decision this feature
makes, and the `gui`-only binary (`src/ui/`) only renders what the core already decided. That split
is what lets the whole feature, apart from two render calls, be covered by
`cargo test --no-default-features`.

## Complexity Tracking

> No Constitution Check violations. Section intentionally left empty.
