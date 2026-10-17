# Autopilot ledger — #806 container-daemon-ssh-provisioning

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 806 — issue #806 "Container daemon: configurable resources, Dockerfile or image source; build from current daemon binary by default", with the owner's scope change: "use basic image with ssh where you can copy daemon and install ai cli"; "should use same mechanism for local 'sandbox' container" (Dockerfile build dropped; shared SSH provisioning reused by #686). labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #806
- **Worktree branch**: claude/project-thread-uwrpqv
- **Started**: 2026-10-17
- **Phase**: M2
- **Next step**: M2 in progress: see Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #975 | design (spec, plan, tasks) | merged | edae87f2b49c0d1ab6315393fe395222c2809a8d |
| #979 | M1 basic image | merged | 110b724e0371648d2de53c5f5c033f57acad16b2 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | basic image published by `basic-image.yml` on merge (prints the `BASIC_IMAGE` pin) | #979 | merged |
| M2 | T010–T025, T087 | full | shared `micold_core::provision` (contract); 489 remote daemons start through it, row shows shared steps — #686 can plan onto it from here | — | in progress |
| M3 | T026–T031 | full | static Linux musl daemons (x86_64, aarch64) in every package; `NoMatchingBuild` | — | pending |
| M4 | T032–T041 | full | AI CLI install step + registry-only relay on app-owned targets; daemon offers only installed CLIs — #686 has the full contract | — | pending |
| M5 | T042–T070 | full | container daemons on the pinned basic image over SSH; two-entry image form; migration and legacy containers | — | pending |
| M6 | T071–T078 | full | per-daemon AI CLI selection; install warnings in the row | — | pending |
| M7 | T079–T086 | full | step bounds, cancel, rollback, concurrent starts; end-to-end suite in CI | — | pending |

Milestone notes:
- M1 is the basic image, not the P1 story (milestones rule 1): US1 needs `BASIC_IMAGE` pinned to a digest that only exists after `basic-image.yml` runs on `main`, so the image must merge first. The shared provisioning follows as M2 because #686 waits on it (unit scope).
- M5 (29 tasks) is not split: `ImageChoice` replaces `ImageSource` in stored settings, so the migration (US4), the two-entry form (US3, FR-003) and legacy adoption (FR-017) must land with the container switch; any split leaves `main` with a form offering the released image or a running legacy container replaced with its sessions.
- Order change vs plan *Delivery order*: AI CLI install (plan step 5) moved before the container target (step 4) so containers never lose their AI CLIs on `main`.
- M4 is observable only through its tests and by #686, which adopts the install step from it; no UI reaches an app-owned target until M5 (kept separate so #686 gets the full contract before the container switch).
- Phase 12 (T088–T089) changes no code: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Dockerfile build or copy-in over SSH? | Basic image with sshd; client copies its daemon build over SSH and installs selected AI CLIs on start; same mechanism for the local sandbox container and #686 | user | owner decision 2026-10-17 (issue #806 comment, project thread) |
| D2 | clarify r1 | FR-002: where does the basic image come from? | Project-published (Containerfile minus daemon/CLIs plus sshd, existing release image jobs), version-independent, client pins by digest | agent | packaging/sandbox/Containerfile, .github/workflows/release.yml#image, spec Overview |
| D3 | clarify r1 | FR-010: default AI CLI selection? | New daemon: Claude Code only; every pre-upgrade container daemon: all supported CLIs | agent | Containerfile FR-023a, spec US4/FR-016, AiCli `#[default] ClaudeCode`, SC-002 |
| D4 | clarify r2 | Default AI CLI not selected / install failed? | Existing rule: start nothing, substitute nothing, say so where chosen | agent | spec 026 FR-002/FR-004 s4, unavailable_default_says_so.rs, spec 027 FR-023b |
| D5 | tasks | Analyze S1: spec clarification said the basic image is built from `packaging/sandbox/Containerfile` by the release image jobs; research R1 / contract say `packaging/basic/` + own workflow | Spec clarification amended to match R1 and contracts/basic-image.md (D2 is superseded on that point) | agent | research.md R1, contracts/basic-image.md |
| D6 | M1 | How `basic-image.yml` pushes both arches before composing the index | Per-arch tags `<hash>-amd64`/`<hash>-arm64`, composed into `<hash>` by `imagetools create`, as `release.yml` `image`/`image-manifest` do (the contract's "by digest" read as "as release.yml"); the summary prints the index digest pin | agent | release.yml `image`, contracts/basic-image.md *Publication* |
| D7 | M1 | T003 test name | `sandbox_real_basic_image_has_no_daemon_or_cli`: CI's `sandbox_real_` filter matches function names, so the task's bare name would never run in CI | agent | ci.yml `sandbox-runtime` comment on the filter |
| D8 | M1 | `org.micold.basic.content` label | Passed as `--build-arg BASIC_CONTENT_HASH` by the workflow (`ARG` default `dev`): a file under `packaging/basic/` cannot carry the hash of its own directory | agent | contracts/basic-image.md *Content hash* |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 010544dfa6285dd3519917045f0d621c0e6d449e:70f5872667fb6e8e9126555f2573ce86e0a7bf80 | CHANGES: 3 MAJOR, 3 MINOR (all fixed; F4 fixed as `unreachable`, not `failed`) |
| Spec | 2 | 74cc41c423ab5d29a6175b050cf3e75df031fe9c:70f5872667fb6e8e9126555f2573ce86e0a7bf80 | CLEAN |
| Plan | 1 | d0b4a6a43fd58d9a107426429874df1d6bc2da64:234ba46aea57fbd8e8220579dce4517a2582febf | CHANGES: 1 MAJOR, 1 MINOR (both fixed: test-strategy rows for FR-013/014/022/024/026; repair_retired_namespace kept before into_choice) |
| Plan | 2 | 92cbdb5e2c5275bb243ed3a4d5096b1f4e785397:11a4dcc7ddbe26563fe2b317d7f0cabdf06d8bd0 | CLEAN |
| Tasks | 1 | 2c3cc526e672d823e5c3d3242c3e812f4328c67d:a093c67d80e868ef4684e22c934607c036f9850f | CHANGES: 2 MAJOR, 1 MINOR (all fixed: [P] on T033/T063/T064 dropped, M4/M5 parallel examples corrected, M4 observability note in ledger); round 2 below |
| Tasks | 2 | ae7101c78be879b2cde37d5455ed29eb6fd93300:2d525fd2157f2c7e851c1e8dd343a38d30d37344 | CLEAN (1 MINOR fixed: checklist item ticked after reviewer confirmed 0 markers, FR-002/FR-010 resolved) |
| M1 review A (code-review high) | 1 | HEAD 687b8f8d | 10 findings: F1–F5, F8, F9 fixed; F6, F7, F10 declined (see Declined review findings) |
| M1 review B (conformance, sonnet) | 1 | 4de41d6b382fa4417634887ea9108075302f1c02:d99547189d32d7da84a573ed1e106fc02ad026e7 | CLEAN, 4 MINOR (F2, F3 fixed, prose only; F1 declined; F4 covered by the gate) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| design | Spec r1 | F4 fix proposal: a new `failed` state | 489 FR-008 already defines `unreachable` as "a start that failed"; the spec now uses `unreachable` consistently instead of adding a state |
| M1 | Review A r1 | F6 README.md is inside the content hash | contracts/basic-image.md *Content hash* defines the hash over `find packaging/basic -type f`; the README describes the image, so a change to it may republish |
| M1 | Review A r1 | F7 `find` hashes the working tree, not the git index | Same contract definition; CI hashes a clean checkout, so the working tree is the committed tree there |
| M1 | Review A r1 | F10 `is_released_image` has no caller | By design: M5 (T046+) calls it; the function is local and parse-based |
| M1 | Review B r1 | F1 (MINOR) `hash` job fails on DRIFT | Intended: the workflow publishes under the tree's hash, so `BASIC_IMAGE_CONTENT_HASH` must match it in the same change; the later "pin PR" is the digest pin `BASIC_IMAGE` (T049) |

## Handover

M2 unit 1 stopped at ~125k context (branch reset to origin/main, WIP committed locally, NOT pushed, no PR). Done:
- `crates/micold-core/src/ssh/argv.rs`: `Reach`, `PinnedReach`, `reach_check`, `install(reach,platform,timeout,relay_port)`, `record_last(reach,platform,timeout,build)`; `probe/copy/bridge` now take `&Reach`; tests T010 written (T017 core done; callers NOT yet moved: `ssh/provision.rs` still passes `&SshTarget`).
- New files, written but NOT yet declared in `lib.rs` and never compiled: `provision/attempt.rs` (CancelToken + tests, T014 part 1/T020 part 1), `provision/failure.rs` (Step, FailureCause, ProvisionFailure::reason(&ReasonContext), ProvisionWarning + tests, T012/T019), `provision/keys.rs` (DaemonKeys, keygen_argv, ensure(dir, platform, &dyn SshRunner), write_known_hosts + tests, T011/T018).
Next (in order):
1. `pub mod provision;` in lib.rs; create `provision/mod.rs`: `pub mod attempt, failure, keys`; `HostPort{host,port}`, `Egress`, `AiCliTable` (empty placeholder struct), `Ownership::{AppOwned{ai_clis,table,state_dir,egress},UserHost}`, `StepBounds{acquire,create_container,reach,probe,copy:Option,install,start,version}` with `container()` / `user_host(Duration)`, `ProvisionTarget{reach,owned,bounds}`, `ProvisionEvent{Entered,CopyPercent(u8),InstallLine,InstallDone}`, `EventSink=Arc<dyn Fn(ProvisionEvent)+Send+Sync>`, `Provisioned{connected,warnings,copied}`, `ensure_build(host,target,opts,cancel,sink)->Result<(Probe,copied),ProvisionFailure>` (Reach, Probe, Copy; InstallAiClis skipped until M4 T035) and `run(...)` = ensure_build + Start + VersionCheck. Add `AiCliSelection(BTreeSet<AiCli>)` (`new_daemon()`={ClaudeCode}, `all()`=AiCli::ALL, serde) in `sandbox/mod.rs` (not yet in SandboxProfile; T077).
   Design decisions made: UserHost skips the `true` reach command (the probe is its first connection; 489 tests script only the probe) but still emits `Entered(Reach)`; Reach retries every 500 ms on stderr containing "Connection refused" / "Connection reset" / "kex_exchange_identification" until `bounds.reach`, then NoSshServer; AppOwned + AuthRefused -> `FailureCause::AuthRefused`, UserHost stays `Ssh(AuthRefused)`; each step runs under a guard (select on `cancel.cancelled()` and `tokio::time::timeout(bound)` -> `TimedOut{bound}` / `Cancelled`; dropping the future kills the ssh child via kill_on_drop); runner `timed_out` -> `TimedOut{bound}`; copy failure classified `Other` -> `CopyFailed`, known class -> `Ssh(class)`; post-copy re-probe without `have` -> CopyFailed; no artefact -> `NoMatchingBuild{os,arch}` at Probe; `!home_writable` -> `Ssh(RemoteNotWritable)`; Start: AppOwned runs `argv::record_last` first, then `host.open(bridge)` + `handshake_with` under `bounds.start`; bridge death -> UserHost `Ssh(HelperFailed)`, AppOwned `DaemonStart` (+output); `Connected::Refused` -> `Entered(VersionCheck)` and `Ok(Provisioned{connected: Refused})` (the client keeps its 489 banner/restart flow); add `FailureCause::from_refusal(&RefusalReason)->Option<FailureCause>` (VersionMismatch{found: daemon,expected: client}) for runtimes (M5). Tests: fake host keyed by command kind (true / sh -s with MICOLD_PROBE in stdin / `cat >` / `printf` / `remote bridge`); fake daemon for the handshake: `Framed::new(far, DaemonCodec::new())`, read Hello, send `Frame::Control(DaemonMsg::Refused{reason})`. T013, T087 live in `provision/mod.rs` tests.
2. `ssh/provision.rs` T022: `provision()` = `ensure_build` wrapper, `connect()` = `run` wrapper with `UserHost` target, `ProvisionOptions` kept; map ProvisionFailure back to SshFailure (Ssh(f)->f, TimedOut->Timeout, CopyFailed->RemoteNotWritable, NoMatchingBuild->SshFailure::NoMatchingBuild, DaemonStart->HelperFailed{last output line}); existing 489 tests there stay unchanged (T015 test: UserHost wrapper uses bounds reach/probe/start = timeout_secs, copy None). Callers of the argv builders: `grep -n "argv::" crates -r`.
3. T014 second half: `SystemSsh` gets `cancel: Option<CancelToken>` (client constructor `shell/daemons.rs::system_ssh` and `crates/micold-daemon/tests/ssh_real_support/remote.rs:38` build it by struct literal: add the field there) and kills the child on cancel; add `cancelled: bool` is NOT needed if `run` returns `Err(io::ErrorKind::Interrupted)` on cancel; test with `sleep` child cfg(unix) and `timeout` on Windows.
4. Client T016/T023/T024: replace `ConnectionMsg::CopyProgress(u8)` with `ConnectionMsg::Provisioning(ProvisionEvent)` (`features/connection.rs:238`, `shell/connection.rs:46`, `daemon.rs:761 copy_progress` and `ssh_attempt` -> use `provision::run` + `ProvisionFailure::reason(ctx)`, `features/daemons.rs:1030/1171` `Msg::CopyProgress`; tests `crates/micold-client/tests/daemons_feature.rs:276`, `daemons_section_geometry.rs:267`); per-daemon current step + percent; failure sets `unreachable` to `reason()` and appends to recent errors; row draws step label + percent via `progress.rs` in `ui/settings/daemons.rs`.
5. T025 `docs/user-guide/remote-daemons.md`; update `contracts/provisioning.md` (UserHost Reach note; `reason(&ReasonContext)`; VersionCheck returns Ok(Refused)); tick T010-T025, T087 in tasks.md.
6. Then verify.md / review / gate (`scripts/gate.sh`, detached; free disk first, df showed 21 GB free at start) / PR files (`$SCRATCHPAD/pr-title` = `#806: Container daemon: configurable resources, Dockerfile or image source; build from current daemon binary by default — M2: shared SSH provisioning (T010–T025, T087)`, `pr-body.md` per pr.md), push `--force-with-lease`.

## Open escalation

None.

## Follow-ups not done

- specs/686-kubernetes-daemon-runtime FR-004, FR-012 and its clarifications still describe #806 as a Dockerfile build pushed to a registry; the owner's decision replaces that with SSH provisioning (806 FR-008). That spec belongs to the #686 flow.
- M5 (T051, `SandboxSpec` argv): the basic image makes `/etc/passwd`/`/etc/group` writable by gid 0 only (contract); a container run as `--user uid:gid` with the host gid cannot append its passwd line unless the create argv also adds group 0 (`--group-add 0`, podman `--userns=keep-id` gives an entry itself). The entrypoint warns instead of failing. Check when M5 writes the argv.
- M1 `sandbox_real_basic_image` was not run green locally (this environment's egress refuses `deb.debian.org`, so `micold-basic:dev` cannot be built here); its red was shown against `node:24-trixie-slim`. CI's `sandbox-runtime` job runs it.
- The `BASIC_IMAGE` digest pin and CI's pin check are T049 (M5): the digest exists only after `basic-image.yml` has run on `main` after this PR merges.
