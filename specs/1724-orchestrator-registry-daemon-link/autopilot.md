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
- **Phase**: milestone M2
- **Next step**: wait for M2 CI and merge, then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1841 | Design | merged | 4652701784d3fba4add8d503d6a6ee614ec7d7f8 |
| #1849 | M1 | merged | 21de2080eb685861ea423f52d4ba4a71428d9798 |
| #1871 | M2 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | orchestrator starts, links and lists a daemon, no session hosted | #1849 | merged |
| M2 | T017–T025 | full | heartbeat (LinkPing/LinkPong, protocol 81), offline and back online | #1871 | in review |
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
| Review A | 2 | 1849 | dc4be4463cf3af17723b84edf3f7f83fce9f37ce | c2dde15b7b31e2f5255141b5a33838fd770ea022:dc4be4463cf3af17723b84edf3f7f83fce9f37ce | CLEAN: round-1 fixes hold, declined reasons stand |
| Review A | 1 | M2 | f55a3107f | 39c243ff36055d9c49d128dc0e59cd7ca0cd59a7:f55a3107ffe3b3fd7538a51692d4c4e49e7dd753 | CHANGES: fixed F3 (bounded sends), F5 (probe clock reset), scoped-gate liveness tests; declined F1/F2/F4/F6 |
| Review B | 1 | M2 | f55a3107f | same | CHANGES: 2 MAJOR accepted as recorded deviations (no red for T017-T022, T022 arm vacuous; see cycle-log, drift-M2); Verify run by full gate |
| Review B | 2 | 1849 | dc4be4463cf3af17723b84edf3f7f83fce9f37ce | c2dde15b7b31e2f5255141b5a33838fd770ea022:dc4be4463cf3af17723b84edf3f7f83fce9f37ce | CLEAN: Verify 37+11 pass; MINOR drift note added |
| Review A | 2 | M2 | 409c8ac49e734c2f9c2c3d0165f769edfd687303 | b6ddee0a059aa4ebdaea0c39799a383f77116533:409c8ac49e734c2f9c2c3d0165f769edfd687303 | CLEAN: 3 MINOR (send timeout, ignore-arm tests are guards, ping-reset assertion) not fixed |
| Review B | 2 | M2 | 409c8ac49e734c2f9c2c3d0165f769edfd687303 | b6ddee0a059aa4ebdaea0c39799a383f77116533:409c8ac49e734c2f9c2c3d0165f769edfd687303 | CLEAN: F1 MAJOR (no red for T017-T022) declined, gap recorded in cycle-log, T022 waiver in drift-M2; heartbeat 4 pass; plan-drift clean; cadence note added |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | F3: three open markers block testability | Template allows three; clarify unit resolves them |
| M2 | Review A 1 | F1/F2: transport end or Goodbye holds the id slot until the 9 s deadline | Spec and plan: a drop is silence, health stays until the deadline; releasing a closed-transport slot for a redial is the claim table of M4 (T031+) |
| M2 | Review A 1 | F4 snapshots discarded; F6 no dialer answers LinkPing | Saver lands in M5; the dialer is outside #1724 |
| M2 | Review B 1 | F1 retroactive red; F2 vacuous arm tests | Already recorded in cycle-log and drift-M2; the heartbeat behaviours are covered by non-vacuous tests (two failed and were fixed in this round) |
| tasks | analyze | F10 split T014/T010; F14 move T041 to M1; F16 dup note | Each task is one coherent file pair; the no-outbound proof needs the whole feature; T018/T021 mention is harmless |

## Handover

None.

## Open escalation

```
- **Category**: 5 Non-convergence, blocked by work outside my flow (red main)
- **Question**: PR 1871 (M2) is red only in jobs red on main too; merge it over them, or wait for main to be fixed?
- **Options**:
  1. Fix 1863 first on its own PR, then rerun 1871 (Recommended) - the container (docker, podman) and kubernetes real-runtime tests find micold-daemon by walking up from current_exe, which breaks under the rustc 1.100 test layout; same failure on main run 43358686770.
  2. Merge 1871 over the red jobs - they do not touch M2 code (orchestrator link, registry, liveness, protocol).
- **Recommended**: Fix 1863 first - keeps main green and the gate honest.
- **Evidence**: run 43402845520: docker job sandbox_real.rs:364 "no micold-daemon build to copy into the container", kube_real_support/mod.rs:403; identical on main 43358686770 (podman, docker, kubernetes). Issue 1863 open. The sandbox against a real runtime job failed at sandbox_real_lifecycle.rs:243 (sandbox_real_no_outbound_still_resolves_names, DNS), a micold-core sandbox test M2 does not touch; main's run did not fail it, so likely a flake; rerun issued (gh run rerun 43402845520 --failed).
- **Banner**: sent 2026-11-13
```

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
