# Autopilot ledger — #806 container-daemon-ssh-provisioning

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 806 — issue #806 "Container daemon: configurable resources, Dockerfile or image source; build from current daemon binary by default", with the owner's scope change: "use basic image with ssh where you can copy daemon and install ai cli"; "should use same mechanism for local 'sandbox' container" (Dockerfile build dropped; shared SSH provisioning reused by #686). labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #806
- **Worktree branch**: claude/project-thread-uwrpqv
- **Started**: 2026-10-17
- **Phase**: M5
- **Next step**: M5 in progress (handover 1): continue from *Handover*

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #975 | design (spec, plan, tasks) | merged | edae87f2b49c0d1ab6315393fe395222c2809a8d |
| #979 | M1 basic image | merged | 110b724e0371648d2de53c5f5c033f57acad16b2 |
| #982 | M2 shared provisioning | merged (Windows job failed once on an unrelated timing test, micold-daemon history_stop_request ten_busy_sessions_are_all_saved_within_the_bound; passed on re-run) | 198a12312b8b801dce177e47998024902732f0ac |
| #988 | M3 static Linux daemons (two CI fixes: SIGPIPE in remote-daemon-bundle.test.sh; zig crashed on the Windows ARM64 runner, so musl daemons are built once in the Linux `linux-daemons` job and downloaded by packaging jobs) | merged | 32798973fe5df793f5c85b6ed52a84e52230591d |
| #995 | M4 AI CLI install step + registry-only relay | merged | abe058a2424ea0551edb2f6667ac732064434557 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | basic image published by `basic-image.yml` on merge (prints the `BASIC_IMAGE` pin) | #979 | merged |
| M2 | T010–T025, T087 | full | shared `micold_core::provision` (contract); 489 remote daemons start through it, row shows shared steps — #686 can plan onto it from here | #982 | merged |
| M3 | T026–T031 | full | static Linux musl daemons (x86_64, aarch64) in every package; `NoMatchingBuild` | #988 | merged |
| M4 | T032–T041 | full | AI CLI install step + registry-only relay on app-owned targets; daemon offers only installed CLIs — #686 has the full contract | #995 | merged |
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

M5 unit 1 (stopped at ~150k context after reading code; only T049 is edited, uncommitted work is committed with this ledger). Branch was reset to origin/main (abe058a2); `branch-start.sh` refuses here (gh GraphQL unauthenticated), so use `git fetch origin main && git checkout -B claude/project-thread-uwrpqv origin/main` only if the branch is not yet ahead of main with M5 commits.

**Done**: T049 `BASIC_IMAGE` added to `crates/micold-core/src/sandbox/basic_image.rs`: `ghcr.io/jaroslawherod/micold-basic:722a7d9ef0d6aadb@sha256:bea51dd1e190fe1ab4c34129c5784eb3e6f2f65ac9b0ac63b6fff1c06d608366` (digest read from ghcr.io registry for the tag that `basic-image.yml` run 39078607729 published on main at 110b724e; `scripts/basic-image-hash.sh --check` prints CURRENT). The `ci.yml` pin check (T055) is still to add. Tick T049 in tasks.md.

**Next**: T042–T048, T050–T070 in dependency order: image.rs (`ImageChoice`, `OwnImage`, `OwnImageDraft`, `ImageSource::into_choice`) → `SandboxProfile.image: ImageChoice` → `SandboxSpec`/argv → lifecycle `Adoption::Legacy` → migration (daemons.rs, settings.rs) → client (daemon.rs `Dial::ContainerSsh`, shell/sandbox.rs, container_fields.rs, features/daemons.rs, ui/settings/daemons.rs) → real-runtime suites, mise.toml, ci.yml, docs, visual pass.

**Design notes found while reading (saves re-reading)**:
- ~30 files use `ImageSource`/`profile.image` (list: `grep -rn 'ImageSource\|profile\.image\|\.image\.' crates`): core `sandbox/{argv,cli,runtime,lifecycle,mod,parse}.rs`, `settings.rs:851`, `daemons.rs:276,659`, client `features/{daemons,settings}.rs`, `shell/{startup:278,daemon_sync:875,persist:374}.rs`, `ui/settings/container_fields.rs`, many tests under `crates/*/tests/` (`sandbox_*`, `settings_retired_image_namespace`, `protocol_auth`, `daemons_*`).
- Chosen serde design: `SandboxProfile` derives `Serialize` (field `image: ImageChoice` renamed `image_choice`) and deserialises through a wire struct that accepts old `image` (`ImageSource`) and `image_choice`; `image_choice` wins. A `#[serde(skip)] moved_off_released: bool` on the profile carries the "released image" fact to `ContainerSettings`' deserialiser / `migrated_entry`, which turn it into `migration: Some(MovedOffReleasedImage)` and reset it. `ImageChoice` JSON: `{"kind":"basic"}` / `{"kind":"own","reference":..}` / `{"kind":"own","archive":..}`; own with neither → Basic. `ImageSource::into_choice` rules are the table in contracts/daemon-settings.md (released repo via `is_released_image` → Basic + moved; other registry ref → Own(Reference); imported_file with path → Own(Archive); local_build → Own(Reference)). Keep `repair_retired_namespace` and its unit tests; T057's test asserts via a loaded settings file.
- `acquire_image` (`sandbox/cli.rs:169`, `runtime.rs:255`, `lifecycle.rs acquire`) takes `&ImageSource` today: change to `&ImageChoice`; Basic → pull `BASIC_IMAGE`; `Own(Reference)` → inspect-then-pull (a local `micold-basic:dev` must work; `LocalBuild` no longer exists); `Own(Archive)` → `load -i`. Keep the 15-minute acquire bound for M7, not here.
- `adopt(existing, expected_image, expected_fingerprint, strict)` (`lifecycle.rs:217`) → drop the fingerprint args and `StaleReason::StaleBuild`'s production; add `Adoption::Legacy(id)` for a *running* container whose image repository `is_released_image` (stopped → `Replace`). `Started` needs a `legacy: bool` (or the adoption) so the client picks `Dial::Container`. `bring_up` caller: `shell/sandbox.rs:383`.
- `argv::create` (`sandbox/argv.rs:99`): publish `127.0.0.1:{port}:2222`; drop token `-v` (`mount_args` line ~224) and `MICOLD_IMAGE_REFERENCE` stays (use `profile.image.reference()`); add `-e MICOLD_HOME=<container home>`; add `--group-add 0` for the Docker dialect (entrypoint needs gid 0 to append /etc/passwd; podman keep-id needs none); mount **three single files** read-only: `<keys>/host_ed25519` → `/run/micold/ssh/host_ed25519_key`, `host_ed25519.pub` → `/run/micold/ssh/host_ed25519_key.pub`, `authorized_keys` → `/run/micold/ssh/authorized_keys` (sshd_config in the image names `host_ed25519_key`; the task text says `host_ed25519` — the file names on the host are those of `DaemonKeys`). Replace `MountSet.secret: SecretMount` with an `SshMount`; `CONTAINER_TOKEN_PATH`/`host_token_path` stay for the legacy path (`shell/sandbox.rs:282-291` still writes a token only for legacy).
- Client flow today: `shell/sandbox.rs::start` (sync, blocking, bring-up) → actors dial (`daemon.rs actor_specs` line ~216 builds `Dial::Container{port,token_path}`). `Dial::ContainerSsh` is built in `actor_specs` for non-legacy container entries: needs `PinnedReach{host:"127.0.0.1", port:c.port, user, key:keys.client_key(), known_hosts:keys.known_hosts()}`, `Ownership::AppOwned{ai_clis: AiCliSelection::all(), table: AiCliTable::pinned(), state_dir:"/var/lib/micold-ai-ide", egress}`, `StepBounds::container()`, and runs like `ssh_attempt` (`daemon.rs:774`) with `provisioning_sink`. Keys dir = `<state_dir>/ssh` per daemon (data-model says `<daemon state dir>/ssh`; pick per-daemon dir by container name). `DaemonKeys::ensure(dir, platform, runner)` is async and must run before `create` (the files are mounted): do it in the bring-up task before `start`. `known_hosts` via `DaemonKeys::write_known_hosts(port)`.
- OPEN question for the next unit: the SSH login name. The entrypoint appends `micold:x:<uid>` only when no passwd line has that uid; the node base image has `node` at uid 1000, so on a host uid 1000 the login name is `node`, else `micold`. Either read the name with `<runtime> exec` or make `PinnedReach.user` robust; changing `packaging/basic/` changes the content hash and needs a new publish (avoid). `provision/mod.rs` tests use user "dev".
- `Dial::Container` keeps the legacy TCP+token path for `Adoption::Legacy`; the connection hook for `Dial` is `daemon.rs:489`, and `tests/per_daemon_actors.rs:93`, `tests/daemons_feature.rs:839` match on it.
- Disk 20 GB free at start: wipe `target-shared/debug` before the gate. `SCRATCHPAD` env var is unset in the shell: export it from the prompt.

## Open escalation

None.

## Follow-ups not done

- specs/686-kubernetes-daemon-runtime FR-004, FR-012 and its clarifications still describe #806 as a Dockerfile build pushed to a registry; the owner's decision replaces that with SSH provisioning (806 FR-008). That spec belongs to the #686 flow.
- M5 (T051, `SandboxSpec` argv): the basic image makes `/etc/passwd`/`/etc/group` writable by gid 0 only (contract); a container run as `--user uid:gid` with the host gid cannot append its passwd line unless the create argv also adds group 0 (`--group-add 0`, podman `--userns=keep-id` gives an entry itself). The entrypoint warns instead of failing. Check when M5 writes the argv.
- M1 `sandbox_real_basic_image` was not run green locally (this environment's egress refuses `deb.debian.org`, so `micold-basic:dev` cannot be built here); its red was shown against `node:24-trixie-slim`. CI's `sandbox-runtime` job runs it.
- The `BASIC_IMAGE` digest pin and CI's pin check are T049 (M5): the digest exists only after `basic-image.yml` has run on `main` after this PR merges.
- M4: `install_step` does nothing for an empty selection (no `offered.json` is written, so a daemon whose selection was emptied after an install keeps offering the old file's CLIs). The per-daemon selection (M6, T071+) owns that case: it must write the file for an empty selection too.
