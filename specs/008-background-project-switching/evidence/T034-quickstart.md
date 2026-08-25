# 008 T034 — the quickstart, run for the first time

**Date**: 2026-08-21
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, Mesa lavapipe (software
Vulkan), driven with `xdotool`, captured with `import`. Per the repo's `visual-pass` skill.
**Build**: this branch's own `micold-ai-ide` + `micold-daemon`, built in one invocation and copied
out of the shared target directory **inside** the build lock (`~/vp83/bin`, 2026-08-20 21:03). The
newest commit touching `crates/` is `d28a0c6` (2026-08-19), so the pinned pair is this branch.
**Isolation**: `XDG_RUNTIME_DIR=/tmp/vp83`, a scratch `XDG_DATA_HOME`. Everything started here was
stopped by PID afterwards.

## Fixture

Two throwaway git repos at **real** paths — `/home/jaro/.aaa-vp83d/myrepo` (Project A) and
`.../repo-b` (Project B); not symlinked, because a symlinked project path misclassifies every
worktree ([002 BUG-002](../../002-project-workspace-management/bugs/BUG-002.md)). A carries one
worktree, `Alpha`. Sessions are real `claude` processes; the session under test runs a shell loop
printing `tick N at HH:MM:SS` every two seconds, so "the output it produced while you were away" is
something a frame can settle rather than something to take on trust.

## Automated half — **PASS**

`mise run test` (`cargo test --workspace`) over the whole suite: 201 test binaries, **0 failures**.
Every assertion the quickstart names by behaviour is present and green:

| Quickstart claim | Test |
|---|---|
| `switch_active` keeps outgoing sessions `Running` (BS-1) | `switch_keeps_outgoing_sessions_running`, `switching_among_projects_never_stops_a_session` |
| restores the stored foreground, else first running, else `None` (BS-3, FR-003a) | `foreground_restored_on_return`, `switching_away_and_back_returns_to_the_session_that_was_in_front`, `entering_a_project_with_no_recorded_foreground_falls_back_to_a_running_session`, `records_outgoing_foreground_before_activating` |
| `find_session`/`find_session_mut` resolve in a **non-active** project (BS-6) | `find_session_resolves_in_non_active_project`, `find_session_mut_allows_lifecycle_mutation`, `find_session_matches_on_session_location_not_a_bare_string`, `find_session_none_for_unknown_id` |
| background restart marks `restarted_while_inactive`; switching in sets the notice (BS-7) | `marks_restart_only_when_owning_project_is_inactive`, `a_restart_in_an_inactive_project_is_remembered_until_the_user_returns`, `a_restart_in_the_active_project_raises_no_return_notice`, `shell::daemon_sync::tests::reconcile_detects_a_background_restart_and_arms_the_return_notice` |
| switching to an unavailable project returns `false` and changes nothing (BS-10) | `switch_to_unavailable_is_rejected_and_leaves_state_unchanged`, `reopening_unavailable_project_is_rejected_and_leaves_active_unchanged` |
| `running_session_count(path)` matches per project (FR-007) | `running_session_count_counts_active_only` |
| switcher rendering | `switcher_entries_reflect_active_running_and_availability`, `a_switcher_row_reports_availability_and_activity_separately`, `toggling_switcher_opens_and_closes_it`, `opening_switcher_closes_the_overflow_menu`, `opening_the_overflow_menu_closes_the_switcher` |

The first attempt at the workspace suite died in the linker — `collect2: fatal error: ld terminated
with signal 7 [Bus error]` while linking the `micold-ai-ide` test binary, with 53 GB free on the
disk. It did not reproduce; the rerun above is clean. Recorded because it looked like a test
failure and was not one.

## Manual walkthrough

| # | Step | Result |
|---|------|--------|
| 1 | Start a session in A; confirm live output | **PASS** — the tick loop advancing in the terminal |
| 2 | Switch to B while A runs; A's session is not killed (BS-1, FR-004/005) | **PASS** — B active, A's `claude` still in `pgrep` |
| 3 | In the switcher, A shows a running count; B shows none (FR-007) | **PASS** — `1 running` on A's row, nothing on B's |
| 4 | Return to A: same session, still running, with the output made while away (BS-2, BS-3, SC-003) | **PASS** — `step4-unbroken-tick-log.png` |
| 5 | Kill A's `claude` externally while backgrounded → auto-restart, notice on return (BS-6, BS-7, SC-007) | **PASS** — `step5-restart-notice.png` |
| 6 | Move B's folder, open the switcher: B badged unavailable and unselectable (BS-10, FR-008) | **PARTIAL — half fails.** Not selectable, never silently activated; **no badge until the row is pressed.** [BUG-003](../bugs/BUG-003.md) |
| 7 | The body "Known projects" list and the folder-browser modal still work | **PASS** |

### Step 4 — what the frame settles

`step4-unbroken-tick-log.png` is the frame after returning to A. The visible log runs `tick 2 at
11:27:29` … `tick 69 at 11:29:43` with **no gap and no reset**, spanning the whole period A spent in
the background. Three claims at once:

- the session was never restarted — a restart would have begun again at `tick 1`;
- nothing was stopped as a side effect of either switch (SC-001, SC-006);
- the output made while inactive is fully present on return (SC-003).

The switcher, open in the same frame, reads `1 running` beside A and nothing beside B (FR-007,
SC-004: running projects are identifiable without opening them).

### Step 5 — the restart, with both PIDs

A backgrounded, its `claude` killed from a terminal. The daemon's poll loop restarted it inside
~13 s — pid `461296` gone, pid `463712` in its place under the same `XDG_RUNTIME_DIR`. Switching
back to A raised the snackbar in `step5-restart-notice.png`:

> A background session was restarted while you were away.   *Dismiss*

Which is SC-007 exactly: surfaced on return, not silently.

### Step 6 — the half that fails

Written up in full as [BUG-003](../bugs/BUG-003.md). In short: `Project::availability` is recomputed
in only two places — startup, and `on_known_project_reopened` — so opening the switcher shows
whatever the last of those left behind. Moving B's folder aside changes nothing on screen; pressing
the row is what runs the scan, and *then* the row dims and takes its badge
(`e25-unavailable-badge.png`, before and after the press). Moving the folder back does not recover:
an unavailable row carries no message, so there is no press left to run the scan, and the flag is
latched until relaunch.

The half that holds is the one that matters most: **nothing was silently activated**. The scan sits
above `switch_active` in the same handler, so the press re-scans, `switch_active` refuses, and A's
chip, title, active marker and running count were all untouched (`2 running` by that point — a
second session had been started in A during step 5's restart work). And A's background sessions were
unaffected throughout, which is the rest of the step's claim.

### Step 7 — the complement check

After a relaunch, the shell body lists both projects under **Known projects**, each with its git
chip and enabled **Open** / **Rename** / **Forget**; the active one carries its marker. Picking
"Add project…" from the switcher opens the folder browser — "Open a project", an **Up** control, the
`/home/jaro` breadcrumb, a directory list annotating which entries are git repositories, and
**Open this folder** / **Cancel**. Both routes intact (2026-07-17 clarification, FR-009).

## One defect found outside the numbered steps

Selecting a project switches correctly and **leaves the switcher panel open**, against the
contract's explicit "Panel closes." Filed as [BUG-002](../bugs/BUG-002.md) with
`e26-switcher-stays-open.png`. It also spends the interaction SC-002 budgets: open and select are
two, and a third press is then needed to clear the panel off the view.

## Success signals

| | |
|---|---|
| SC-001 / SC-006 — no session stopped as a side effect of any switch | **PASS** (step 4's unbroken log; `pgrep` across every switch) |
| SC-002 — switching is ≤ 2 interactions | **PASS as designed, undercut in practice** — open and select do switch, but the panel then needs dismissing (BUG-002) |
| SC-003 — output made while inactive is present on return | **PASS** |
| SC-004 — running projects identifiable from the switcher without opening them | **PASS** |
| SC-005 — the newly selected project displays within ~1 s | **not measured.** `import` costs ~300 ms on its own and lavapipe is a software rasteriser; a number from this pipeline would not mean anything about a user's machine. Every switch here appeared complete in the frame taken ~1.5 s later, which is consistent with the claim without evidencing it. |
| SC-007 — a background restart/failure is surfaced on return, never silently | **PASS** |

## Harness notes

- Frames captured immediately after a click can predate the render — `import` itself takes ~300 ms —
  which made one switch look like a no-op until a second capture 1.5 s later showed it had happened.
  Every result above rests on a settled frame.
- The window on `:83` answers `xdotool search` before it has drawn anything; a black frame is that,
  not a launch failure. Re-capture after a few seconds.
