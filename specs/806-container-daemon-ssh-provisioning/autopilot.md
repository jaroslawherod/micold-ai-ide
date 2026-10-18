# Autopilot ledger — #806 container-daemon-ssh-provisioning

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 806 — issue #806 "Container daemon: configurable resources, Dockerfile or image source; build from current daemon binary by default", with the owner's scope change: "use basic image with ssh where you can copy daemon and install ai cli"; "should use same mechanism for local 'sandbox' container" (Dockerfile build dropped; shared SSH provisioning reused by #686). labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #806
- **Worktree branch**: claude/project-thread-uwrpqv
- **Started**: 2026-10-17
- **Phase**: M5b
- **Next step**: M5b part 1 shipped (PR files in the scratchpad); then M5b part 2 (*Handover*)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #975 | design (spec, plan, tasks) | merged | edae87f2b49c0d1ab6315393fe395222c2809a8d |
| #979 | M1 basic image | merged | 110b724e0371648d2de53c5f5c033f57acad16b2 |
| #982 | M2 shared provisioning | merged (Windows job failed once on an unrelated timing test, micold-daemon history_stop_request ten_busy_sessions_are_all_saved_within_the_bound; passed on re-run) | 198a12312b8b801dce177e47998024902732f0ac |
| #988 | M3 static Linux daemons (two CI fixes: SIGPIPE in remote-daemon-bundle.test.sh; zig crashed on the Windows ARM64 runner, so musl daemons are built once in the Linux `linux-daemons` job and downloaded by packaging jobs) | merged | 32798973fe5df793f5c85b6ed52a84e52230591d |
| #995 | M4 AI CLI install step + registry-only relay | merged | abe058a2424ea0551edb2f6667ac732064434557 |
| #998 | M5 subset (container daemons over SSH on the basic image; pulled in T047, T055, T065) | merged | d526eaca82b43d60d5f2eb85926259f0fefc45e4 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | basic image published by `basic-image.yml` on merge (prints the `BASIC_IMAGE` pin) | #979 | merged |
| M2 | T010–T025, T087 | full | shared `micold_core::provision` (contract); 489 remote daemons start through it, row shows shared steps — #686 can plan onto it from here | #982 | merged |
| M3 | T026–T031 | full | static Linux musl daemons (x86_64, aarch64) in every package; `NoMatchingBuild` | #988 | merged |
| M4 | T032–T041 | full | AI CLI install step + registry-only relay on app-owned targets; daemon offers only installed CLIs — #686 has the full contract | #995 | merged |
| M5 | T042–T044, T049–T054, T057, T060, T061 | full | container daemons on the pinned basic image over SSH (`Dial::ContainerSsh`), two-entry image form, migration read-repair, legacy adoption; shipped as a subset (orchestrator limit on parts) | (opened by the orchestrator) | gate green at c43d8040 |
| M5b | T045, T046, T048, T056, T058, T059, T062–T064, T066–T070 (T047, T055, T065 done in #998) | full | the rest of M5: notices and gated legacy restart, tests, real-runtime suites on the basic image, CI pin check, user guide, visual pass | — | pending (new milestone) |
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

- M5 CI-fix (real-runtime suites, diff 419f0bf1..cf61def3, `autopilot-reviewer`) round 1: 1 MAJOR fixed (`sandbox_real_idle` asserted RestartCount 1 after a manual `docker restart`; it counts only policy restarts: now 0). MINOR fixed: the handshake state test now writes from inside the container. MINOR deferred (real-runtime suites, unrunnable here): staleness survival test also passes via the bridge's daemon start; fingerprint test checks only the version substring; limits create-time flags hand-written; harness `docker create` hand-built instead of `argv::create`; host uid 0 cannot log in (CI is non-root); host-built daemon vs container glibc.
- M3 review A round 1 (`b353fa8c1c76:198a12312b8b`): CHANGES; F1 MAJOR (`zig` not on PATH after `pip install ziglang`) fixed: the script accepts `python3 -m ziglang`; F2 MINOR (python3 on Windows) fixed. Round 2 (sonnet, scoped): CLEAN.
- M3 review B round 1 (`snapB`, sonnet): CHANGES; F1 MAJOR (no M3 cycle-log rows) fixed; F2, F3 MINOR fixed. Round 2 (sonnet, scoped): CLEAN, 1 MINOR (the T029 red claim, corrected to "none recorded"). Verify green.
- M4 review A round 1 (`8df8fcd612e8:8646b077dd62`): CHANGES; F1 MAJOR (offered.json not refreshed when nothing is missing) fixed: the check script rewrites it; F2 MAJOR (`-R` without `ExitOnForwardFailure`) fixed; F3, F5 MINOR fixed (`</dev/null` for npm; relay upstream connect timeout, accept-error backoff); F4 declined below. Fixes verified by `provision::` and `ssh::` tests.
- M4 review B round 1 (`snapB`, sonnet): CHANGES; F1 MAJOR (no red evidence for M4 behaviours) fixed: reds recorded by mutation in the cycle-log; F2 MINOR declined (below). Verify green (`provision::` 59, `state::` 4). Round 2 (sonnet, scoped): CLEAN, 1 MINOR (a count in the cycle-log, 7 → 6) fixed, prose only.

- M2 review B (sonnet, `fb2179e9334e:2bd42e9facfa`): CHANGES; F1 MAJOR (no M2 cycle log) fixed; F2 MINOR (Windows twin of cancel test) declined: module is Unix-only like its siblings. Verify green. Docs-only fix: no re-run.

- M2 review A round 1 (`9cace2fa233d:d1333e94`): CLEAN, 3 MINOR; F2 (record_last failure -> DaemonStart) and F3 (late Provisioning event ignored unless Starting) fixed; F1 declined below.

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

- M2 review A F1 (MINOR, `SystemSsh.cancel` on the shared host): kept, T020 requires the field; `SSH_HOST` stays token-less (`cancel: None`) and the per-attempt token cancels through `guard`. M7 builds a per-attempt `SystemSsh` clone.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| design | Spec r1 | F4 fix proposal: a new `failed` state | 489 FR-008 already defines `unreachable` as "a start that failed"; the spec now uses `unreachable` consistently instead of adding a state |
| M1 | Review A r1 | F6 README.md is inside the content hash | contracts/basic-image.md *Content hash* defines the hash over `find packaging/basic -type f`; the README describes the image, so a change to it may republish |
| M1 | Review A r1 | F7 `find` hashes the working tree, not the git index | Same contract definition; CI hashes a clean checkout, so the working tree is the committed tree there |
| M1 | Review A r1 | F10 `is_released_image` has no caller | By design: M5 (T046+) calls it; the function is local and parse-based |
| M1 | Review B r1 | F1 (MINOR) `hash` job fails on DRIFT | Intended: the workflow publishes under the tree's hash, so `BASIC_IMAGE_CONTENT_HASH` must match it in the same change; the later "pin PR" is the digest pin `BASIC_IMAGE` (T049) |
| M4 | Review A r1 | F4 (MINOR) relay bind failure should be reported as the io error | contracts/provisioning.md *Failure causes* and T034 define `PostureBlocksDownload` for a relay that cannot be opened; the cause text covers it |
| M4 | Review B r1 | F2 (MINOR) `standin_table` has no automated test | No suite uses it before M5/M7 (the container suites); packing and installing both variants was run by hand and is recorded in the cycle-log; a test now would be a container-free test of test support |

## Handover

M5b part 1 is committed on the branch (PR files in the scratchpad). **Done**: T049 re-pin (`BASIC_IMAGE` = tag `6009b58e0dd70464`, index digest `sha256:ab15041b…` from `basic-image.yml` run 39128313716 on `d526eaca`, read from ghcr's anonymous manifest endpoint); the bridge-started container daemon now gets `MICOLD_IMAGE_REFERENCE`, `TZ`, `MICOLD_IDLE_STOP=off` (survive_logout) and `XDG_DATA_HOME` through `ProvisionTarget.env` (`container_env` in `daemon.rs`; `Dial::ContainerSsh` carries `image` and `survive_logout`, `daemon::install_time_zone` is set at startup), which also closes follow-up (a) of the survive-logout idle env; T056 (`settings_released_image_migration.rs`); T058 + T062 (`DaemonView.notice`/`stop_blocked`, `Model.legacy_container`, `settings.legacy_container` set by `shell/sandbox.rs` after each sandbox message, `DaemonRegistry::clear_migration` after the first non-legacy `Started` on Basic); T069 (user guide "Which image it runs on").

**Next step (fresh unit, M5b part 2)**: T045 (`prepare_keys`/`container_attempt` tests with a fake `SshHost`; the `Entered(Acquire)`/`Entered(CreateContainer)` events are not emitted by the actor), T046 geometry gate, T063, T064, T066, T068 (two-entry `Select`, archive picker), the real-runtime suites T048/T059/T067 (compile only here; CI's sandbox job runs them), T070 visual pass, and follow-up items (6) `strict_fingerprint` dead code, (7) "getting the image" row text. The history/idle real-runtime suites may now drop their `provision_then_restart` workaround (the bridge-started daemon has the container env): leave it until CI shows them green. The UI of the notices is not visually checked (T070).

## Open escalation

None.

## Follow-ups not done

- specs/686-kubernetes-daemon-runtime FR-004, FR-012 and its clarifications still describe #806 as a Dockerfile build pushed to a registry; the owner's decision replaces that with SSH provisioning (806 FR-008). That spec belongs to the #686 flow.
- M5 (T051, `SandboxSpec` argv): the basic image makes `/etc/passwd`/`/etc/group` writable by gid 0 only (contract); a container run as `--user uid:gid` with the host gid cannot append its passwd line unless the create argv also adds group 0 (`--group-add 0`, podman `--userns=keep-id` gives an entry itself). The entrypoint warns instead of failing. Check when M5 writes the argv.
- M1 `sandbox_real_basic_image` was not run green locally (this environment's egress refuses `deb.debian.org`, so `micold-basic:dev` cannot be built here); its red was shown against `node:24-trixie-slim`. CI's `sandbox-runtime` job runs it.
- The `BASIC_IMAGE` digest pin and CI's pin check are T049 (M5): the digest exists only after `basic-image.yml` has run on `main` after this PR merges.
- M4: `install_step` does nothing for an empty selection (no `offered.json` is written, so a daemon whose selection was emptied after an install keeps offering the old file's CLIs). The per-daemon selection (M6, T071+) owns that case: it must write the file for an empty selection too.
- **M5b (new milestone, tasks as listed in the Milestones table)**: what part 4 left, in order. (1) `DaemonView.notice` and the legacy notice "Still running on the released daemon image; restart it with no sessions running to move it to the basic image." plus the migration notice (`MigrationNotice::message`) in `ui/settings/daemons.rs` (T058/T062); `features::daemons::Model` needs `legacy_container: Option<DaemonId>` (from `app.sandbox.legacy` and `shell/routing.rs::sandbox_daemon`), and Stop disabled while `model.sessions[id]` is non-empty for a legacy container; clear `ContainerSettings.migration` and save after the first `Started` on Basic in `shell/sandbox.rs::apply`. (2) T045 `prepare_keys` / `container_attempt` tests with a fake `SshHost`; the `Entered(Acquire)`/`Entered(CreateContainer)` events are not emitted by the actor today (the shell's bring-up has its own progress). (3) T056 migration table test; T063/T064/T066 tests; T046 geometry gate; T068 archive picker (the archive is still a text field). (4) Real-runtime suites T048/T059/T067 still to do (T047/T055/T065 were pulled into PR #998 and converted onto `micold-basic:dev`, compiled but never run here: CI's sandbox job verifies). Follow-ups from that conversion: (a) the survive-logout idle env (`MICOLD_IDLE_STOP=off`) set at `create` reaches only the daemon the entrypoint starts, not one started by the SSH bridge (sshd does not pass the container env): FR-022 holds for a restart-policy restart only; (b) `sandbox_real_ai_cli_sessions` shared run is skipped (the basic image has no real claude); (c) `sandbox_real_idle` §5.17 now measures the daemon, not the container. (5) T069 `docs/user-guide/sandboxed-daemon.md`; T070 visual pass. (6) `strict_fingerprint` is dead (always false) in `Placement`/`ActorSpec`/`actor_specs`: remove it. (7) The actor waits for the container with `exec id -un` for up to 16 minutes (acquire + create bound) without emitting a step event; show "getting the image" in the row.
- Review A r1 (M5 subset): F1 MAJOR fixed (`SettingsDraft::validate` ignored an invalid image; now a `FieldError` on `DaemonImage`; its test belongs to M5b T066). Declined/deferred MINOR: F2 (`prepare_keys` without an installed `ssh` host: the shell always installs one, the no-host case is the test process), F3 (`Sandbox.legacy` not cleared on stop/failure: the next `Started` resets it; M5b), F4 (`moved_off_released` through `sync_legacy`: M5b with T056).
- Red CI #998, part 8 (history/mcp suites): `sandbox_real_history` (6) and `sandbox_real_mcp` failed on the basic image. Causes: (1) a daemon the client starts over SSH does not inherit the container's `-e` env (`MICOLD_IMAGE_REFERENCE`, `TZ`), so the history suite now provisions then restarts the container (shared `provision_then_restart` in `sandbox_real_support`, as `sandbox_real_idle` does) so the entrypoint's daemon runs with it; (2) `docker stop` signals only PID 1 (sshd, `exec`'d), so the daemon was killed unsaved: `packaging/basic/entrypoint.sh` now runs sshd as a child and on SIGTERM/SIGINT stops every daemon (up to 10 s, the runtime's default stop grace) then sshd (contract + README updated; simulated with stub scripts, the real image is verified by CI); (3) `sandbox_real_mcp` compared `<runtime> port` output to the host port; it names the container port 2222. Follow-up (product): a bridge-started daemon lacks the container env (`MICOLD_IMAGE_REFERENCE`, `TZ`); only an entrypoint-started one has it. `cargo test --no-run`/clippy with `sandbox-real-runtime` clean; suites not run here.
- Part 8: `BASIC_IMAGE_CONTENT_HASH` moved to the new tree hash (the entrypoint changed); `BASIC_IMAGE` (tag + digest) still names the last published image until `basic-image.yml` publishes this tree on `main` (T049 re-pins it).
