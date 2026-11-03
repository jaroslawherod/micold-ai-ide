# Autopilot ledger — #1526 daemon-config-file (Issue 5)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1526` (Deliver the configuration to host and SSH daemons); labels: `flow:feature`, `effort:low`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525 and #1534 (see `specs/daemon-config-file/autopilot-1524.md`, `autopilot-1525.md`, `autopilot-1534.md`); the tasks unit only cuts milestones for the "Issue 5" group (T025–T029, T080a).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1526
- **Worktree branch**: fix/1526_deliver-the-configuration-to-host-and-ssh-daemons
- **Started**: 2026-11-02
- **Phase**: implement
- **Next step**: M1 (T025–T029, T080a)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1656 | Design (milestone for Issue 5) | MERGED | 86a9e5d9df604be5849f8cb7a8db9e9ea487d996 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T025–T029, T080a | full | host file written `0600` before spawn, only on change; SSH file and secrets copied to `~/.micold/remote/` (`0700`/`0600`), a rotated token re-copies only that file; SSH env list for user hosts gone | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main (PR #1509); same precedent as #1524, #1525, #1534 | agent-resolved | specs/daemon-config-file/autopilot-1524.md, autopilot-1525.md, autopilot-1534.md |
| D2 | tasks | Split Issue 5? | No: 6 tasks, one story (host + SSH delivery share the write-if-changed path and FR-005/FR-029), rule 3 of milestones.md | agent-resolved | milestones.md rule 3 |
| D3 | tasks | Include T080a? | Yes: Issue 13's ledger (#1534, D2) already moved it here because it needs T027's delivery | agent-resolved | specs/daemon-config-file/autopilot-1534.md |
| D4 | tasks | Tier | full: touches process spawn, SSH provisioning and file permissions (security-relevant); M1 is always full | agent-resolved | milestone-format.md |

## Review rounds

- Tasks review round 1 (snapshot a9a1e1257e80ba2b94fe9cefd52688d321ef553f:8176ca30aac5c24fca47a85695e888d3122ad2b5): CHANGES; F1 (MAJOR, Satisfies line omitted US3/FR-014 for T080a) fixed, prose only (`Satisfies: US1, US3; FR-005, FR-014, FR-029`); no re-review needed.

## Declined review findings

None.

## Handover

No code written yet; this unit spent its budget on architecture research (it hit the context cap
before writing any test or implementation). Next unit: pick up at `implement.md` step 1
(`speckit-implement`/`speckit-tdd-run`, no prior cycles logged in `tdd/cycle-log.md` for this
milestone — `tdd/test-list.md` only has behaviors U1-U20/A1-A2 from Issue 2/3, none for M1; add new
rows there for T025/T026/T080a before running the loop, and mark T025/T026/T027/T028/T080a's lines
in tasks.md with the new behavior ids `[U21]` etc. so Phase 6 can tick them).

**Scope narrower than it looks**: M1 only delivers the keys `render()` already covers (`[daemon]`,
`[listen]`, `[auth]`, `[storage]`, `[ssh]`, `[[projects]]` — contracts/daemon-config.md §3.1/§3.2).
The `[git]`/`[[github.tokens]]`/`[ai]`/`[session]`/`[startup]` tables are Issues 8-12, not this
milestone — do not expand `render()`'s key coverage here.

**T025/T027 — host delivery** (write-if-changed before spawn, mode 0600, no `MICOLD_*` env):
- `spawn_detached_daemon` (`crates/micold-core/src/spawn.rs:73`) is called only from
  `connect_or_spawn` (`crates/micold-core/src/connect.rs:514,528`), itself called from
  `crates/micold-client/src/daemon.rs:694` at `(Dial::Host, Some(endpoint)) =>
  connect_or_spawn(...)` inside `connect_and_pump` (`daemon.rs:681`). `connect.rs` is core and has
  no access to `micold-client`'s `daemon_config_render` (the render lives in the client crate per
  Issue 4) — so the write-if-changed call must happen in `daemon.rs` just before that
  `connect_or_spawn` call, not inside core.
- Problem to solve: `Dial::Host` (`daemon.rs:191`) carries no fields today, so the config to render
  is not available at the call site. `ActorSpec`/`Dial` is the actor's **subscription identity**
  (`#[derive(Hash)]`, `daemon.rs:326`) — adding the rendered settings (or a hash of them) to
  `Dial::Host` restarts the actor when Settings change, which looks like the right trigger for a
  fresh write + reconnect, but confirm against spec.md/FR-029 before committing to it. Plumb from
  `actor_specs_answered` (`daemon.rs:392`, the `DaemonRuntime::Host => Dial::Host` arm at line 416)
  using `DaemonSettings::from_entry(entry)` (`daemon_config_render.rs:91`) — `entry: &DaemonEntry`
  is already in scope there.
- Write-if-changed: render via `daemon_config_render::prepare(settings, Kind::Host)`
  (`daemon_config_render.rs:155`), compare against the file at `<data_dir>/daemon.toml` (read
  existing bytes, skip the write when equal), else write + `set_permissions(0o600)` (unix; Windows
  has no mode bit, skip or no-op per the repo's existing `cfg(windows)` pattern — check
  `crates/micold-core/src/spawn.rs` for the existing `cfg(unix)`/`cfg(windows)` split style).
  `data_dir` comes from the same source `state_dir`/`legacy_default` resolution `actor_specs`
  already does (`daemon.rs:375` `state_dir: &Path` param) — likely the same directory
  `spawn_detached_daemon`'s daemon reads by default; confirm against `crates/micold-core/src/paths.rs`.
- "No `MICOLD_*` env": `spawn_detached_daemon` (`spawn.rs:73`) already builds a bare `Command` with
  no explicit env vars added (inherits parent env only) — grep the test fixture / CI env for
  `MICOLD_*` vars that might leak from the *client's own* process env into the spawned child
  (`Command::env_clear()` may be needed before `spawn()` if inheritance is the actual risk T025
  tests for). Read `spec.md`/FR-005 "by the kind's own means" before assuming inherited env is fine.
- New test file `crates/micold-client/tests/daemon_config_delivery_host.rs`; exemplar
  `crates/micold-client/tests/daemon_config_render.rs` (T078a) for style; helper
  `crates/micold-client/tests/support/mod.rs`.

**T026/T028 — SSH delivery** (copy `daemon.toml` + named secrets to `~/.micold/remote/`, rotated
token re-copies only that file; "drop the SSH env list for user hosts"):
- Finding: `ProvisionTarget::user_host` (`crates/micold-core/src/provision/mod.rs:142-153`, the
  exact lines the task cites) **already** sets `env: Vec::new()` — has done so since the `env`
  field was introduced (checked full git history of the file, every commit touching it adds the
  field and the empty value together). There never was a non-empty env list for user hosts to
  "drop". Read this as a **constraint to honor while implementing T028** (never add a `MICOLD_*`
  env var to carry config to a user host — deliver only via the file), not as removal work; note
  this in the PR body so a reviewer does not look for a diff that is not there.
  `bridge_with_env`/`bridge` (`crates/micold-core/src/ssh/argv.rs:320-346`) is the mechanism that
  would carry such env (used today only for the container-over-SSH path's `MICOLD_IDLE_STOP`,
  `daemon.rs:1283` `container_env`) — confirm it is never reached for `Dial::Ssh`.
  `ssh_attempt` (`daemon.rs:1230`, called from `daemon.rs:702`) is the user-host SSH connect path;
  the config copy belongs before/alongside its `ssh_run(...)` call (`daemon.rs:1241`), analogous to
  how `ensure_build` (`provision/mod.rs:341`) copies the daemon binary during `ssh_run`'s sequence.
- The binary copy's shell idiom is the pattern to copy from: `argv::copy` (`ssh/argv.rs:281-288`)
  builds `sh -c 'umask 077; p="$dir/.part.$$"; mkdir -p "$dir" && cat > "$p" && chmod 700 "$p" && mv
  "$p" ...'`, run via `host.run(cmd, bytes, bound, progress)` (`provision/mod.rs:389`, the
  `SshHost`/`SshRunner` trait, `provision.rs:71`). A new `argv::copy_config`-style helper (or a
  generalized `copy_file` taking the remote path and mode) writing to
  `~/.micold/remote/daemon.toml` (`0600`, directory `~/.micold/remote` presumably already `0700`
  from the existing build copy) and one per named secret under
  `~/.micold/remote/secrets/<name>` (dir `0700`, file `0600`) is the shape to build.
- **Open design question, not yet resolved — read `data-model.md` and `research.md` line 299
  before coding**: M1's `DaemonSettings` (`daemon_config_render.rs:41`) has exactly one secret-
  shaped field, `client_token_file: Option<PathBuf>` (contracts/daemon-config.md marks it
  `(secret)`), and it is only ever set for sandbox kinds (`SANDBOX_TOKEN` in `from_entry`'s
  `sandbox()` closure) — **never for `Kind::Host`/`Kind::Ssh` today**. So M1's actual host/SSH
  secret set may be empty in practice, and "the named secrets" in the task/milestone text is
  infrastructure for Issues 8/9/11/16 (github tokens, AI CLI secrets) to plug into later, not
  something with content yet. Two ways to resolve, pick by what the tests at T026 actually assert:
  (a) build a generic "copy every `SecretPath`-shaped field the *parsed* `micold_core::daemon_config::DaemonConfig`
  (`daemon_config/mod.rs:37`, `SecretPath` in `daemon_config/secret.rs:11`) resolves from the
  rendered text" — future-proof, more code now; or (b) ship the copy-step plumbing against
  `client_token_file` only (today's one secret field) and note the rest as a follow-up when Issues
  8/9/11/16 add fields. (b) is probably the right scope for a `low`-effort issue; confirm against
  spec.md's FR-005/FR-029 wording for Issue 5 before choosing.
- "A rotated token re-copies only that file": requires per-file content/hash tracking (likely a
  local sidecar, e.g. a small `BTreeMap<name, hash>` persisted beside the keys the
  `DaemonKeys`/`attempt::AttemptLedger` machinery already tracks under `state_dir`/`keys_dir`
  per-daemon) — look at how `probe.have` (`provision/mod.rs:360`, skips the binary copy when
  already present) short-circuits for the binary, and mirror that per-secret rather than
  per-daemon.
- New test; task text says "the provisioning copy step" — likely a new integration test in
  `crates/micold-core/tests/` (the T026 line says `provision/mod.rs`, not a client test), following
  the existing test helpers at `provision/mod.rs:780-830` (`user_target`, fake `SshHost`).

**T080a — `add_runtime_project` from `ProjectAdd`**: `DaemonSettings::add_runtime_project`
(`daemon_config_render.rs:71`) already exists and is unit-tested via T079. T080a's gap is wiring:
find where the client sees a `ProjectAdd` message/event (grep `ProjectAdd` across
`crates/micold-client/src/`) and call `add_runtime_project` on the live `DaemonSettings` that T027's
write-if-changed reads from, then trigger a re-render/re-write (reuse whatever "Settings changed"
path T027 wires `Dial::Host` to, above) so the next written file includes it. Needs T027's plumbing
to exist first — do T027 before T080a.

**T029 — docs**: `docs/user-guide/remote-daemons.md`, not started; straightforward once T026/T028
land (document the `~/.micold/remote/daemon.toml` path and that hand edits are overwritten, per
the file's own header comment, contracts/daemon-config.md §3.2).

**Stack profile** (`.specify/memory/tdd-profile.md`): rust, `cargo test` only, no mocks/coverage/
mutation tooling; exemplars `crates/micold-core/tests/workspace_lookup.rs` (unit/integration),
`crates/micold-client/tests/support/mod.rs` (fixtures); single-test command
`scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (needs the `{name}` guard, a
typo'd filter silently passes).

PR: not opened — no code to review yet.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 4 (#1525)" through "Issue 14 (#1535)"; #1525 stays open for follow-up work in PR #1654 and may edit the same section — expect a rebase if the second to merge.
