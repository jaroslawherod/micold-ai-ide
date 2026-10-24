# Autopilot ledger — #1126 session-context-header

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1126 "Layout: worktree context header and session tabs above the terminal" (app redesign issues, started by dependency order); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1126
- **Worktree branch**: claude/project-thread-y28cqn
- **Started**: 2026-10-24
- **Phase**: close
- **Next step**: close unit: speckit-tdd-verify, Polish T056–T057, review, gate, close PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1318 | design (spec, plan, tasks) | merged | f8eeaa43e5f151a08a2c20b055c5c9ce64bf9584 |
| #1328 | M1 context header | merged | 127a817868d7b463ad78ea588b43a0339d483991 |
| #1334 | M2 tab strip | merged | 25de2370813389d6ae82fcfb49ebce6cf15da226 |
| #1337 | M3 Changes button | merged (CI red twice on the layout snapshot as main moved under it; rebased and re-recorded each time) | 9c1f758fc9a573960f71729679e6cabee25d2a36 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | context header above the terminal (title, state, branch, PR chips); bar unchanged | #1328 | merged |
| M2 | T021–T042 | full | tab strip replaces the bar; chords and focus unchanged (US2 + US4) | #1334 | merged |
| M3 | T043–T055 | full | Changes button with counts opens/closes the Changes view | #1337 | merged |

## Decisions

- Plan (2026-10-24): header facts from the Overview card's own `CardModel` (R2); counts from the Changes view's list reader in the background for the shown location (R5, R6); strip = session tabs + shown session's shells and "+", status/restart, New session, splits at the right; keyed row for presses (R7–R10); header stays above the open Changes view, strip does not (R12). Edits confined to `ui/mod.rs`'s active-session/Changes branch for #1124/#1125 rebases.
- Plan review 1: tab right-click menu stays on the shown session's tab and its shell tabs; another session's tab opens no menu (`TerminalRestartRequested` acts on the shown session). `StripTab` + predicates re-exported from `ui/terminal.rs`; `features/session.rs` imports from `features::session_strip`.
- Clarify round 1 (strip tab contents, US2 sc. 3): option A, session tabs (main, then helpers) with the shown session's tab as its AI conversation, its shell tabs and "+" after it; New session runs **Start another session...** with its confirmation. Orchestrator's provisional pick of the recommended option, user decision pending (decision card). Applied to spec.md Terms, US2 sc. 3/7, FR-008 to FR-010, Key Entities (Tab), SC-004.
- Tasks (2026-10-24): M2 keeps US2 whole with US4 (22 tasks, over the ~10 split guide): no acceptance-scenario split leaves main without a bar control (FR-009: bar removal and strip land together); the behaviour-preserving moves (StripTab and predicates into `features::session_strip`, `cdk::Keyed`) go in Foundational (M1) to shrink M2. Polish T056–T057 is code-free, left to the close unit.
- Clarify round 2: session tab order and press behaviour agent-resolved from worktrees-and-sessions.md; no new requirement, clarify ends.
- Rebase onto origin/main afef53af (#1124 M1 status bar, #1299, #1260) for PR #1328's merge conflicts: no behaviour choice; four gates restated for both chromes together (header above, status bar below the pane), fixture regenerated. Details in tdd/cycle-log.md.

- M3 (2026-10-24): the Changes button sits on the header's title line (`column![row![title, button], chips]`), not beside both lines as H9 first said: at 640×480 beside a 300 px sidebar the button beside the column left the chips 0 px (`session_area_fits` red). Contract H9 and R11 updated; mockup look kept at wide windows (button at the header's right).
- M3: the counts read is drained by `shell::changes::run_followed` after every message's follow step (main.rs `update`); `Msg::Changed` returns the view's `ReadList` and the header's `ReadCounts` batched; one watcher per location (`State::header_watch_entry` skips the view's own entry).

- M3 review B F2: the Changes label is one `Ellipsized` line with the full label in its tooltip, and contract H10 states its bound: whole at 640×480 for counts up to three digits, cut with an ellipsis beyond (FR-013's rule for header text), whole at 1280×800 for any realistic count (gate `the_widest_counts_fit_at_the_layout_window`). Spec unchanged.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- M3 red CI (2026-10-24, PR #1337): `build + test (ubuntu-latest)` failed `layout_snapshot::the_layout_matches_the_committed_fixture` in `status-bar-warnings` (header 12 px taller). CI tests the merge with main, which had gained #1124 M3 (7b6639cb, re-recorded 706d1d52) with the new states `status-bar-disconnected` and `status-bar-warnings`, recorded without M3's Changes button; git merged the fixture text cleanly but stale. Rebased onto origin/main (no conflicts) and re-recorded. Diff vs main: only the states M3 already changed (session-area-*, session-transcript-*, status-bar-*) plus #1124's two new states, each the header's title row gaining the Changes button (+12 px, rows below shifted); #1124's own changes (error-daemon-disconnected, settings-view-rail-collapsed, status-bar-settings, status-bar-warnings-overview) are byte-identical to main. Not a layout regression.
- M3 red CI 2 (2026-10-24, PR #1337 at bd4d521e): same test, first in `session-area-one-tab` (status-bar items at y 778 missing from the fixture). Main had gained #1124 M4 + M5 + close (CLI item follows the focused pane, Overview/Jump key hints, plan usage in the status bar, new state `status-bar-usage`, Settings wording) and #1260 close. Rebased onto origin/main c79dd7f0 (no conflicts), re-recorded. Check: for every state main→new differs in, the diff is byte-identical to the M3 diff before this rebase (12d38521→bd4d521e), except `status-bar-usage` (new on main; its diff equals `status-bar-steady`'s header change) and M3's own `session-area-changes-open` (gains only main's two key-hint items at y 778, matching main's `status-bar-changes`: no CLI item while the Changes view has focus). Every other state is byte-identical to main. Not a layout regression.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 0c554b9b6b3c47d8f964a5440a77a10f44444bf6:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 4 MAJOR, 3 MINOR; all fixed |
| Spec | 2 | d6925d167b7f9c841828824458dcbad6a7efa800:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (sonnet); 1 MINOR fixed, prose only |
| Plan | 1 | 4d4fb75608f438dc27c9078b4aa4db62f62e68a1:938d94c5b529d63aa2ac171bcb132cfb2383899a | CHANGES: 3 MAJOR (tab menu not mapped, StripTab importers, containment gate), 3 MINOR; all fixed |
| Plan | 2 | 29583b02c933d279616339822384ea215db890db:3616cce290dbc0e95358c640c3b0c58fabf0feaa | CLEAN (sonnet); 1 MINOR fixed (containment retarget detail) |
| Tasks | 1 | 8ab80bc5203f8fcd8be0c73b112b09dc4539067a:2fa1a219553a964ec33e235d36e1d193a87f7128 | CLEAN; 3 MINOR: checklist ticks + note fixed, provisional clarification noted in follow-ups |
| M1 A (code review, high) | 1 | 77849f887d81f709896e3b79ed506ee1f7c5e311:7a7afc6b185623b1c507387ea64f1a38484a286e | CLEAN; 2 MINOR: F1 fixed (title read once titles become readable), F2 declined |
| M1 B (conformance, sonnet) | 1 | fd556e46e1cb2aa0abc417f86102606b87d79e16:157c47ec92b0f0734825178f76445f2a041b9540 | CLEAN; 3 MINOR (cycle-log evidence for T004/T016, T003's extra moved helpers) recorded in the cycle log |
| M2 A (code review, high) | 1 | ef68e24018cb80883fc8e3cd890be0482b2b0da6:a00127d3e3e8f40571f7c94f2a70aac16f049c7c | CHANGES: 2 MAJOR (new session tab not revealed; nothing-under-panes check could not fail), 1 MINOR; all fixed in 0e326613 |
| M2 A (re-review, sonnet) | 2 | dd505d9a035d24e925094c0ceb5780578a4b3107:0e326613517ad725c43eaba23d8c3341db1b4ebe | CLEAN; F1–F3 resolved, lines cited |
| M2 B (conformance, sonnet) | 1 | dd505d9a035d24e925094c0ceb5780578a4b3107:0e326613517ad725c43eaba23d8c3341db1b4ebe | CLEAN; 1 MINOR (T039 guard written after T033, already noted in the cycle log) |
| M3 A (code review, high) | 1 | 971ff20c3c357d30d2f9710c6cbe9b1481b50dea:d2f82b178335efee2987c5707c1e18199b1d1463 | CHANGES: 1 MAJOR (counts stale after the Overview hid the header), 3 MINOR; F1–F3 fixed, F4 declined |
| M3 B (conformance, sonnet) | 1 | ad49afbb772c22a42e02e75f8f1cb4a1b944597a:52632d457a59d561d87d7f30750d5367df16e913 | CHANGES: 1 BLOCKER (Verify file unreadable: prompt path lost its scratchpad prefix, not a code finding), 1 MAJOR (Changes label wraps at 640×480 with four-digit counts), 2 MINOR (ledger next step; late reds); F2–F4 fixed |
| M3 A (re-review, sonnet) | 2 | 971ff20c3c357d30d2f9710c6cbe9b1481b50dea:d2f82b178335efee2987c5707c1e18199b1d1463 → 5a9d52fa | CLEAN; F1–F3 fixed and F4's decline confirmed, lines cited (first r2 answer cited none and was re-asked) |
| M3 B (conformance, sonnet) | 2 | 7fd22c02a06c5b38313d3be8153870c14ebd905d:5a9d52fa68a1ff52373e5d0dc73e3717902a12fe | CLEAN; 1 MINOR (Verify output did not name the new gate) fixed in the Verify file |
| Close review (tests + docs) | 1 | 3c2eade41bf2761279d2d9d2f1132b5bc05e73b3:1cd58a22f0613ffcd203d6af4ef995a3fa7e09c6 | running |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| tasks | speckit-analyze | C1 HIGH: `gates/session_bar_fits.rs` not retargeted though the bar is removed | that gate holds feature 944's transcript session bar (`session-transcript-insights`), not the bottom bar under the terminal (`layout_snapshot.rs:222`) |
- M3 A F4 (derive the header's counts from the view's list when both are on one location): R5 reads both on purpose; the view's list carries the user's toggles, so a derived figure would change with a toggle (R5's rejected alternative). Two list reads per debounced burst, on the view's open location only.
- M1 A F2 (`pr_chip_input` lacks a Default exclusion): the Default entry's `CardModel` has no branch (`ui/overview.rs::card`, R2), so `pr_chip_input` gets `None` and returns `None`; the header cannot show a pull request the sidebar does not.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M3 red CI 2 gate: `micold-daemon` `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` failed the first gate run and 2 of 5 reruns (notice reached no window). Main's code: the branch changes nothing the daemon uses. A race in the test or in C14's focus fallback; the second full gate was green. Not fixed here.
- M3 gate: `micold-daemon` `mcp_audit_log::every_successful_mutating_call_writes_one_info_line` failed once (paste refused: "no bracketed paste") and passed 3/3 on rerun; untouched by this feature, a flake to watch in CI.

- M3: at 640×480 beside a 300 px sidebar the Changes button leaves the title little room (`fe…` with `Changes · 3 files · +42 −7`, visual pass B6); FR-013 allows it. A compact label for the narrowest window would give the title room back; not specified.

- Clarification 1 (strip contents, New session) is still the orchestrator's provisional pick; confirm with the user before M2 starts. If it changes, T021–T023, T032, T033, T037 change with it.
