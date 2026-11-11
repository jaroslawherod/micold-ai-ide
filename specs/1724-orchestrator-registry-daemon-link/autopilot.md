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
- **Next step**: push (force-with-lease needed: origin holds 4 pre-rebase twins of local commits), open design PR, then M1 after merge

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1841 | Design | merged | 4652701784d3fba4add8d503d6a6ee614ec7d7f8 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | orchestrator starts, links and lists a daemon, no session hosted | #… | pending |
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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | Spec review 1 | F3: three open markers block testability | Template allows three; clarify unit resolves them |
| tasks | analyze | F10 split T014/T010; F14 move T041 to M1; F16 dup note | Each task is one coherent file pair; the no-outbound proof needs the whole feature; T018/T021 mention is harmless |

## Handover

M1 unit, written at 150k context. PR #1841 is recorded merged above. All M1 code is written and
committed in one WIP commit, but it was NEVER COMPILED OR RUN: other worktrees held the shared build
lock for 40+ minutes, so no `cargo` ran. No red phase was captured; the next unit compiles first and
fixes what the compiler and tests say. A detached `cargo test -p micold-core` of mine may still be
queued on the lock (log in the old scratchpad); it is harmless.

Written (tasks.md ticks NOT done; tick T001-T016 once green):
- core: `crates/micold-core/src/orchestrator/{mod,record,announce,registry}.rs` (tests included for
  T004 T005 T006; impl T007 T008 T009), `pub mod orchestrator` in lib.rs, `Kind` derives `Serialize`
  plus a round-trip test in `daemon_config/tests.rs` (T001 T003). Registry M1 API: `claim(&ann)`,
  `confirm(link, wall_secs)`, `closed(link)`, `list()`, `Snapshot{seq,records}`; no `now`/Keepalive
  yet (M2 adds it); `LinkAction::{Stale,Idle}`; a reserved or live id refuses a rival.
- daemon: `crates/micold-daemon/src/orchestrator/{mod,link,query}.rs` (T013 T014: `Mode`,
  `parse_mode`, `Clock`, `SystemClock`, `run`, `run_with(socket, clock)`, `main()`; uses
  `singleton::acquire` on `orchestrator.sock`/`.lock` beside the daemon's), `main.rs` dispatch (T015),
  `lib.rs` `pub mod orchestrator`, `Cargo.toml` `[[test]] orchestrator` (T002), `catalog.rs` free
  `wire_settings(&Settings)` used by `Catalog::settings_wire` and the orchestrator's Welcome.
- tests: `crates/micold-daemon/tests/orchestrator/{main,support,link_and_list,not_hosted,dispatch}.rs`
  plus one-line stubs for heartbeat, netns, no_outbound, one_link_per_id, restart, version_mismatch
  (T010 T011 T012). Session start = `SessionCreate{req}` answered `OperationError{Refused,
  "not hosted by the orchestrator"}`; session list = `Attach` answered `Attached` with no sessions.
- docs: `docs/daemon.md` "Orchestrator mode (#1724)", `docs/user-guide/orchestrator-mode.md`,
  `docs/SUMMARY.md` (T016).

Next steps: 1) `scripts/build-lock.sh cargo test -p micold-core --lib -- orchestrator daemon_config`
(`preamble_expired` is a pure helper the daemon does not use; it uses `tokio::time::timeout`);
2) `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator` (watch: `Framed::map_codec`
after the preamble, `Stream` Unpin, clippy); 3) `every_test_file_is_wired` and one existing daemon
suite (SC-006); 4) tick T001-T016, `mise run test-core`, quickstart Part B steps 1, 2, 7, then
implement.md step 3 onward (verify.md, review A and B, gate, PR). Review rounds: none yet.

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
