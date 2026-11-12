# Autopilot ledger — #1813 supervisor-skill

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M2: supervisor skill that queues issues and starts sessions (issue #1813); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1813
- **Worktree branch**: feat/1813_autopilot-supervisor-m2-supervisor-skill-that
- **Started**: 2026-11-12
- **Phase**: plan
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | … | #… | pending / in-progress / in-review / ci / merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |
| D5 | plan | Plan review round 3 still found 2 MAJOR; accept hand fixes or one more round? | Accept fixes (commit 09892a167); no fourth round | decided by user | AskUserQuestion 2026-11-12 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Review B M1 | 1 | M1 | <git rev-parse HEAD, 40 hex> | <review-snapshot.sh tree:head> | CHANGES: 2 MAJOR |
| Review spec | 1 | spec | 62f51d14dbd48b0ba3434ad53aed66a19cd26b65 | c1af633993e0d08fce04690906bf0074fe12df1d:62f51d14dbd48b0ba3434ad53aed66a19cd26b65 | CHANGES: 3 MAJOR, fixed |
| Review spec | 2 | spec | 0fb591fff8b90b2d0562841744f93891dd323802 | 6362a8c68621e9b59d41d9acad67d623b69dbd07:0fb591fff8b90b2d0562841744f93891dd323802 | CLEAN (3 MINOR fixed, prose only) |
| Review plan | 1 | design | 09892a1672a95d7ec7f8772b8f1fe16d30dd566a | 215dda838db4069559cf0af8fc45ed9bf133cc53:3cb9cd476b47253d273ced1dd535825e4c22ba4a | CHANGES: 3 MAJOR, fixed |
| Review plan | 2 | design | 09892a1672a95d7ec7f8772b8f1fe16d30dd566a | 282ce57f9c076ebb5fa2df68e3a6b2cb74dd0d7c:9c767eb4ab5e77c5bbbb7976e8aad1b2e4b276fb | CHANGES: 1 MAJOR, fixed |
| Review plan | 3 | design | 09892a1672a95d7ec7f8772b8f1fe16d30dd566a | 8fefdbf6fba62a8c2ff5177f66f97b4f3f284331:65b3da22ecaa145ce24e100a77aa940e8e0c51a0 | CHANGES: 2 MAJOR (spec/quickstart consistency), fixed, not re-reviewed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
