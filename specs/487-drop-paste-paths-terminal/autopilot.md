# Autopilot ledger — #487 drop-paste-paths-terminal

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #487 "Drop files or paste images into a session terminal to insert their paths" (proposal and acceptance criteria per the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #487
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-09
- **Phase**: close
- **Next step**: final review of the close diff, gate, push; orchestrator opens the close PR from scratchpad pr-487-close.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #661 | Previous run (#484) | merged | a42cf25517a42ad7a17a25a84db513fea48a4565 |
| #662 | Design (spec, plan, tasks) | merged | ab2f9967b89658219500097833054c14634698c3 |
| #663 | M1 drop files | merged | b1d3c18944e1172294740bffd45295af60570322 |
| #667 | M2 paste image | merged | 3d65796a1f523c657562ab8b6ea91cee2f40ebf4 |
| #671 | M3 sandbox container paths | merged (rebase) | 567676fc070508f04ed368aa11f278b2ae2e5394 (two Windows test fixes added on top by the orchestrator: host-equals-container test now #[cfg(unix)] with a mapped cross-platform twin) |
| #675 | M4 cleanup | merged (rebase) | 474f5937d83f3afbb72677325991d63571479585 |
| TBD | Close (TDD remediation T035–T042, spec closed) | in progress | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | drop files insert quoted paths | #663 | merged |
| M2 | T018–T024 | full | paste image into AI session | #667 | merged |
| M3 | T025–T029 | full | sandbox container paths, refusals | #671 | merged |
| M4 | T030–T034 | full | cleanup on delete and startup | #675 | merged |

## Decisions

- M1: the planned `CursorMoved` subscription is banned by `tests/idle_subscriptions.rs`; the pointer lives in `SplitView`'s widget state and the widget publishes `PaneMsg::FileDropped(pane, path)`; `shell::drops` coalesces with a 40 ms settle (research.md, M1 spike findings). No pane rectangles are published to app state, so T015 became `PaneLayout::pane_at` + the widget hook.
- M2: visual pass skipped: no visible element changed (paste inserts text; errors use the existing snackbar).
- M2: `arboard` (image-data, wayland-data-control) and `png` added; `paste_source` decides, the reducer reuses `files_dropped` for the saved path; the worktree fallback test uses a file-as-worktree (works as root).
- M1: visual pass skipped: M1 adds no visible element (errors use the existing snackbar) and a file drag cannot be synthesised on Xvfb.
- M4: visual pass skipped: no visible element changed (daemon-side cleanup and one docs paragraph). Daemon `set_pasted_data_dir` is set from `ProjectDirs::data_dir()` (same as the client's host data dir); sweep skipped when the catalog did not load; non-UUID names under a pasted root are left alone.
- M3: visual pass skipped: no visible element changed (refusals use the existing snackbar). `SandboxShare` (mounts + really-mounted projects) rides on `FilesDropped`/`ImagePasted`; `shell::sandbox::share` builds it without host probes and fails closed (a notice) under the sandbox placement with no running container.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 8577d9164166df52b5472b80d7aab8e487ee78ee:a42cf25517a42ad7a17a25a84db513fea48a4565 | CHANGES: 3 MAJOR (fixed) |
| Spec | 2 | (fix diff, sonnet) | CLEAN (1 MINOR) |
| Plan | 1 | cf9fa8c2fa31216b4d33a2826532e036bfc60e5a:9a2546f998dc3f8911b8404479a2a297555d6919 | CHANGES: 1 MAJOR (fixed) |
| Plan | 2 | scoped fix diff | CLEAN |
| A M1 (code-review high) | 1 | 8f8d224863015e668a987938c539ac181beb136f:16e690ab28a89a13168bb4d176596569bd609fb8 | CHANGES: 1 MAJOR (fixed: refuse any control character when unbracketed), 2 MINOR (CursorLeft clears pointer fixed; shell-detect limit commented) |
| A M1 (re-review of fix, sonnet) | 2 | 82d149fb7a8a2448c09cc2012611ff61bb357c5b:1257a0b81f0899a9b88f23a7c934a196f2e7a2a5 | CLEAN (1 MINOR, fmt-checked) |
| B M1 (conformance, fresh) | 1 | same | CLEAN (4 MINOR: doc order fixed; shell limit noted; no cycle-log; no widget-level FileDropped test, declined) |
| A M2 (code-review high) | 1 | ffc7c326:e0447ebf | CLEAN (1 MINOR fixed: unopenable clipboard stays a silent empty paste) |
| B M2 (conformance, sonnet) | 1 | same | CHANGES: 1 MAJOR (chord and middle-click paste never reached the image path; fixed via `paste_message`), 1 MINOR (comment) |
| B M2 (re-review of fix, sonnet) | 2 | scoped fix diff | PASS (1 MINOR: empty bracketed paste on empty clipboard, harmless) |
| A M3 (code-review high, fresh) | 1 | e2a5e7bd:5ef13056 | CHANGES: 1 MAJOR (state-dir container path joined with Path::join; fixed), 2 MINOR (fail-open without a running sandbox fixed; HostFacts probe removed) |
| B M3 (conformance, fresh) | 1 | same family | CHANGES: 1 MAJOR (image paste with no running sandbox was silent; fixed with a notice), 1 MAJOR (reviewer could not run Verify; the unit ran the three test commands green), 1 MINOR (data-dir choice not unit-tested; declined) |
| Close converge | 1 | 474f5937 | converged: all FRs/SCs built (M3 B had no round 2 and M4 rows were prose; code checked) |
| Close tdd-verify | 1 | 474f5937 | BLOCKED (test-after, 3 surviving mutants); remediated T035–T042 except T041 (partly, open), verdict kept as audited |

## Declined review findings

- M4 A/B MINORs: not-loaded-catalog sweep test, routing the sweep through `PastedLayout::remove`, spawn_blocking for delete: left as is (guards exist in code; dirs are small). Unused test binding fixed.

- B M1 F2 (widget-level FileDropped test): the pure hit-test, the reducer, the coalescing and the wiring are tested; a headless widget harness for one event path was judged out of proportion. Pointer-staleness during a native drag is an unverified platform risk recorded in research.md.
- B M1 F4: no `tdd/cycle-log.md`: tests were written beside the code; red-first was not recorded and is not invented now.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

- Tasks review (round 1, snapshot 3a1a5833:788b5d57): CHANGES; F1 (paste_source test task) fixed, F2-F4 MINOR fixed; no further round (tasks added, tests-only).
