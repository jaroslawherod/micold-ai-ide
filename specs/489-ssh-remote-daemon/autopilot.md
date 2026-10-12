# Autopilot ledger — #489 ssh-remote-daemon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Run the session daemon on a remote host over SSH (GitHub issue #489); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #489
- **Worktree branch**: feat/489_run-the-session-daemon-on-a-remote-host-over-ssh
- **Started**: 2026-10-12
- **Phase**: implement M1
- **Next step**: M1 implement, continue from Handover (fresh unit)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #789 | Design (spec, plan, tasks) | merged | 63838fce6f95ea5195529109ba0e0b1073530135 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T027 | full | Remote over SSH daemon connects (add, start, Connected) | — | pending |
| M2 | T028–T038 | full | Remote projects, worktrees and sessions | — | pending |
| M3 | T039–T050 | full | Reconnect after a network drop | — | pending |
| M4 | T051–T055 | full | Local work unaffected, concurrent clients | — | pending |
| M5 | T056–T065 | full | Host picker, failure reasons, daemon management | — | pending |
| M6 | T066–T072 | full | Version mismatch explained | — | pending |
| M7 | T073–T081 | full | Replace a mismatched daemon with no sessions | — | pending |
| M8 | T082–T084 | full | Remote daemon artefact for macOS and Windows | — | pending |

## Decisions

- Clarify Q1: daemon delivered by the client copying its own matching build over SSH to a per-user dir _(decided by user)_.
- Clarify Q2: mismatched remote daemon is explained only; replacement offered only with no running sessions _(decided by user)_.
- Tasks: M1 kept whole at 27 tasks (Setup and Foundational are not observable alone; P1 story must be in M1); US1 split scenario 1 / 2-4 (M1/M2); US5 split M6/M7; Polish M8 changes build config, so it is a milestone.
- Tasks: speckit-analyze round 1 found 1 HIGH (shared picker widget) and 5 MEDIUM; all fixed in tasks.md, no plan change.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | ba77a079ad4169573eddc31e57ce119656d81c5f:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 4 MAJOR (reconnecting state, timeout value, backoff cap, session state while disconnected), all fixed |
| Spec | 2 | 0c21e1c00af41296ea2daf2c724c93e197d42c98:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 1 MAJOR (canonicalize_best_effort is a lexical walk, not verbatim: now a required change), 3 MINOR; fixed |
| Plan | 3 | dfcf4d63a3a9a5e42df4dfb44d48c741b07de2e9:e022a929bab01046ada386829ee3f6fa1e62e347 | CLEAN (1 MINOR fixed) |
| Plan | 1 | 9a5ff7ae39fe35fcffdb2f2c26b8dcbe452da2be:c33ad03cfb1fec6ccb1c1856d766226a886e57b6 | CHANGES: 3 MAJOR (local Unreachable vs FR-008; R5 audit list incomplete; R3 cross-OS matrix/glibc), 3 MINOR; all fixed |
| Plan | 2 | 8194452ff2f9a451e06c04598eebc2d0f00299a9:2c9c6abed776f4cdbdf710d33f6c9bea62dd5df1 | CLEAN (round 3 followed) |
| Tasks | 1 | f72a58cd6bb526d69c196ac5286ceb5e01e46526:7b87ef147273f34743f12f80b97367aea13812bc | CLEAN (3 MINOR fixed, prose/task wording; checklist ticked) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit M1 part 3 continues (implement, T001-T027). Commits on the branch, not pushed, no PR yet.

Done and compiled, tests green (`cargo test -p micold-core --lib` 570, `daemons_ssh` 5):
- T001-T003, T005-T009, T012-T016, T018 (ticked in tasks.md). `crates/micold-core/src/ssh/`:
  `target`, `argv`, `classify`, `stdio`, `provision` (`SshRunner`/`Artefacts`/`SshHost` traits,
  `ProvisionOptions`, `provision()`, `connect()` returning `Connected`, `parse_probe`,
  `select_artefact`); `DaemonRuntime::Ssh` + `DaemonEvent::StartFailed` in daemons.rs;
  `Transport::Ssh`, `DialAddress::Ssh`, `endpoint::resolve_pinned()`, `RemotePlacement{target}`.
- tdd/test-list.md and tdd/cycle-log.md written (test-after admitted, 5 mutants killed).

Next steps:
1. T010+T017 daemon helper: `crates/micold-daemon/src/remote.rs` `bridge` (use
   `endpoint::resolve_pinned`, dial via `connect::dial`, spawn detached daemon with
   `XDG_RUNTIME_DIR` removed + poll like `connect::connect_or_spawn` (SPAWN_TIMEOUT,
   RESPAWN_AFTER), relay stdin<->socket byte-for-byte; stdout only relayed bytes) and `prune
   --keep N`; dispatch in main.rs beside `status-line`, before runtime/singleton. `status`/`stop`
   are M7. Tests first (T010) against a temp endpoint.
2. T004+T022 harness + e2e: `crates/micold-daemon/tests/ssh_real_support/{mod,sshd}.rs` (rootless
   sshd on loopback, keys, authorized_keys, private config `Host` alias, skip with printed reason),
   `tests/ssh_real_endtoend.rs` part 1 (names start `ssh_real_`). Needs a `SystemSsh` runner
   usable from the test: put the real runner in micold-core `ssh/system.rs` (spawn `ssh`,
   `kill_on_drop`, deadline watchdog, chunked stdin with progress) if it is simplest, and have the
   client shell install it (plan said client/shell only; core placement is fine if
   `no_concrete_implementations.rs` stays green - re-run it).
3. T011, T019-T021, T023-T025 client: `Dial::Ssh{target}` in client/src/daemon.rs `actor_specs`
   (an `Ssh` entry whose state is Unreachable is not dialled; Start re-adds), actor calls
   `ssh::provision::connect`, on failure sends `StartFailed(reason)` and ends; copy progress ->
   `copying daemon to <host> (<n>%)`; Draft/Msg fields in features/daemons.rs; form + geometry
   case in ui/settings/daemons.rs; connected row shows version and `user@host:port`.
4. T026 CI job `ssh-remote` + docs/development/ci-pipeline.md; T027 docs/user-guide/remote-daemons.md,
   SUMMARY.md, settings.md, sandboxed-daemon.md links, docs/daemon.md.
5. Then tick tasks, scoped gate with review A (`code-review` high) and B (conformance), full
   `mise run gate`, PR `feat(489): remote over SSH daemon connects (#489)` per tasks/pr.md.

Tip: run cargo detached (`setsid nohup ... &`) and `scripts/autopilot/hold.sh <log>`; the hook
blocks foreground calls over 250 s.

## Open escalation


None.

## Follow-ups not done

- Issue #687 duplicates this feature's scope; close it with the final PR.
- Spec 491's tree shows SSH as out of scope; no change needed.
- Plan R3: macOS/Windows clients need a bundled `linux-x86_64` daemon to reach Linux hosts (adds daemon size to those packages); tasks sequence it last and separable. Owner to confirm packaging size.
