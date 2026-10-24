# Autopilot ledger — #1289 sandbox-project-setup

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on all tickets that are not yet in progress use speckit autopilot (issue #1289: Sandbox: per-project setup command and PATH for project toolchains); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1289
- **Worktree branch**: claude/project-thread-d7q05f
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks review round 2 (sonnet, scoped to fix diff), then specs-only gate and design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007 | full | sandbox home's ~/.local/bin and mise shims on PATH | — | pending |
| M2 | T008–T017 | full | core setup model, output buffer, shell, StreamingChild, settings field | — | pending |
| M3 | T018–T030 | full | protocol 69; daemon runs setup on create, streams output, MCP field, run/stop | — | pending |
| M4 | T031–T039 | full | records persist, restart → Interrupted, attach catch-up, delete stops first | — | pending |
| M5 | T040–T049 | full | Settings › Project, ProjectSetupSet sync, row tag, failure notification | — | pending |
| M6 | T050–T055 | full | Setup output dialog, delete confirmation line | — | pending |
| M7 | T056–T059 | light | Run setup / Stop setup menu entries | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-009: where is the setup command defined? | A: per-project app setting in settings.json; no repo file read | provisional — recommended option, awaiting user answer | docs/user-guide/settings.md#jira (per-project base URL in settings.json) |
| 2 | clarify | Story 4: per-project image in scope? | A: deferred to follow-up; second container daemon + remembered Daemon select documented (FR-025), Out of scope | provisional — recommended option, awaiting user answer | docs/user-guide/worktrees-and-sessions.md ~466 (Daemon select remembered per project) |
| 3 | plan | How does a daemon learn the setup command (MCP create_worktree has no client in the loop)? | Client stores it in settings.json and pushes ProjectSetupSet (in memory) to each daemon holding the project, on connect and on change | unit (research R4) | persist.rs sends service settings to active daemon only |
| 4 | plan | Which daemons get the home PATH folders? | Those with offered.json (app-owned container/pod) | unit (research R2) | provision::Ownership::AppOwned; MICOLD_IMAGE_REFERENCE absent on pods |
| 5 | tasks | Milestone cut vs plan's five suggestions | Seven: plan M3 split into M3 (run on create) / M4 (persistence, delete) by size; client cut along scenarios into M5 (command, sync, tag, notification), M6 (output view), M7 (US3 run/stop). M3 kept at 13 tasks (~1000 lines): protocol messages, their handlers and the run task cannot land apart without a second version bump or a server answering Unsupported. plan.md §Milestones updated to point at tasks.md | unit (milestones.md rules 2–3) | tasks.md §Milestones |
| 6 | tasks | How does the daemon look up `bash` (resolve_on_path is pub(crate) in core)? | New core `setup::shell::bash_on_path(&OsStr)` wraps it; data-model §shell_line updated | unit | provider.rs:575 private fn |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b0a77d005ab504233e2ea45937bad3888c721813:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 PATH order + scenario; F2 stop vs disconnect; F3 FR-023a one run per worktree, notification per app; F4 bash/sh; F5 Default entry state/output; F6 citation) |
| Spec | 2 (sonnet, scoped) | daabf27317f59eec2cdebd72955b471611832070:8dead641fd56a494c0cee555f52cb82540b3d57d | CLEAN, 2 MINOR — both fixed in clarify (Story 1 scenarios renumbered in order; FR-011 names bash/sh/PowerShell and points to Platforms) |
| Clarify | 1 | 54752543 | 2 questions → provisional A/A applied (FR-009, FR-025, Out of scope, Story 4 removed) |
| Clarify | 2 | after round-1 answers | no critical ambiguities (UI placement and app→daemon command hand-off left to plan) → CLEAN |
| Plan | 1 | f5e4485bc71dd9eaf5ff5dfa2d7a3c0444768d51:cfc387029e91b351cae9a77b2329e2e5b12d471a | CHANGES: 2 MAJOR, 3 MINOR — all fixed (F1 daemon-wide persisted RunId counter + A12/A16 recreate test; F2 delete awaits run end via stop_and_wait bounded by RUN_OVERRUN, stale finish dropped, A12 asserts no record/log; F3 C4 three values + Started(RunId); F4 M4 split into M4a/M4b; F5 ProjectSetupSet carries daemon-catalog path, A16 mapped case) |
| Plan | 2 (sonnet, scoped) | c6904f37533cfa3183cad458c3e458eb852b92f7:71a9120e6a1fca096755b00c0c38b356273d79f9 | CLEAN, 1 MINOR — fixed, prose only (data-model §RunId: id allocated in memory under the lock, counter file persisted just after release, before the record write) |
| Tasks | 1 | afa51c1f5757e7d989ae7915d46182fa1e6dc0f7:0b28553c5169f0e4d6cc8ce59e6cff80cbff1039 | CHANGES: 3 MAJOR, 2 MINOR — all fixed (F1 A4 tests split into setup_model/setup_output/setup_shell.rs, Parallel line and M2 Verify updated; F2 G2 test now T052 before impl T053, deps updated; F3 "T020–T022 pass" moved to T027, deps T020–T022 → T025–T027, T021 red until T025–T027; F4 T022 gains a WorktreeCreate positive control so it is red until T025–T027; F5 `delete_warns_setup` predicate case in T050/T051, T054 uses it) |
| Tasks | 2 (sonnet, scoped) | SNAP2 | pending — reviewer to dispatch |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (Clarify Q1/Q2 asked of the user; provisional answers in *Decisions*.)

## Follow-ups not done

- Per-project sandbox image (spec Story 4, deferred by clarify Q2): needs its own issue.
