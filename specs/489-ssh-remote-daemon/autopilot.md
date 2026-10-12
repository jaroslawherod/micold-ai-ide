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

Unit M1 (implement, T001-T027) stopped at the 150k context cap. WIP commit on the branch, not pushed,
no PR yet. Gate/review/PR still to do after implement.

Done (uncommitted-to-gate, nothing compiled yet; tasks.md boxes NOT ticked):
- T001 `process` feature on workspace tokio; T002 `ssh-real-runtime` feature (core + daemon) and
  `mise run test-ssh` task (mise.toml).
- Written, never compiled, in `crates/micold-core/src/ssh/`: `mod.rs` (declares argv, classify,
  provision, stdio, target; re-exports `ClientPlatform, SshCommand, ReasonContext, SshFailure,
  Artefacts, ProvisionProgress, SshHost, SshOutput, SshRunner, SshStdio, SshSettings, SshTarget`),
  `target.rs` (T005+T012, with tests), `argv.rs` (T006+T013, with tests), `classify.rs`
  (T007+T014, with tests), `stdio.rs` (SshStdio from_parts/from_child, StderrTail, with a test).
  Tests were written WITH the code (test-after for these pure modules): run a deliberate-mutant
  check on a couple before claiming red, and say so in tdd/cycle-log.md. `lib.rs` has NOT yet got
  `pub mod ssh;`. `mod.rs` re-exports names from `provision` that do not exist yet.

Next steps (design decided, see below):
1. `ssh/provision.rs` (T009+T016): traits `SshRunner` (`run(cmd, stdin: Vec<u8>, deadline,
   progress) -> BoxFuture<io::Result<SshOutput{status, stdout, stderr, timed_out}>>`, `open(cmd) ->
   io::Result<SshStdio>`), `Artefacts` (own daemon bytes; bundled `<os>-<arch>` bytes), `SshHost =
   SshRunner + Artefacts`; `parse_probe` of `MICOLD_PROBE os= arch= home_writable= have= dir=`
   (os `Linux`->linux, `Darwin`->macos; arch x86_64, aarch64|arm64); pure `select_artefact(client
   platform+arch, remote os/arch)` (own when equal, else bundled, else `NoMatchingBuild`); sequence
   probe -> copy only if `have=0` (progress %) -> re-probe -> `open(bridge)`; `connect()` returns
   `Connected` via `connect::handshake_with(Transport::Ssh(stdio), ..)` mapping errors with
   `StderrTail` to `HelperFailed`. Fixed 15 s deadline (`DEFAULT_CONNECT_TIMEOUT_SECS`); ConnectTimeout
   from `ssh -G` is M5. Tests over a fake `SshRunner` (duplex stand-in for the remote end).
2. Core wiring (T015, T018): `pub mod ssh;` in lib.rs; `DaemonRuntime::Ssh(SshSettings)` in
   daemons.rs (label "Remote over SSH", startable; custom Serialize/Deserialize there: kind "ssh",
   flat host/user/port/key_file; bad entry -> `Unsupported`); registry `check` validates the target
   and returns `InvalidField`; add `DaemonEvent::StartFailed(String)` (Starting -> Unreachable at
   once; ssh runtimes make one attempt) ; settings.rs round trip with no `settings_version` bump;
   `Transport::Ssh(SshStdio)` in connect.rs (delegate! macro + dial arms), `DialAddress::Ssh(SshTarget)`
   in endpoint.rs (`describe`, `socket_addr` None), `RemotePlacement` carries `SshTarget` (currently
   private `host: String`, `#[non_exhaustive]`; used in logout_survival.rs:232 test),
   `Placement::from_runtime(Ssh)` -> `Remote`, `connect_or_start` Remote arm unchanged (error).
   `endpoint.rs` needs a pinned `resolve` for the helper: `/tmp/micold-<uid>` ignoring
   XDG_RUNTIME_DIR (research R2); the bridge spawns the daemon with `XDG_RUNTIME_DIR` removed so it
   binds the same endpoint (macOS is HOME-based already).
3. Daemon helper (T010, T017): `crates/micold-daemon/src/remote.rs` `bridge` (resolve pinned endpoint,
   dial, spawn detached daemon + poll like `connect::connect_or_spawn` (SPAWN_TIMEOUT, RESPAWN_AFTER),
   relay stdin<->socket byte-for-byte, stdout only relayed bytes) and `prune --keep N`; dispatch in
   main.rs beside `status-line` (before the runtime/singleton). `status`/`stop` are M7, not M1.
4. Harness + e2e (T004, T022): `crates/micold-daemon/tests/ssh_real_support/{mod,sshd}.rs` (rootless
   sshd on loopback, host+client keys, authorized_keys, private ssh config `Host` alias, skip with a
   printed reason when sshd missing), `tests/ssh_real_endtoend.rs` part 1 (names must start `ssh_real_`).
5. Client (T011, T019, T020, T021, T023-T025): `Dial::Ssh{target}` in client/src/daemon.rs
   `actor_specs` (exclude an `Ssh` entry whose state is Unreachable so no retry; Start re-adds);
   actor calls a runner installed with `daemon::set_ssh_host(Arc<dyn SshHost>)` (a replaceable
   global; the real `SystemSsh` + artefact reader is defined and installed only in
   client/src/shell/ (daemons.rs), Windows `ssh.exe` lookup (PATH, then
   `%SystemRoot%\System32\OpenSSH\ssh.exe`), CREATE_NO_WINDOW; not a port in inventory PORTS so the
   no_concrete_implementations guard is unaffected, re-run it); on failure send
   `ConnectionMsg::StartFailed(reason)` and end the stream; copy progress as
   `ConnectionMsg::Progress(u8)` -> shown as `copying daemon to <host> (<n>%)`; Draft/Msg SSH fields
   in features/daemons.rs; form + geometry case in ui/settings/daemons.rs; connected row shows
   version and `user@host:port` (DaemonView.endpoint).
6. T026 CI job `ssh-remote` (.github/workflows/ci.yml) + docs/development/ci-pipeline.md; T027
   docs/user-guide/remote-daemons.md, SUMMARY.md, settings.md, sandboxed-daemon.md links, docs/daemon.md.
7. Then: tdd/cycle-log.md (no tdd/test-list.md exists for 489: tasks.md already orders tests first; the
   mandatory tdd hook could not run `speckit.tdd.plan`; note it), tick T001-T027, scoped gate with
   review A (`code-review` high) in its shadow, review B (conformance), full `mise run gate`, PR
   `feat(489): remote over SSH daemon connects (#489)` with body per tasks/pr.md.

## Open escalation


None.

## Follow-ups not done

- Issue #687 duplicates this feature's scope; close it with the final PR.
- Spec 491's tree shows SSH as out of scope; no change needed.
- Plan R3: macOS/Windows clients need a bundled `linux-x86_64` daemon to reach Linux hosts (adds daemon size to those packages); tasks sequence it last and separable. Owner to confirm packaging size.
