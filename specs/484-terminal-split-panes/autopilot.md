# Autopilot ledger — #484 terminal-split-panes

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 484 — issue #484 "Split the terminal area into panes to watch several sessions at once" (scope verbatim in the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #484
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-09
- **Phase**: milestone M4
- **Next step**: reviews A and B, gate, push; orchestrator opens PR from scratchpad pr-484-m4.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #658 | M3 | merged | 3ae9822e88b48e0f73d83400881719f6a48a777a |
| #657 | M2 | merged | e8f3abcccfc89f5a31c27474f0ba7ec6442629e9 |
| #656 | M1 | merged | b3372f2016f9326499e632c32f08522736fb5525 |
| #655 | Design (spec, plan, tasks) | merged | 909f4b233cfe660881ab0cbdc62a0ad7488e5221 |
| #654 | Previous run (#483), not this run's | merged | 334cc99ff48ae82e60a4a107f4b76cb1ef9d5959 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020, T006a | full | Live panes with focus and own PTY size (US1+US2+US5 folded: no half-wired UI; diff exceeds split threshold but no scenario split leaves a working deliverable) | #656 | merged |
| M2 | T021–T023 | full | Pane shortcuts (split, focus) | #657 | merged |
| M3 | T024–T028 | full | Resize, close, rearrange | #658 | merged |
| M4 | T029–T035 | full | Layout survives restart | — | in progress |

## Decisions

- M4: the client prunes gone terminals (existing `sync`) and then saves, so a terminal that vanished is dropped from the stored layout at the next change; the file keeps unresolved terminals only until the client next saves. `layout_version` stays separate from `schema_version`. `pane_layout` is held raw (`serde_json::Value`) in `StoredProjectState` so a bad shape fails that field only.
- Clarify round 1: 3 markers agent-resolved (one pane per terminal; empty panes persist; chords Ctrl/Cmd+Shift+D/H/W/Arrow, not Ctrl+Alt+Arrow). No escalation.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 303c12f166730835012ddb99e2d6f068115ff086:e61d1682ef7b666342bfb8834c130312e9e3e26a | CHANGES: 1 MAJOR (bad `feature 182` citation: it was a commit hash; fixed, prose only), 3 MINOR (fixed, prose only) |
| Plan | 1 | dbb88a3c2cb206cb6716902bb89d94ac49cd8803:8f919183081da946845bf3a7e9b909967d5192fe | CHANGES: 2 MAJOR (layout write path: daemon-owned via Workspace + SetPaneLayout; process field semantics), 4 MINOR; fixed |
| Plan | 2 | 2c92523f4feb02154bd127d01e88edcb54c2d30c:ea4f3c2260e6d692e94b4d3122dfc2776902dafb | CLEAN (1 MINOR: snapshot name, fixed, prose only) |
| Tasks | 1 | 0301e32716201fec8325e22c1d5d8d31d669aebd:5a7cd75a2dbce27d266c19362ff9763f1b66a99a | CHANGES: 2 MAJOR (M1 half-wired: US2+US5 folded into M1; last-pane refusal moved to T026), 4 MINOR; fixed; analyze HIGH C1/U1 fixed (T006a, T002) |
| M1 diff A | 1 | d50bde24 | CHANGES: 2 MAJOR (daemon stream not re-attached after PTY restart; pane_sizes unkeyed by project) + 2 MINOR; all fixed |
| M1 diff B | 1 | HEAD | CLEAN (1 MINOR wording, fixed); visual pass: showcase SplitView, label inset fixed |
| M2 diff A+B | 1 | ceb9881f:075de250 | CLEAN (3 MINOR: shadow case in test fixed; no tdd/ dir in this feature; ledger cell fixed) |
| M3 diff A | 1 | 8c89cb11:cef39232 | CLEAN (3 MINOR: stale header press, stuck divider_dragging, divider gone mid-drag; all fixed) |
| M3 diff B | 1 | c25a230e:a59eb500 | CLEAN (Verify not runnable by reviewer, run in the gate; 2 MINOR: no tdd/ dir, T027 names ui/terminal.rs for the close button, which is in ui/panes.rs) |
| M3 visual pass | 1 | real client on Xvfb | pass; fixed header strip height (header took half its pane) and the single-pane refusal text; specs/484-terminal-split-panes/visual-pass-m3.md |
| Tasks | 2 | e3ce4bb44072efb9b7df77aca2989996be758acd:5a7cd75a2dbce27d266c19362ff9763f1b66a99a | CHANGES: 1 MAJOR declined, 2 MINOR fixed prose only; done |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Tasks 2 | F1 MAJOR: T036/T037 in no milestone | Polish changes no code: left to the close unit (milestones.md rule 4) |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Review-A daemon fix (restarted PTY re-streams): done in M4, `a_restarted_terminal_streams_again_to_the_client_viewing_it` (fails with the fix mutated away).
- T018's divider-drag coalescing and drag-end flush has no M1 surface (dividers are not draggable until M3/T027): M1 sends a pane's size when the reporter sees it change, at most once per layout pass; T027 must add the drag-end flush and its test. (M3: done, `divider_dragging` in App + `a_divider_drag_sends_pane_sizes_once_on_release`.)
- Empty-pane picker is a list of buttons (one press per choice) rather than the `picker` component; revisit if review asks.
- M2 ships chords only through the focused terminal widget (no app-level key listener): with the terminal unfocused the chords do nothing; the header buttons remain. Revisit if review asks.
