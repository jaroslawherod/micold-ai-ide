# Autopilot ledger — #1792 structured-select-m3

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: feat: structured view: Material 3 select input and expand animation for the model, effort and permissions settings (issue #1792). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no <!-- `yes` when the run started with AUTOPILOT_SUPERVISED set: an escalation is recorded and the turn ends, no interactive question -->
- **Issue**: #1792
- **Worktree branch**: feat/1792_feat-structured-view-material-3-select-input-and
- **Started**: 2026-11-11
- **Phase**: M1 PR open
- **Next step**: orchestrator merges M1 PR; then M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1836 | Design | merged | e0894d44a661930bebb203ffd8e9e2d1554d039e |
| #1842 | M1 | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Material 3 trigger on the three structured selects | #1842 | open |
| M2 | T011–T012 | light | menu parity pinned (test-only if green) | — | pending |
| M3 | T013–T021 | full | options row expand/collapse animation; collapse closes pickers (T017/T020) | — | pending |

Polish T022–T023 (no code) is left to the close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|


## Review rounds

- tasks review round 1: snapshot 199ca061a9e675043e43796afbe22d627936dd42:0923e2cbffbbdc6215eb213142a9ae51bb4c669d — CHANGES (F1 MAJOR checklist ticked, 2 with waiver; F2 MAJOR paths added to T014-T018; F3 MINOR M2 note). Fixes are artifact text only (paths, ticks); checked against the findings, no new round. speckit-analyze: no CRITICAL/HIGH; C1-C7 MEDIUM fixed.
- plan review round 1: snapshot 2abfa9d1e636fc5404daaf01505ef89fbd1530d6:0725198f2bce1c07e7457288f82dcb60a582ea54 — CHANGES (F1 MAJOR duration undecided: fixed MEDIUM_2; F2-F4 MINOR fixed). Fixes are research/contract decisions, checked against the finding; no new round.

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec.md | n/a | 6fc85675fbf3d6f586add2d902fa44d1c23fbb8c:4b0e90f051efb9e5f2dbdfe85a4faf47c5575244 | CLEAN (3 MINOR: empty-list outcome fixed, checklist unticked; reduced-motion FR awaits clarify) |
| plan | 1 | design | d39de3e8e407a7fc9de2b4ab0619660de082bdb6 | 2abfa9d1e636fc5404daaf01505ef89fbd1530d6:0725198f2bce1c07e7457288f82dcb60a582ea54 | CLEAN |
| tasks | 1 | design | d1a229af7921b3585c363180a0ef535de79d0742 | 199ca061a9e675043e43796afbe22d627936dd42:0923e2cbffbbdc6215eb213142a9ae51bb4c669d | MINOR-ONLY fixes applied (prose/paths) |
| plan | 2 | design | 6008539e285d26039a6d672265c5ad935219a48c | 4911e995a35744f47c1e54d42e3558a096da5ebe:6008539e285d26039a6d672265c5ad935219a48c | CLEAN (re-review of fix diff since d39de3e8) |
| A | 1 | M1 | fc4869254e79369837838f82ad0cd6bc724440ca | 9bf76c49c8d226fd67193dd436febc0f16b1f301:a6ef0850eac034c3acd89097673bfed4ff004a84 | CLEAN (3 MINOR handled; code-review high) |
| B | 1 | M1 | fc4869254e79369837838f82ad0cd6bc724440ca | 9bf76c49c8d226fd67193dd436febc0f16b1f301:a6ef0850eac034c3acd89097673bfed4ff004a84 | CLEAN (MINOR: no tdd cycle-log, tests after implementation) |
| re-review | 2 | M1 | fc4869254e79369837838f82ad0cd6bc724440ca | d18df42aae0fb8766577d905b2ce643448e35008:fc4869254e79369837838f82ad0cd6bc724440ca | CLEAN (filled() opt-in and SelectTrigger exemption, sonnet; 1 MINOR test comment) |
| A | 2 | M1 | b5b53dc563ed7ac0f4d995b91affc24f90cb0abe | 036204b27bc5b3a64874237b02ba18aa3b41e429:b5b53dc563ed7ac0f4d995b91affc24f90cb0abe | CLEAN (docs-only fix diff since fc4869254; 2 MINOR fixed, prose only) |
| B | 2 | M1 | c3006f98426d537f9efdfbd610d68bfb786d8c99 | b3843889d265b8ecafb685cdd31a4fde808229b0:c3006f98426d537f9efdfbd610d68bfb786d8c99 | CLEAN (docs-only fix diff since fc4869254; 1 MINOR fixed, prose only) |
| re-review | 3 | M1 | c3006f98426d537f9efdfbd610d68bfb786d8c99 | de5364e74e35bf1faa0d4b82bfa4e5206887bd84:c3006f98426d537f9efdfbd610d68bfb786d8c99 | CLEAN (docs-only since fc4869254, no code) |
| A | 3 | M1 | 479f03320a9e5061f848e43dd1132ddac78464bc | 9653b12411aeb09ce729b80ac2af910ef9a517b6:479f03320a9e5061f848e43dd1132ddac78464bc | CLEAN (docs-only fix diff since b5b53dc56, no code) |
| B | 3 | M1 | 479f03320a9e5061f848e43dd1132ddac78464bc | 9653b12411aeb09ce729b80ac2af910ef9a517b6:479f03320a9e5061f848e43dd1132ddac78464bc | CLEAN (same head as B round 2; no change since) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. <or, one question:>

```
- **Category**: <number and name from rules/escalation.md>
- **Question**: <one sentence>
- **Options**:
  1. <label> (Recommended) — <description>
  2. <label> — <description>
- **Recommended**: <label> — <reason>
- **Evidence**: <paths, commands, results>
- **Banner**: sent <date>
```

<A batch: `### Q1`, `### Q2`... each holding Question, Options, Recommended and Evidence; Category and
Banner stay above them. `scripts/autopilot/run-status.sh` reads these fields.>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
