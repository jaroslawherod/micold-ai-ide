# Autopilot ledger — #490 claude-plan-usage

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 490 — implement GitHub issue #490 "Show Claude plan usage and next limit reset" (show current usage and the next limit reset for the signed-in Claude account in the status area or Settings; warn past a configurable threshold; show and log nothing alarming when data is unavailable; works offline; no credentials read beyond the chosen source's needs, none leave the machine except to the provider; undocumented endpoints not used). labels: enhancement, flow:feature (in-progress added at claim)
- **Kind**: feature
- **Effort**: default
- **Issue**: #490
- **Worktree branch**: claude/project-thread-u1pay5
- **Started**: 2026-10-10
- **Phase**: clarify
- **Next step**: clarify unit — resolve FR-012 (usage source) and FR-001 (default on/off; depends on FR-012)

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
| Spec | 1 | 9b2c2be0c4f91c4d56352d31de30d7e240d7d52b:149cda625a4e7942faafdff528924971fb1bd223 | CLEAN (3 MINOR, all fixed: FR-019 + US3 scenario 6 for account change, FR-007 single 5-minute interval, checklist testable item unticked) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product decision the repo does not settle). Evidence: the only documented source that reports plan percentages and reset times is Claude Code's status line JSON (`rate_limits.five_hour` / `seven_day`, `used_percentage` 0-100, `resets_at` epoch s; present only for Pro/Max subscribers and only after a session's first API response; https://code.claude.com/docs/en/statusline). The Admin/usage APIs cover API-key orgs, not plans; `/api/oauth/usage` is undocumented (forbidden by the issue); local logs give token counts, not plan %/reset. The app already hands each Claude session a `--settings` file (crates/micold-daemon/src/state.rs:872, feature 026 hooks), so a status-line command can ride in it; it must chain the user's own status line so theirs keeps working.

- Q1 (FR-012): Which usage source? A (Recommended) Claude Code status line `rate_limits` via the app's existing `--settings` file, chaining the user's own status line; indicator shows only while an app-launched Claude session has a reading. B Local usage logs (estimate, no plan %/reset; fails FR-004/SC-002). C Drop the feature until Anthropic documents a standalone usage API.
- Q2 (FR-001): Default of the switch? A (Recommended) On: with source A no credential is read and nothing leaves the machine beyond the CLI's own traffic, the spec's stated condition for on. B Off (opt-in like PR status on worktrees).


## Follow-ups not done

- Plan usage for other AI CLIs (issue's open question): out of scope for this feature, a later request.
