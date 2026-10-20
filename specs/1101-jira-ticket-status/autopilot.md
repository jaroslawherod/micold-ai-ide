# Autopilot ledger — #1101 jira-ticket-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Show a Jira ticket's current status (issue #1101); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1101
- **Worktree branch**: feat/1101_show-a-jira-ticket-s-current-status
- **Started**: 2026-10-20
- **Phase**: tasks
- **Next step**: M1 (milestone unit) after the design PR merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1152 | Design PR | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Status chip in the issue picker | | pending |
| M2 | T014–T027 | full | Current status on the sidebar | | pending |
| M3 | T028–T030 | full | Status on Overview cards | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1152 | Design PR | open | |
| Spec | 1 | 952ec53aa3d23346f6fed8f12d836bcf43d073d0:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CHANGES: 4 MAJOR |
| Spec | 2 | 2791b5d842f73b2d98560fad5de1801a962e828a:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CHANGES: 1 MAJOR (not-found contradiction), 2 MINOR; fixed, checked by author (wording reconciliation only) |
| Plan | 1 | 87105a5c80cf11e780ace91fd3629036eddc60f5:f3cacdb0b33caca2142ce3ac8f3e8ef396c86102 | CHANGES: 3 MAJOR (400 on unknown key, project-open hook, epoch), 3 MINOR; all fixed in plan/research/data-model/contracts (design decisions added: bisection on 400, epoch, daemon_sync hook) |
| Plan | 2 | 1efaa84c3741213ab69f9dbf3dfb2877567fc01f:f3cacdb0b33caca2142ce3ac8f3e8ef396c86102 | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | 6807400cb49973f201fe92f29c7f7bf9c08cd521:1a784129401f5b853ac8d121376a401053af3260 | CHANGES: 2 MAJOR (shared-file [P], accessible_text test order), 3 MINOR; fixed, checked by author (task wording/ordering only); analyze HIGH C1, C2, I1 fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1152 | Design PR | open | |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
