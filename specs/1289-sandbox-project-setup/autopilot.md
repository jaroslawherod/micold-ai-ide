# Autopilot ledger — #1289 sandbox-project-setup

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on all tickets that are not yet in progress use speckit autopilot (issue #1289: Sandbox: per-project setup command and PATH for project toolchains); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1289
- **Worktree branch**: claude/project-thread-d7q05f
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: plan unit (clarify CLEAN; 2 provisional answers may still change if the user picks differently)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-009: where is the setup command defined? | A: per-project app setting in settings.json; no repo file read | provisional — recommended option, awaiting user answer | docs/user-guide/settings.md#jira (per-project base URL in settings.json) |
| 2 | clarify | Story 4: per-project image in scope? | A: deferred to follow-up; second container daemon + remembered Daemon select documented (FR-025), Out of scope | provisional — recommended option, awaiting user answer | docs/user-guide/worktrees-and-sessions.md ~466 (Daemon select remembered per project) |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b0a77d005ab504233e2ea45937bad3888c721813:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 PATH order + scenario; F2 stop vs disconnect; F3 FR-023a one run per worktree, notification per app; F4 bash/sh; F5 Default entry state/output; F6 citation) |
| Spec | 2 (sonnet, scoped) | daabf27317f59eec2cdebd72955b471611832070:8dead641fd56a494c0cee555f52cb82540b3d57d | CLEAN, 2 MINOR — both fixed in clarify (Story 1 scenarios renumbered in order; FR-011 names bash/sh/PowerShell and points to Platforms) |
| Clarify | 1 | 54752543 | 2 questions → provisional A/A applied (FR-009, FR-025, Out of scope, Story 4 removed) |
| Clarify | 2 | after round-1 answers | no critical ambiguities (UI placement and app→daemon command hand-off left to plan) → CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (Clarify Q1/Q2 asked of the user; provisional answers in *Decisions*.)

## Follow-ups not done

- Per-project sandbox image (spec Story 4, deferred by clarify Q2): needs its own issue.
