# Quickstart: Worktree Overview

**Feature**: 813 | **Plan**: [plan.md](./plan.md) | Contracts: [wire](./contracts/wire.md),
[ui](./contracts/ui.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree (in the cloud environment: `scripts/gate.sh`).
- For §B: the `visual-pass` skill's private display, client and daemon built together, a private
  data directory, Claude Code hooks configured for the sessions, and `gh` signed in to a test
  repository with pull requests on the branches below (or the fake reading the client tests use,
  for B4 and B5 only).
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core   # card_state table, location_type, counts, task_progress parsers, hook_activity, PR query/parse
mise run gate        # wire round-trip + schema hash, daemon hooks/state, client state, keymap, geometry, contrast, icons, showcase, docs
```

Expected: both green. Each test in [plan.md](./plan.md#test-strategy-by-layer) exists and fails
without the change it covers.

## §B — Recorded pass on Linux (visual-pass)

Project P with Default and worktrees `feat-42_alpha` (branch `feat/42_alpha`, issue #42 exists,
`specs/042-alpha/tasks.md` with 6 of 10 ticked), `fix-7_beta` (one session asking for a
permission; its PR has failing checks), `chore-9_gamma` (a working session), `docs-3_delta` (merged
PR, removable), `misc` (no session, no convention), an included worktree, and a missing folder.
Screenshot each step that names a look, in light and dark.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Click the Overview button in the top bar. | Grid of 8 cards (one per sidebar location) with type icon + word; order: `fix-7_beta` (red, *Needs action*), then Working, Done, Idle; Idle greyed. Sidebar and top bar stay; crossfade. | US1.1, US1.4, US1.5, US2.2, FR-001–007, FR-023 |
| B2 | Read `feat-42_alpha`'s card. | Summary is issue #42's title; *Implement · 6/10* with a 60 % bar; counts per its branch. | US4.1–4.3, FR-015, FR-016 |
| B3 | Tick a task in its `tasks.md` on disk. | *7/10* within 5 s. | US4.5, FR-008 |
| B4 | Read the `misc` and missing-folder cards. | Branch name or name; *No task progress*, no bar; missing: no counts. | US4.4, FR-017, Edge: no branch |
| B5 | Turn pull request status off in Settings, then reopen the Overview. | Counts read *—* with tooltip *pull request status is off*; `docs-3_delta` no longer Done. | Edge: PR off, FR-004 |
| B6 | Close the Overview; look at the button. | Red badge with icon and count *1*; tooltip *1 needs action*. Grant the permission: the badge turns yellow (failing checks) within 1 s. | US2.4, FR-012, US1.6 |
| B7 | Focus a terminal, press Ctrl+Shift+O twice (check the PTY with `cat -v`). | Overview opens, then closes and the terminal view returns; `cat -v` printed nothing. | US2.3, FR-011, SC-006 |
| B8 | Open the Overview; Tab, arrows, Enter on `chore-9_gamma`. | Focus ring moves card to card; Enter closes the Overview and shows its main session. Escape on a reopened Overview closes it. | US3.1, US3.4, FR-014 |
| B9 | Activate `misc`; then the missing folder's card. | Row revealed and the start menu opens at it, nothing starts; missing: only revealed. | US3.2, US3.3, FR-013 |
| B10 | In `chore-9_gamma`, prompt the agent to write a 5-item todo list and finish 2. | Card shows the in-progress item and *2/5*. A session without hooks keeps B2-style text. | US5.1–5.3, FR-018 |
| B11 | Open Settings, press Ctrl+Shift+O. | Nothing changes; the button is disabled. | R2 |
| B12 | Narrow the window to its minimum; then open a project with 30 worktrees. | One column, ellipsized text with tooltips, no overlap; 30 cards appear within 1 s and scroll, attention cards first. | Edge: narrow, many, SC-007 |
| B13 | Stop the service; then start it. | Connection banner above the Overview, cards keep their session state; on reconnect they update. | Edge: disconnected |
| B14 | Open a second window on P; change a session. | Both windows' cards change together; each opens its own Overview. | Edge: several windows |
| B15 | Showcase: `OverviewCard`, `CardGrid`, `ProgressBar`; greyscale screenshot of B1. | Five states told apart by icon and word in both schemes. | FR-021, SC-005 |
| B16 | From B1 alone, name every location that needs you. | Correct in under 5 s. | SC-001, SC-002 |

## §C — macOS and Windows

Repeat B1, B7 (Cmd+Shift+O on macOS) and B15 on macOS and Windows. Expected: the same results
(Principle VI).
