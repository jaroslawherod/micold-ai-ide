# Autopilot ledger — #943 structured-commands-fallback

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 943 — issue #943 "Structured session: slash commands, model switching and PTY fallback" (epic #945); labels: enhancement, flow:feature (in-progress added at start)
- **Kind**: feature
- **Effort**: default
- **Issue**: #943
- **Worktree branch**: claude/project-thread-k392yq
- **Started**: 2026-10-22
- **Phase**: clarify
- **Next step**: clarify unit — apply the user's answer to Open escalation (US1 s5 `/clear`), then next clarify round; US5 s2 agent-resolved (round 1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| 1 | clarify r1 | Terminal-period turns shown in the transcript? (US5 s2) | Only marked, not rebuilt from the CLI store; FR-023 updated | agent | 938 spec US7; 582 Out of Scope |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 67da24dc3a70beca179b57d029e7f152bc1a7ab6:dc6473291290b51830029c741e2fe6ab5630544e | CHANGES: 2 MAJOR, 3 MINOR — all fixed |
| Spec review | 2 (sonnet, fix diff) | 910550013b1158c635de912a8ee68881ce5de668:dc6473291290b51830029c741e2fe6ab5630544e | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, Q1 (category 1, product decision). After `/clear` in a structured session, which
conversation does a later restart, resume or switch of kind continue? Evidence: the session id is
the CLI's conversation uuid and `--resume` handle (crates/micold-daemon/src/state.rs#launch modes);
`/clear` makes the CLI continue under a new uuid (BUG-904, specs/582-.../bugs/BUG-904.md), and in
stream-json it emits a new init carrying the new `session_id`. BUG-904 decision 1 / 582 FR-017
accepted, for terminal sessions, that the session keeps its id and a later restart resumes the
PRE-clear conversation. For 943 that means a switch to terminal after `/clear` brings back the
conversation the user cleared, against US5 s1 / FR-020.
- A (Recommended): follow the CLI in structured sessions — the session keeps its identity but
  records the conversation handle the latest init reported and resumes that one on every later
  launch (restart, daemon restore, switch); terminal sessions keep the BUG-904 trade-off.
- B: keep the BUG-904 rule everywhere — later launches resume the pre-clear conversation; the
  `/clear` marker and the guide say so.
- C: app-handled `/clear` — the app ends the CLI and starts a new conversation under its control.

## Follow-ups not done

None yet.
