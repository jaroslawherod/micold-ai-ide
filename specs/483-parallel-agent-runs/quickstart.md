# Quickstart: Run One Prompt Across Several Agents and Pick the Best Result

**Feature**: 483 | **Plan**: [plan.md](./plan.md) | Contracts:
[run-group-wire](./contracts/run-group-wire.md), [integration](./contracts/integration.md),
[parallel-surfaces](./contracts/parallel-surfaces.md) | Data model:
[data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree; `git` 2.38 or newer on `PATH` (research R1).
- For §B: the `visual-pass` skill's private display, pinned binaries and private data directory, and
  two fake AI CLIs that echo their input (the one
  `crates/micold-daemon/tests/mcp_create_session.rs` installs works), so a delivered prompt can be
  read in each run's terminal.
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core     # runs::{naming,group,integrate,summary} + real-git integration tests
mise run gate          # daemon run-group tests, client state tests, geometry gates, fmt, clippy, docs
```

Expected: both green. Each test named in [plan.md](./plan.md#test-strategy-by-layer) exists and was
seen failing before the code that satisfies it (cycle log).

## §B — Recorded pass on Linux (visual-pass)

Fixture project P: a git repository on `main` with a second local branch `spike`, one unrelated
worktree `wt-other`, and a fake AI CLI available under two provider names.

Record a screenshot for every step that names a look, in the light and the dark scheme.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Project menu → Run in parallel. Type a prompt, type `feat`, name `login page`, base `main`, 3 runs with mixed providers. | The dialog of D1–D3: the derived names read `feat/login-page-1..3` with their folders; Start runs enabled. | US1 s1, FR-001–003 |
| B2 | Empty the prompt; then the name; then remove runs down to 1; then add up to 9. | Each shows its own error and Start runs is disabled; the run list stops at 2 and at 8. | US1 s5, FR-002, Edge Empty input / Too many runs |
| B3 | Start runs. | Dialog closes; a group row `login page` with `3 runs` appears; its 3 runs are listed under it, each with its number and provider; `wt-other` stays outside. Each run's terminal shows the prompt once. | US1 s1, s2, US2 s1, FR-004, FR-007, SC-001 |
| B4 | Before starting a second group, create a worktree on branch `feat/clash-1` by hand. Then start a group named `clash` with 3 runs. | Run 1 is Failed with the name-collision message; runs 2 and 3 are created, started and prompted; no half-created folder or branch for run 1. | US1 s3, FR-005, FR-006, SC-002 |
| B5 | Start a group of 2 where one run's provider is made unavailable between confirming and starting. | That run is Failed or Prompt not delivered with the reason; the other is unaffected. | US1 s4, Edge Provider not installed |
| B6 | Collapse the `login page` group row. | Runs hidden; the row still shows `3 runs` and, for the `clash` group, `1 failed`. | US2 s2, FR-007 |
| B7 | Quit and restart. | Both groups, their runs and the run order are as they were; the interrupted-run rule has nothing to report. | US2 s3, FR-008 |
| B8 | Group row → Compare. Make different changes in the three runs (run 2 committed, run 3 uncommitted, run 1 none). | Rows per C1–C2: provider, status, `<files> files`, `+a`, `−r`; run 1 reads `0 files +0 −0`; run 3 carries the uncommitted tag. Each row's counts equal that run's Changes view totals with both toggles on. | US3 s1, s2, FR-009, FR-010, SC-004 |
| B9 | With Compare open, edit a file in run 2 from a terminal. | That row's counts change within 2 s without reopening Compare. | US3 s3, FR-010 |
| B10 | Press Open diff on run 2. | Run 2's Changes view opens. | US3 s4, FR-011 |
| B11 | Look at the failed run of the `clash` group in its Compare. | Its reason in place of the counts; no Open diff. | US3 s5, FR-009 |
| B12 | While a group's runs are still being created, open its Compare. | Pick this one is disabled on every row with the reason as its tooltip. | US4 s7, FR-017 |
| B13 | Pick run 3 (uncommitted changes). | Refused, naming the uncommitted files; nothing changes. Commit them in run 3's terminal, pick again: it succeeds. | US4 s9, FR-012, Edge Winner with uncommitted changes |
| B14 | Make `main` and run 2 change the same lines; pick run 2. | Refused, naming the conflicting files; `main`, every run's branch and every worktree are unchanged. | US4 s2, FR-013, SC-006 |
| B15 | With `main` checked out in the project root holding an uncommitted edit the merge would overwrite, pick a run that touches that file. | Refused with git's reason; nothing changes. | Edge Base branch checked out, FR-013 |
| B16 | Pick run 2 with `main` unmoved; then with `main` holding one unrelated commit. | First: `main` fast-forwarded. Second (new group): a merge commit on `main` naming the group and run; neither run branch moved. | US4 s1, FR-012, Edge Base branch moved |
| B17 | In the cleanup offer after B16. | K1–K3: a row per loser with what removal deletes and its branch checkbox; the loser with uncommitted changes carries its tag and is unselected. | US4 s3, s4, FR-014, FR-015 |
| B18 | Select the uncommitted loser as well and confirm. | A second confirmation naming it and its uncommitted changes; cancelling removes the others only; its worktree and branch survive. | US4 s4, FR-015, SC-005 |
| B19 | Pick in a second group and dismiss the cleanup offer. | Nothing removed; every loser stays in the group; Compare shows the winner and offers no further pick. | US4 s5, Edge Picked group |
| B20 | Open a second window on P. Pick a run in each window at the same time. | Exactly one run is integrated; the other window says the group already has a winner and changes nothing. | US4 s6, FR-018 |
| B21 | Delete one run's worktree from its own row; then dismiss a group. | The run leaves the group and the group keeps the rest; a group with no runs left disappears. Dismissing leaves the worktrees, branches and sessions as ordinary rows. | US2 s4, s6, FR-007, FR-020 |
| B22 | Kill the app while a group's runs are being created, then start it again. | The unfinished runs read Failed ("interrupted"); the finished ones are intact; no half-created branch or folder of the unfinished ones remains. | US2 s5, FR-019 |
| B23 | Showcase (`mise run showcase`): the Run in parallel dialog, the group row and the Compare rows, both schemes. | Every pose readable; the counts, tags and statuses legible in both schemes. | Principle VIII |

## §C — Other platforms

On macOS and Windows: B1, B3, B8, B9 (refresh within 2 s via FSEvents / ReadDirectoryChangesW), B13,
B16 and B22. B16's merge commit message and the derived branch and folder names are compared with
Linux's byte for byte (Principle VI, Edge Cross-platform; the core naming and integration tests pin
them).

## Results

### §B — Linux (recorded per milestone)

(filled by the implementing milestones)

### §C — macOS and Windows

(filled when run, or recorded as not run with what CI covers instead)
