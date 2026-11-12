# Autopilot ledger — #1724 orchestrator-registry-daemon-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1724 "feat: `micold-daemon orchestrator` — registry, daemon link, heartbeat, offline" (part of #1722; covers FR-001, FR-002, FR-003, FR-010). labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1724
- **Worktree branch**: feat/1724_feat-micold-daemon-orchestrator-registry-daemon
- **Started**: 2026-11-11
- **Phase**: milestone M1
- **Next step**: full gate green, push (force-with-lease), open M1 PR, hand to orchestrator

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1841 | Design | merged | 4652701784d3fba4add8d503d6a6ee614ec7d7f8 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | orchestrator starts, links and lists a daemon, no session hosted | #… | in review |
| M2 | T017–T025 | full | heartbeat (LinkPing/LinkPong, protocol 81), offline and back online | #… | pending |
| M3 | T026–T030 | full | older-protocol daemon refused and listed needs-update | #… | pending |
| M4 | T031–T035 | full | one live link per id, reservation expiry | #… | pending |
| M5 | T036–T040 | full | registry persisted, restart lists all offline | #… | pending |
| M6 | T041–T043 | full | no-outbound proof (source test, netns), quickstart recorded | #… | pending |

M1 has 16 tasks (over ~10): Setup+Foundational+US1 cannot split without a milestone with no observable deliverable. Store moved to M5 so M1 stays in-memory. Next step: tasks analyze and review.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Clarify

Round 1: 3 questions, all agent-resolved from the repo (local socket, JSON file atomically replaced, keepalive constants); no escalation, no new requirement.

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec review | 1 | spec | aefdd44 | aefdd445998f481ca6d90346e166619373ec0f18:6f3f8c1eb35c332d2bcb12a8a26721384b0a327e | CHANGES: 3 MAJOR (fixed; F3 declined) |
| Spec review | 2 | spec fix diff | 26483b0 | 4a600a9093ae9f31e49c119977bea2be43dd4935:26483b071f6c280f063e568cd199eac4edfc4a45 | CLEAN (2 MINOR fixed, prose only) |
| Spec review | 2 | spec | a634d456f5dce2a86ef52452fb662a38ca838325 | 4a600a9093ae9f31e49c119977bea2be43dd4935:26483b071f6c280f063e568cd199eac4edfc4a45 | CLEAN |
| Plan review | 3 | plan fix diff | 083fc61c928b0d8ce21a594b04abb48d560e3fe4 | 7148d52d533661e32da2d27c68b70ccbca4522ed:b5c73c9b8026e32e151b9010fb4fe81a427e49d9 | CLEAN (rounds 1-2: 2 MAJOR each fixed; 2 MINOR fixed, prose only) |
| Tasks review | 1 | design | e54fe2ac892095469bf8be6f87367263f851fc04 | 673e0707cfeb5edb3ef35ec5c344958f332e2803:a072867aaeee867f059df4bfbe37482ecec78c6e | CHANGES: 1 MAJOR (fixed) |
| Tasks review | 2 | design | 0ce895e9c35ffccbfb9286ff28c6ea420909299e | 9ad55870f0db7e677deeb1fae7de1961d3fd11aa:4ffca1f8894248ad587365c08647f34375bfcff1 | CLEAN after 1 MAJOR fixed (T010 stubs, task text only, self-checked) |
| Design review | 3 | 1841 | 1c98492e91d156a3e3ea9266e07bb50b22ce4efa | fd50a3aac7fb35f49b943487c4b3db7ecfe12203:1c98492e91d156a3e3ea9266e07bb50b22ce4efa | CLEAN |
| Review A | 1 | M1 | 5c59d3f85a518f02f1c7905c62fdf46deac74115 | 199f867e401c566658aba6d806dc0ce1497f0bfa:5c59d3f85a518f02f1c7905c62fdf46deac74115 | CHANGES: fixed accept-error exit, stderr tracing, vacuous child test; declined Ping/offline/read-timeout (M2), VersionMismatch (M3), auth, MINORs |
| Review B | 1 | M1 | 5c59d3f85a518f02f1c7905c62fdf46deac74115 | d6cc4d875c0d02cea290fa072f377ffe5504906a:32a0514a3f960be76b65f4e701e38e0a2bc4f516 | CHANGES: 3 MAJOR fixed by drift-M1.md, tdd/cycle-log.md (no red captured, recorded), Verify via full gate; MINOR declined |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | F3: three open markers block testability | Template allows three; clarify unit resolves them |
| tasks | analyze | F10 split T014/T010; F14 move T041 to M1; F16 dup note | Each task is one coherent file pair; the no-outbound proof needs the whole feature; T018/T021 mention is harmless |

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
