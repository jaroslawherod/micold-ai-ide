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
- **Next step**: orchestrator asks the user the 2 questions in *Open escalation*; continue the clarify unit with the answers (record as decided by user, apply to spec.md FR-009 / Story 4, run next clarify round)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b0a77d005ab504233e2ea45937bad3888c721813:0e5a1e9bb48c633b80eca5a20ce7c9ba3d807316 | CHANGES: 3 MAJOR, 3 MINOR — all fixed (F1 PATH order + scenario; F2 stop vs disconnect; F3 FR-023a one run per worktree, notification per app; F4 bash/sh; F5 Default entry state/output; F6 citation) |
| Spec | 2 (sonnet, scoped) | daabf27317f59eec2cdebd72955b471611832070:8dead641fd56a494c0cee555f52cb82540b3d57d | CLEAN, 2 MINOR — both fixed in clarify (Story 1 scenarios renumbered in order; FR-011 names bash/sh/PowerShell and points to Platforms) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product/scope decision the repo does not settle):

1. **FR-009 — where is a project's setup command defined?**
   - (A, Recommended) App setting per project, entered in the app, kept in `settings.json`. Precedent: the per-project Jira base URL is kept in `settings.json` (`docs/user-guide/settings.md` "Jira", ~line 797). No command from a cloned repo runs without the user typing it.
   - (B) A file checked into the repo (e.g. `.micold/setup`): travels with the code, but a cloned repo runs commands on worktree creation with no consent step.
   - (C) Both, app setting wins when set: most flexible; needs a trust/consent rule for the file, more scope.
2. **Story 4 — per-project image: in scope?**
   - (A, Recommended) Defer to a follow-up issue; ship Stories 1–3. Already reachable today: add a second container daemon with **My own image** (`FROM micold-basic`), and the New worktree form's Daemon select is remembered per project (`docs/user-guide/worktrees-and-sessions.md` ~line 466). Story 4 becomes a doc note pointing to that.
   - (B) A project's image starts its own container for that project's worktrees, with its own home: new container lifecycle per project, large scope.
   - (C) Per-project choice of default daemon (with its own image), surfaced in project settings: mostly exists already via the remembered Daemon select.

## Follow-ups not done

None yet.
