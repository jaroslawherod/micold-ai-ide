# Autopilot ledger — #1724 orchestrator-registry-daemon-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1724 "feat: `micold-daemon orchestrator` — registry, daemon link, heartbeat, offline" (part of #1722; covers FR-001, FR-002, FR-003, FR-010). labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1724
- **Worktree branch**: claude/project-thread-430lii
- **Started**: 2026-11-11
- **Phase**: milestone M6
- **Next step**: M6 implemented and committed; Review A and B, then PR (orchestrator opens it)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1841 | Design | merged | 4652701784d3fba4add8d503d6a6ee614ec7d7f8 |
| #1849 | M1 | merged | 21de2080eb685861ea423f52d4ba4a71428d9798 |
| #1871 | M2 | merged | b05740e146295f8e3dde4b5fe2499359aef7d17a |
| #1892 | M3 | merged | 8bcf7894b91548a647319de47fecca688e0f74af |
| #1895 | M4 | merged | fa0e41a737c21651652a6954264cefd2d07b1065 |
| #1898 | M5 | merged | da60fabfaa8264ac0b74d9b9deee5865339b694f |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | orchestrator starts, links and lists a daemon, no session hosted | #1849 | merged |
| M2 | T017–T025 | full | heartbeat (LinkPing/LinkPong, protocol 81), offline and back online | #1871 | merged |
| M3 | T026–T030 | full | older-protocol daemon refused and listed needs-update | #1892 | merged |
| M4 | T031–T035 | full | one live link per id, reservation expiry | #1895 | merged |
| M5 | T036–T040 | full | registry persisted, restart lists all offline | #1898 | merged |
| M6 | T041–T043 | full | no-outbound proof (source test, netns), quickstart recorded | #… | in progress |

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
| Review A | 1 | M3 | c0f5e236b0c42f8c345c67f906e4d49d0ec75bd3 | 462e410b0aa2664d012347bfd57afea4b9f4b1f6:c0f5e236b0c42f8c345c67f906e4d49d0ec75bd3 | CLEAN: fixed F1 (Hello's version recorded), F2 (live-link and silent-link wire tests) |
| Review B | 1 | M3 | c0f5e236b0c42f8c345c67f906e4d49d0ec75bd3 | 462e410b0aa2664d012347bfd57afea4b9f4b1f6:c0f5e236b0c42f8c345c67f906e4d49d0ec75bd3 | CHANGES: fixed F1 MAJOR (drift-M3.md), F2 (wire test), F3 (T027 text); round 2 pending on e606147c |
| Review A | 2 | M3 | e606147c373b9ecc4e2f7b8e22d10901cd4ddffe | d87571396ed0129d5f067f12db6fb698ff2ae759:e606147c373b9ecc4e2f7b8e22d10901cd4ddffe | CLEAN (declined MINOR: close the silent link right after the offline wait; FR-006 says the mismatched claim closes it) |
| Review B | 2 | M3 | e606147c373b9ecc4e2f7b8e22d10901cd4ddffe | d87571396ed0129d5f067f12db6fb698ff2ae759:e606147c373b9ecc4e2f7b8e22d10901cd4ddffe | CLEAN: F1-F3 fixed, no plan drift for M3, Verify 9 passed |
| Review A | 1 | M4 | ac07209bf661c1bfbf127210d11ce041b8f82800 | 1e3b41175be72fc247fe7e2dc62334c6b02f8b44:a98b297aefbee75c4cd4df7282956739262a8837 | MINOR-ONLY: CLEAN, 3 MINOR fixed (F1 Refused after AlreadyLinked confirm, wire test red-first; F2 pong-time assertion; F3 shared record_when) |
| Review B | 1 | M4 | ac07209bf661c1bfbf127210d11ce041b8f82800 | 1e3b41175be72fc247fe7e2dc62334c6b02f8b44:a98b297aefbee75c4cd4df7282956739262a8837 | CLEAN: 1 MINOR declined (T034 Dropped wire test); Verify one_link_per_id 5 passed, core orchestrator 69 passed, plan-drift clean |
| Review A | 1 | M5 | d4cb49cb237d3deba9aa3c724d5a666a380c5dd1 | aac9cfc19474c5fac0d8f3d0538564de47022218:cb8173dc616777c337ad5fcf63471e7300ecf735 | CHANGES: fixed F1 (CHILDREN lock), F2+F3 (one saving thread, coalesced, joined on stop; red-first unit test), F4 (list after stop); round 2 pending |
| Review B | 1 | M5 | d4cb49cb237d3deba9aa3c724d5a666a380c5dd1 | aac9cfc19474c5fac0d8f3d0538564de47022218:cb8173dc616777c337ad5fcf63471e7300ecf735 | CLEAN: core orchestrator 82 passed, daemon orchestrator 35 passed, no plan drift for M5 |
| Review A | 2 | M5 | 35bd7e8cc4660bb44785f66a5ffa553f1c573127 | 276e72832ecb6c3d8857e2357901f971a0a38805:35bd7e8cc4660bb44785f66a5ffa553f1c573127 | CLEAN: round-1 fixes hold; 1 MINOR declined (save thread panic unlogged) |
| Review A | 1 | M6 | 617dba58e2c2632676ed27cdf65b7245a322615b | 9f8b399a5314ad8c02f6c244229215019acca3bc:617dba58e2c2632676ed27cdf65b7245a322615b | CHANGES: F1 BLOCKER (strace of connect calls added), F2 (probe asserts NetworkUnreachable and an outer loopback listener), F3 (scan covers dialing paths, self-probe allow-listed, docs reworded), F4 (named tests must run), F5 (char literals); round 2 pending |
| Review B | 1 | M6 | 617dba58e2c2632676ed27cdf65b7245a322615b | 9f8b399a5314ad8c02f6c244229215019acca3bc:617dba58e2c2632676ed27cdf65b7245a322615b | CHANGES: F1 cycle-log M6 red/green added, F2 drift-M6.md added, F3 CI gate to confirm; no re-review of B needed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | F3: three open markers block testability | Template allows three; clarify unit resolves them |
| M2 | Review A 1 | F1/F2: transport end or Goodbye holds the id slot until the 9 s deadline | Spec and plan: a drop is silence, health stays until the deadline; releasing a closed-transport slot for a redial is the claim table of M4 (T031+) |
| M2 | Review A 1 | F4 snapshots discarded; F6 no dialer answers LinkPing | Saver lands in M5; the dialer is outside #1724 |
| M2 | Review B 1 | F1 retroactive red; F2 vacuous arm tests | Already recorded in cycle-log and drift-M2; the heartbeat behaviours are covered by non-vacuous tests (two failed and were fixed in this round) |
| tasks | analyze | F10 split T014/T010; F14 move T041 to M1; F16 dup note | Each task is one coherent file pair; the no-outbound proof needs the whole feature; T018/T021 mention is harmless |
| M3 | Review A 1 | none declined | F1, F2 fixed; B F1 (drift-M3.md), F2 (wire test), F3 (T027 text) fixed |
| M3 | Review A 2 | F1 assert closure right after the offline wait | FR-006: the mismatched claim closes a silent link; an offline link is not closed by silence alone, so closure after the refusal is the behaviour under test |
| M4 | Review B 1 | F1: T034's explicit `claim_failed(Dropped)` has no wire test that tells it from M3 | No wire-observable difference: `link()` calls `closed(link)` after every path, which releases the reservation the same way; the core unit test covers `Dropped` (cycle-log, M4) |
| M5 | Review A 1 | none declined | F1 (CHILDREN lock), F2/F3 (saving thread, drift-M5), F4 (list after stop) fixed |
| M5 | Review A 2 | F1 MINOR: a panic in the `orchestrator-save` thread goes unlogged (`let _ = thread.join()`) | The store returns errors and has no panicking path; follow-up noted |

## Handover

M5 squash-merged 2026-11-14 as da60fabf (PR #1898). CI: all jobs green on 8f2c9a7b first run. Review A round 1 CHANGES (2 MAJOR, 2 MINOR, all fixed in d4cb49cb), round 2 scoped (sonnet) CLEAN with 1 MINOR declined; Review B round 1 CLEAN. origin/main merged into the branch after the merge. Cloud thread branch: `claude/project-thread-3cm73q`. Next thread: `/speckit-autopilot resume #1724`, which starts M6 (T041–T043, no-outbound proof, quickstart recorded) from this branch, then the close unit.

## Open escalation

None. (The 2026-11-13 question on red main is settled: 1863 was fixed on main, option 1.)

## Follow-ups not done

- jaro (2026-11-14, web client planning thread): the orchestrator should use a relational database for storage, assume PostgreSQL. M5's persistence is behind the `RegistryStore` trait (`crates/micold-core/src/orchestrator/store.rs`), held as `Arc<dyn RegistryStore>`; the JSON store is built only at `crates/micold-daemon/src/orchestrator/mod.rs` (`JsonRegistryStore::in_data_dir`), so a Postgres store can replace it. Not in #1724's scope.
- M5 re-review MINOR: log a join error of the `orchestrator-save` thread in `crates/micold-daemon/src/orchestrator/save.rs` `Drop` (a panic there would silently stop saving).
- M4 re-review MINOR: the pre-steady handshake sends in `crates/micold-daemon/src/orchestrator/link.rs` (both `Refused`, `Welcome`) are unbounded, unlike `send_bounded` in the steady state; wrap them in one `LIVENESS_DEADLINE` timeout later.
- Possibly flaky on CI (seen on PR #1895, passed on re-run): `sandbox_real_no_outbound_still_resolves_names` (outside DNS), `structured_permission::two_sessions_waiting_are_marked_separately`.
