# Autopilot ledger — #804 session-service-in-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Move the Session service settings (container runtime, limits) into the Daemons manager (#804); labels: enhancement, flow:feature, effort:low
- **Kind**: feature
- **Effort**: low
- **Issue**: #804
- **Worktree branch**: feat/804_move-the-session-service-settings-container
- **Started**: 2026-10-15
- **Phase**: tasks
- **Next step**: M1 after design PR merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #869 | Design | merged | 4b96bf2b526ca9b42e7dd9993d4b437016f49f18 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Container settings in the Daemons form | #… | in review |
| M2 | T014–T021 | full | Session service section removed | #… | pending |
| M3 | T022–T023 | light | Existing settings carry over | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b13a0044e288eb4d69133c3183da0ba44d895f63:1bf62326e8f9f29b3ef9df261c3ddba110f6ae54 | CHANGES: 4 MAJOR (fixed; F2 routed to clarify) |
| Spec | 2 | 23e1dba8160d1800dfdd6d8915d4550176991274:1bf62326e8f9f29b3ef9df261c3ddba110f6ae54 | CLEAN (1 MINOR: remote-daemon marker has no FR; for clarify) |
| Plan | 1 | e8b9da12b0d78a98730b2678305343a44c3e1daa:506df1d556f5f8a14ab742264002c269ec5851fa | CHANGES: 2 MAJOR (badge source, budget error type) fixed; 3 MINOR fixed |
| Plan | 2 | 2ae1b02bed2b8ad66a62b54a31fda380948a8911:d0659308fae210beb1fdf3989b8f0b8d68845619 | CLEAN (1 MINOR: name parse_budget in plan D1, in data-model) |
| Review B M1 | 1 | ef3356982b2d:e016b011a15f | CHANGES: 1 MAJOR declined (no red evidence), 1 MINOR fixed |

## Declined review findings

- Review B M1 F1 (MAJOR, no red-before-green evidence for T005-T007): cannot be produced after the fact; git history is one WIP commit. Recorded honestly in tdd/cycle-log.md. F2 fixed (T010 wording). F3 covered by the cycle log.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

- Tasks review round 1: 2 MAJOR (T018/T009 paths) fixed, 3 MINOR fixed; not re-reviewed (path/order fixes only).
