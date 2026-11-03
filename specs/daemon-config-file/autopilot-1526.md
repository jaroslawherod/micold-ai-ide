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
- **Next step**: M1b review B, gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1656 | Design (milestone for Issue 5) | MERGED | 86a9e5d9df604be5849f8cb7a8db9e9ea487d996 |
| #1659 | Re-cut M1 into M1a/M1b | MERGED | 1f4e769c01901d65377a3cafc8dee0ca7606009b |
| #1660 | M1a (host delivery) | MERGED | 974af5dc9804ffe266f8fd9e1fd70f22f50eefdf |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1a | T025, T027, T080a | full | host file written `0600` before spawn, only on change; a runtime-added project appears in the next file written | #1660 | merged |
| M1b | T026, T028, T029 | full | SSH file and secrets copied to `~/.micold/remote/` (`0700`/`0600`), a rotated token re-copies only that file; SSH env list for user hosts gone; user guide documents the SSH file | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main (PR #1509); same precedent as #1524, #1525, #1534 | agent-resolved | specs/daemon-config-file/autopilot-1524.md, autopilot-1525.md, autopilot-1534.md |
| D2 | tasks | Split Issue 5? | No: 6 tasks, one story (host + SSH delivery share the write-if-changed path and FR-005/FR-029), rule 3 of milestones.md | agent-resolved | milestones.md rule 3 |
| D3 | tasks | Include T080a? | Yes: Issue 13's ledger (#1534, D2) already moved it here because it needs T027's delivery | agent-resolved | specs/daemon-config-file/autopilot-1534.md |
| D4 | tasks | Tier | full: touches process spawn, SSH provisioning and file permissions (security-relevant); M1 is always full | agent-resolved | milestone-format.md |
| D5 | implement | Generic secret copy (a) or `client_token_file`-only (b)? | (b): ship the copy-step plumbing against `client_token_file` only; note the rest as a follow-up for Issues 8/9/11/16 | agent-resolved | FR-005 ("deliver it, with the secret files it names, by the kind's own means" — "it names" ties the secret set to what the renderer already names, not a generic reflective walk); contracts/daemon-config.md §3.2 R3/R5; `daemon_config_render.rs:41-103`: `DaemonSettings` has exactly one secret-shaped field (`client_token_file`), and `from_entry`'s `host()` closure (used for both `Kind::Host` and `Kind::Ssh`) never sets it — only `sandbox()` does. So M1's real secret set is empty for host/SSH today; the generic mechanism is exercised by a test that constructs a `DaemonSettings` with `client_token_file` set directly, not through the real `from_entry` wiring |
| D6 | tasks | How to split the non-converging M1? | Decided by user: Option A — split M1 into two milestones/PRs, M1a host delivery (T025, T027, T080a) and M1b SSH delivery (T026, T028, T029); M1b depends on M1a (reuses its write-if-changed helper) | decided by user | Open escalation (now resolved, see Milestones table and `specs/daemon-config-file/tasks.md`'s M1a/M1b entries) |

## Review rounds

- Tasks review round 1 (snapshot a9a1e1257e80ba2b94fe9cefd52688d321ef553f:8176ca30aac5c24fca47a85695e888d3122ad2b5): CHANGES; F1 (MAJOR, Satisfies line omitted US3/FR-014 for T080a) fixed, prose only (`Satisfies: US1, US3; FR-005, FR-014, FR-029`); no re-review needed.
- M1-split re-cut review round 1 (snapshot 45db00e65384f233e2329504c35e84c587bf14c1:251ed96cb96486391f3e171c6fc2e24a6ebe81e6): CLEAN. No findings.
- M1a code review A (code-review, high; snapshot 0c7becd612254eda65be945bcb840d03966f7789:582673e2660ca5d02d39a7fb2b8fab18cf6426b2): CLEAN. No findings.
- M1a review B (conformance, sonnet; same snapshot): CLEAN. Verify run for real: `daemon_config_delivery_host` (3/3 ok), `daemon_config_delivery_host_env` (ok), `daemon::tests::a_runtime_added_project_reaches_the_host_dials_settings` (ok), `features::daemons::runtime_project_tests::a_recorded_project_is_kept_once_per_daemon` (ok); confirmed M1b (SSH) untouched and tasks.md shows T025/T027/T080a ticked.

## Declined review findings

None.

## Handover

**Continuation (unit 2)**: review-A fixes a-c applied (ssh_attempt returns Err on a config that does not render; extra Copy event dropped; copy_if_changed chmods on skip, test asserts it); scoped gate green. Remaining: review B, gate, docs check, PR. Declined (review A, follow-up): client_token_file path vs delivered secret, unreadable token file / SshDial not hashing secret content, stale secrets never removed (all dormant until a later Issue sets client_token_file, D5(b)); config change not applied to an already-running remote daemon (no reload; Issue 9/16); copy error-mapping duplication (small).

**M1b UNIT HANDOVER (read this first; the M1a text below is history).** The M1b unit was stopped
by `Disk quota exceeded` on `/tmp/claude-1000` (Bash tool unusable: every call fails with EDQUOT
before running). Free space there (the session scratchpad holds big gate logs) or set
`CLAUDE_CODE_TMPDIR` elsewhere, then continue.
- Done: ledger records M1a merged (commit 1); M1b implemented in commit 2 `feat(1526): deliver the
  configuration to SSH daemons`: `ssh/argv.rs` (`REMOTE_CONFIG`, `remote_secret_path`,
  `copy_if_changed` + real-shell test), `provision/mod.rs` (`ConfigDelivery`, `ProvisionTarget.config`,
  `deliver_config` called at both `ensure_build` exits, 3 Fake tests; `config: None` added at the
  other `ProvisionTarget` sites incl. `kube/start.rs`, `daemon.rs::container_target`,
  `micold-daemon/tests/sandbox_real_support/mod.rs`), `daemon.rs` (`Dial::Ssh { target, settings:
  SshDial }`, `named_secrets` (D5(b)), `ssh_attempt` takes settings, runtime projects applied to the
  Ssh arm too), docs (`remote-daemons.md` new section, `daemon-configuration.md` sentence), T026/T028/
  T029 ticked, test-list U24 DONE. `cargo test -p micold-core --lib provision:: / copy_if_changed`
  passed. Uncommitted: a clippy fix in `provision/mod.rs` (`.err().expect` -> `expect_err`).
- The scoped gate (`scripts/autopilot/scoped-gate.sh`) first run failed only on that clippy lint;
  the rerun's result was lost to the quota error. Rerun it, then commit.
- Review A (code-review high) returned 9 findings; decisions: FIX (small): (a) `ssh_attempt` should
  return `Err(<joined ConfigError text>)` when `prepare` fails instead of `.ok()`/silent drop, and
  the comment "starts with its defaults" is wrong; (b) drop the extra `sink(Entered(Step::Copy))` in
  `deliver_config` (UI noise when nothing is copied); (c) `copy_if_changed`'s skip branch should
  `chmod {mode:o}` the file too (a hand-widened mode is corrected). DECLINE/follow-up (record under
  Declined): client_token_file rendered path vs delivered `secrets/client-token` (dormant: `from_entry`
  never sets `client_token_file` for host/SSH, D5(b); belongs with the first Issue that sets it),
  unreadable token file silently empty and `SshDial` not hashing secret content (same), stale
  secrets never removed (same), config change not applied to an already-running remote daemon (no
  reload exists; Issue 9/16 area), duplication of copy error mapping (accept, small).
  Add a test for (a)/(c) if cheap (the argv real-shell test can assert the mode correction).
- Remaining: apply a-c, scoped gate, review B (conformance, sonnet, `autopilot-reviewer`; verify =
  `cargo test -p micold-core provision`, `cargo test -p micold-core ssh::argv`), then full `mise run
  gate` (see Local gate note: OOM contention and the pre-existing `micold-daemon` exclusivity test
  `a_displaced_client_is_not_terminated` may fail; if so, record scoped tree per gate.md and say
  "Local gate: scoped only, full gate left to CI"), docs check:
  `CHANGE_TITLE="feat(1526): ... (#1526)" scripts/check-user-guide-updated.sh origin/main HEAD`
  (needs the env var). PR title `feat(1526): deliver the configuration to SSH daemons (#1526)`,
  body notes: "SSH env list for user hosts" was already empty (a constraint honored, not removal
  work); the secret path is untested against real content because none exists for host/SSH yet
  (D5(b)); body ends `Refs #1526`.

**M1a (T025, T027, T080a) is implemented, reviewed (A and B both CLEAN) and its PR is open: #1660.**
The orchestrator waits on CI and merges; a fresh unit should not redo any of the implementation,
gate or review work below.

**CI fix (attempt 1 of 3)**: PR #1660's first CI run came back RED on the `docs check` job only
(`feat(1526): …` title + `crates/micold-client/src/features/daemons.rs` flagged user-facing, no
`docs/user-guide/` page touched). Fixed by updating
`docs/user-guide/daemon-configuration.md`'s "Projects and the clone root" section: the old sentence
"A project you add in the application while the daemon runs works at once; it is not yet written
into the file" was stale as of T080a/T027 — it is now written into `daemon.toml` for a **host**
daemon (not yet for SSH, M1b's job). Verified locally with `scripts/check-user-guide-updated.sh
origin/main HEAD` (after committing) and `mise run test-scripts` (docs-only gate per gate.md's
table); both clean. No code changed.

**Local gate note**: the scoped gate (fmt + clippy + tests of `micold-client`, the only crate this
diff touches) passed clean. The full `mise run gate` was attempted twice and failed both times for
reasons unrelated to this diff: (1) resource contention with another concurrently-running worktree
building on the same shared `target-shared/` caused one run to SIGKILL (OOM, signal 9) mid-compile;
(2) `crates/micold-daemon/tests/exclusivity.rs::a_displaced_client_is_not_terminated` fails
deterministically on this tree — confirmed pre-existing and unrelated: `git diff --stat
origin/main...HEAD -- crates/micold-daemon` is empty (this diff touches only `micold-client`), and
`origin/main`'s own latest CI run (sha `b7b7410083a18ebd04b5bfffe26013eb9e07340f`) is already red
(a Windows build failure, PR #1653, also unrelated to this diff). Per gate.md's "let CI validate
instead" and escalation.md's "a problem from work outside this flow... do not fix it", this unit
recorded the scoped-gate tree in `autopilot-gate-ok` and pushed; the PR body says "Local gate:
scoped only, full gate left to CI". A red CI on this PR should be checked against the same two
causes before treating it as this flow's own failure.

**What landed (all tests green, `cargo fmt --check` clean, `cargo clippy -p micold-client
--all-targets -- -D warnings` clean; `mise run gate` not yet run)**:
- `crates/micold-client/src/daemon_config_render.rs`: `DaemonSettings` now derives `Eq`.
- New `crates/micold-client/src/daemon_config_delivery.rs` (`deliver_host`, write-if-changed, mode
  `0600` via `set_owner_only`), wired into `lib.rs`.
- `crates/micold-client/src/daemon.rs`: `Dial::Host` → `Dial::Host(HostDial)` (`HostDial { settings:
  DaemonSettings, data_dir: Option<PathBuf> }`, manual `Hash` over the rendered bytes); the 4 match
  sites updated (`actor_specs_answered`, `actor()`, the 2 `connect_and_pump` arms — the second now
  calls `deliver_host` before `connect_or_spawn` and propagates a write error as `Err`).
- T080a: `crate::features::daemons::State` gained `runtime_projects: BTreeMap<DaemonId,
  Vec<PathBuf>>` plus `add_runtime_project`/`runtime_projects()`; `actor_specs_answered` and
  `connections` both gained a trailing `runtime_projects` parameter (threaded through every call
  site: `shell/sandbox_real.rs`, `shell/daemons.rs` test, `tests/daemons_feature.rs`,
  `shell/subscriptions.rs`'s real call via `app.core.daemon_list.runtime_projects()`); the Host arm
  applies `settings.add_runtime_project(path)` for each recorded path. `crate::shell::setup::
  add_project` now calls `app.core.daemon_list.add_runtime_project(daemon, path)` when `binding` is
  `Bound` (the `Output::ProjectAdd` handler in `daemon_sync.rs:4102` already calls `add_project`, so
  nothing else needed changing). The Ssh arm is untouched on purpose — it carries no settings yet;
  that is M1b/T028's job, so a project added at runtime to an SSH daemon is not yet delivered (only
  the host path is M1a's scope).
- Tests: `crates/micold-client/tests/daemon_config_delivery_host.rs` (3 cases: write-if-changed,
  no-op on unchanged, rewrite on a settings change — wired into `tests/main.rs`'s `mod` list) plus
  `crates/micold-client/tests/daemon_config_delivery_host_env.rs` as its **own** `[[test]]` target
  in `Cargo.toml` (not a `main.rs` module: it sets process env vars, which `main.rs`'s own header
  comment says must not share a binary with tests that don't expect that). A unit test in
  `daemon.rs`'s existing test module (`a_runtime_added_project_reaches_the_host_dials_settings`) and
  one in `features/daemons.rs` (`runtime_project_tests::a_recorded_project_is_kept_once_per_daemon`)
  cover T080a directly.
- **A real gotcha worth recording so nobody re-derives it**: on this machine `/bin/sh` is `dash`,
  and dash drops any inherited env var whose name is not a valid shell identifier — in particular
  `CARGO_BIN_EXE_micold-ai-ide`/`CARGO_BIN_EXE_micold-showcase` (hyphen from the binary's own name)
  — before it execs a command inside a script. A naive "dump env via a `sh` script, diff against
  `std::env::vars()` in the test process" test fails on exactly those two vars, even though
  `spawn_detached_daemon` adds nothing. The env regression test instead dumps through the **same**
  script twice (once run directly, once through `spawn_detached_daemon`) and diffs those two dumps,
  which cancels the dash-filtering artifact out and still catches a real regression.
- `specs/daemon-config-file/tasks.md`: T025/T027/T080a ticked `[x]`, each with its behavior ids.
- `specs/daemon-config-file/tdd/test-list.md`: new "Issue 5 (#1526), milestone M1a" section, U21-U23
  all `DONE`.

**M1b (T026, T028, T029) is not started** — a later unit does it, after M1a's PR merges, reusing
`deliver_host`'s write-if-changed shape server-side over the SSH copy step. Its design is below,
unchanged from the last handover.

**Scope narrower than it looks**: M1 only delivers the keys `render()` already covers (`[daemon]`,
`[listen]`, `[auth]`, `[storage]`, `[ssh]`, `[[projects]]` — contracts/daemon-config.md §3.1/§3.2).
The `[git]`/`[[github.tokens]]`/`[ai]`/`[session]`/`[startup]` tables are Issues 8-12, not this
milestone — do not expand `render()`'s key coverage here.

**Confirmed facts, do not re-derive**:
- `micold_core::paths::data_dir()` is the *exact* directory the daemon itself resolves by default
  for the host config (`crates/micold-daemon/src/config_load.rs:36-38`,
  `DefaultLocations::system(data_dir)`), so the client writes there directly — no `state_dir`
  parameter threading needed for the host path.
- `Dial::Host` / `Dial::Ssh` (`daemon.rs:191`/`203`) are matched in only ~4-5 places total in
  `daemon.rs` (grepped across the whole crate) — adding fields to each is a contained change, not a
  sweeping one. Sites: `actor_specs_answered`'s `DaemonRuntime::Host =>`/`Ssh(s) =>` arms (~416,
  441), `actor()`'s endpoint-resolution match (~583), `connect_and_pump`'s two `Dial::Host` arms
  (~693, 696) and one `Dial::Ssh` arm (~700), plus one unit test at ~2005 that destructures
  `Dial::Ssh { target }` (needs `, ..` added) and one at ~2244 (already uses `Dial::Ssh { .. }`,
  untouched).
- `DaemonEntry` (`micold-core/src/daemons.rs:171`) has **no `projects` field at all** —
  `DaemonSettings.projects` is always empty out of `from_entry`; `add_runtime_project` only mutates
  an in-memory `DaemonSettings`. T080a therefore needs a *new* place to hold runtime-added project
  paths across the app's run (see its own section below) — this was not obvious going in.
- `run_with_relay` (`provision/mod.rs:~592`) is the **shared** sequence for both `Ownership::UserHost`
  (SSH host, this issue) and `Ownership::AppOwned` (container-over-SSH, a different issue) targets.
  Any new delivery step must no-op when its new `ProvisionTarget` field is `None`, which it always
  is for the container-over-SSH path — load-bearing, not cosmetic.
- `Step` (`provision/failure.rs:15`) is shared with feature 489's provisioning UI row list and
  `contracts/provisioning.md` (a different feature) — **do not add a variant** for this feature's
  config/secret copy; fold it into the existing `Step::Copy` (it still says "copying the daemon",
  which stays true enough — the config rides along).
- `ProvisionTarget::user_host`/`owned_with` (`provision/mod.rs:142-153`, `~800`) already set
  `env: Vec::new()` and have done so since the field was introduced (checked full file history) —
  there never was a non-empty SSH env list for user hosts to "drop"; T028's "drop the env list" is a
  **constraint to honor** (never add `MICOLD_*` env to reach a user host), not removal work. Say so
  in the PR body.

**T025/T027 — host delivery** (write-if-changed before spawn, mode 0600, no added `MICOLD_*` env):
1. Add `Eq` to `DaemonSettings`'s derive (`daemon_config_render.rs:40`, currently
   `#[derive(Debug, Clone, PartialEq)]`) — every field is already `Eq` (`ProjectSetting` already
   derives it at line 32; `Listen`/`IdleStopSetting` in `micold_core::daemon_config` are `Eq` too),
   so this is a one-line, risk-free change.
2. New module `crates/micold-client/src/daemon_config_delivery.rs` (wire into `lib.rs`'s `pub mod`
   list beside `daemon_config_render`): `pub fn deliver_host(settings: &DaemonSettings, data_dir:
   &Path) -> io::Result<bool>` — `daemon_config_render::prepare(settings, Kind::Host)` (map its
   `Vec<ConfigError>` to an `io::Error` via `ConfigError`'s existing `Display`), then a private
   `write_if_changed(path, bytes) -> io::Result<bool>`: `std::fs::read(path)` and compare, skip if
   equal, else `create_dir_all` the parent, `fs::write`, then `set_permissions(0o600)` behind
   `#[cfg(unix)]` (no-op `#[cfg(windows)]`, matching `spawn.rs`'s existing split style). Returns
   whether it wrote.
3. In `daemon.rs`, add near the `Dial` enum:
   ```rust
   pub struct HostDial { pub settings: DaemonSettings, pub data_dir: Option<PathBuf> }
   // #[derive(Debug, Clone, PartialEq, Eq)] — needs DaemonSettings: Eq (step 1)
   impl Hash for HostDial { // hash the rendered bytes, DaemonSettings itself is not Hash
       fn hash<H: Hasher>(&self, state: &mut H) {
           daemon_config_render::render(&self.settings, Kind::Host).hash(state);
           self.data_dir.hash(state);
       }
   }
   ```
   (same idiom as the existing `KubeDial`/`ContainerBringUp` wrappers a few lines below it, which
   also hash a derived string because their payload isn't `Hash`). Change `Dial::Host` →
   `Dial::Host(HostDial)`.
4. `actor_specs_answered`'s Host arm (~416): `DaemonRuntime::Host => Dial::Host(HostDial {
   settings: DaemonSettings::from_entry(entry)?.0, data_dir: micold_core::paths::data_dir() })`.
5. `actor()`'s match (~583): `Dial::Host(_) => match endpoint::resolve() { ... }` (fields unused
   here; only which variant matters).
6. `connect_and_pump`'s two arms (~693, 696): on `(Dial::Host(host), Some(endpoint))`, call
   `crate::daemon_config_delivery::deliver_host(&host.settings, dir)` for
   `host.data_dir.as_deref()` (an `io::Error::new(NotFound, ...)` when `data_dir` is `None`, same
   style as the existing `(Dial::Host, None) => Err(...)` arm) **before** awaiting
   `connect_or_spawn`; propagate a write error as `Err(err)` (the `attempt` variable's type is
   `io::Result<Connected>`, confirmed by reading the whole match and how `attempt` is consumed right
   after it — every arm already funnels into that one type).
7. New test file `crates/micold-client/tests/daemon_config_delivery_host.rs`; wire it into
   `crates/micold-client/tests/main.rs`'s `mod` list, alphabetically between `daemon_adopt_on_restart`
   (line 61) and `daemon_config_render` (line 62) — tests here are modules of one linked binary now
   (#1631), not separate targets; `every_test_file_is_wired` enforces this. Cases:
   - write-if-changed: build a `DaemonSettings` (reuse/extend the `settings()`/fixtures helper at
     `tests/support/daemon_config_fixtures.rs`), a tempdir as `data_dir`; first `deliver_host` call
     → `Ok(true)`, file exists, bytes equal `daemon_config_render::prepare(&settings,
     Kind::Host).unwrap()`, mode `0600` (unix: `fs::metadata(..).permissions().mode() & 0o777`);
     second call, same settings → `Ok(false)`, file untouched (check mtime or just re-assert bytes).
   - a settings change (e.g. `settings.add_runtime_project(...)`) → `Ok(true)` again, new bytes.
   - a `#[cfg(unix)]` env regression test: set `MICOLD_DAEMON_BIN` to a tiny `sh` script (write one
     to the tempdir) that does `env > "$SOME_OUT_PATH"`, set `SOME_OUT_PATH` (a test-chosen env var
     name, inherited — that's fine, it's not something the *client* adds) to a tempfile path, call
     `micold_core::spawn::spawn_detached_daemon()`, poll (bounded) for the output file to appear,
     then assert the keys in that file equal `std::env::vars()`'s keys of the test process **before**
     the call (i.e., nothing was added) — proves `spawn_detached_daemon` adds no env, which is
     already true in `spawn.rs` today (a bare `Command::new` with no `.env()` calls); this is a
     regression guard, not new production code.

**T026/T028 — SSH delivery** (copy `daemon.toml` + named secrets to `~/.micold/remote/`, rotated
token re-copies only that file):
1. `ssh/argv.rs`: add `pub const REMOTE_CONFIG: &str = "$HOME/.micold/remote/daemon.toml";` and
   `pub fn remote_secret_path(name: &str) -> String { format!("$HOME/.micold/remote/secrets/{name}")
   }`, plus:
   ```rust
   /// Writes stdin to `remote_path` (mode octal) only when the file's current sha256 differs from
   /// `hash`: the remote shell itself decides, so a rotated secret re-copies only itself with no
   /// local cache (research D10's write-if-changed, done server-side for the copy path).
   pub fn copy_if_changed(reach: &Reach, platform: ClientPlatform, timeout: u32, remote_path: &str,
       mode: u32, hash: &str) -> SshCommand {
       let remote = format!(
           "sh -c 'd=$(dirname \"{remote_path}\"); mkdir -p \"$d\" 2>/dev/null; \
            cur=$(sha256sum \"{remote_path}\" 2>/dev/null | cut -d\" \" -f1); \
            if [ \"$cur\" = \"{hash}\" ]; then cat >/dev/null; else \
            umask 077; p=\"$d/.part.$$\"; cat > \"$p\" && chmod {mode:o} \"$p\" && mv \"$p\" \
            \"{remote_path}\" || {{ rm -f \"$p\"; exit 1; }}; fi'"
       );
       command(reach, platform, timeout, &remote, None)
   }
   ```
   (`hash` is always our own `sha256_hex` hex output — safe to interpolate; `remote_path` is always
   one we construct, never user text). Mirrors `copy`'s `.part.$$` idiom (`ssh/argv.rs:281-288`).
2. `provision/mod.rs`: add `pub struct ConfigDelivery { pub config: Vec<u8>, pub secrets:
   Vec<(String, Vec<u8>)> }` and a field `pub config: Option<ConfigDelivery>` on `ProvisionTarget`
   (`None` in both `user_host()` and the test-only `owned_with()`/`owned_target()` helpers — the
   production `AppOwned` construction site, wherever that is for container-over-SSH, also leaves it
   `None`, satisfying the "shared `run_with_relay`" constraint above).
3. In `ensure_build` (`provision/mod.rs:341`), add a `deliver_config(host, target, opts, cancel)`
   call at **both** exit points that currently `return Ok((probe, false))` (the `probe.have` early
   return) and `Ok((probe, true))` (after a fresh binary copy) — i.e., config/secrets are delivered
   on every `ensure_build`, binary-cached or not. `deliver_config` no-ops when `target.config` is
   `None`; otherwise it `guard`s under `Step::Copy` (reuse, do not add a `Step` variant — see
   "Confirmed facts") and calls a small `upload(host, target, opts, bound, path, mode, bytes)`
   helper per file: `sha256_hex(bytes)` (`micold_core::protocol::hashing::sha256_hex`, already used
   in `config_load.rs`), build the command with `argv::copy_if_changed`, `host.run(cmd,
   bytes.to_vec(), bound, None).await`, map a non-`ok()` outcome the same way the existing binary
   copy does (`TimedOut`/`CopyFailed`, reusing `io_fail`/`fail`).
4. `provision/mod.rs`'s test module (`#[cfg(test)] mod tests`, ~640): the `kind(cmd, stdin)`
   classifier (~660) checks `last.contains("cat >")` for `"copy"` — add checks for
   `last.contains("daemon.toml")` → `"config"` and `last.contains("/secrets/")` → `"secret"`
   **before** that check (both new commands also contain `cat >`), so `Fake` can script config/
   secret responses distinctly from the binary copy. Existing tests are unaffected: they never set
   `target.config`, so `deliver_config` no-ops and never issues a "config"/"secret" command.
5. New unit tests in that same module, built directly on `ensure_build`/`run_with_relay` with a
   `ProvisionTarget` whose `.config = Some(ConfigDelivery { config: b"...".to_vec(), secrets:
   vec![("client-token".into(), b"tok1".to_vec())] })` (constructed directly — this is where D5's
   "(b)" is actually exercised, since the real wiring below never populates a secret today):
   first run copies both the config and the one secret; an identical second run copies neither
   (script the Fake's `"config"`/`"secret"` kinds with a success `SshOutput` and assert `count(...)
   == 1` each, or check the Fake's remote-side hash comparison end to end if the Fake simulates a
   filesystem — simplest is to script the **remote** side realistically enough that
   `sha256sum`-against-`hash` actually differs/matches, which the current `Fake` does not simulate
   (it just returns scripted `SshOutput`s per kind, blind to content) — the test instead asserts at
   the `SshCommand` level: the second call's `copy_if_changed` command string contains the *same*
   `hash` as the first (proving the client passed the unchanged content's hash both times; the
   remote-side skip is the shell script's job, already covered by a plain shell-logic read, not by
   the Fake); then change only the secret's bytes and assert the **config**'s command hash is
   unchanged while the **secret**'s command hash changed — that is what proves "only that file
   re-copies" at the level this test harness can see (the Fake never runs the shell for real).
6. Wire it in `daemon.rs`:
   - `Dial::Ssh` gets a second field: `settings: SshDial` where `pub struct SshDial(pub
     DaemonSettings);` with a custom `Hash` (`render(&self.0, Kind::Ssh).hash(state)`), same pattern
     as `HostDial`/`KubeDial`. Construction at `actor_specs_answered`'s Ssh arm (~441):
     `settings: SshDial(DaemonSettings::from_entry(entry)?.0)`.
   - `ssh_attempt` (`daemon.rs:1230`) gains a `settings: &DaemonSettings` parameter; build
     `ProvisionTarget::user_host(target, ...)` as today, then set `.config =
     daemon_config_render::prepare(settings, Kind::Ssh).ok().map(|text| ConfigDelivery { config:
     text.into_bytes(), secrets: named_secrets(settings) })` where `named_secrets` is a small new
     function (per D5(b)) that, **only** for `settings.client_token_file`, reads the local file's
     bytes and returns `vec![("client-token".into(), bytes)]` (empty `Vec` when `None`, which is
     every real case today — `from_entry`'s `host()` closure never sets it). Note in the PR body
     that this path is untested against real secret content because none exists yet for host/SSH;
     the generic mechanism's test coverage is the provision/mod.rs unit tests in step 5.
   - Update the call site (`connect_and_pump`'s `(Dial::Ssh { target }, _) =>`, ~700) to `(Dial::Ssh
     { target, settings }, _) => { ... ssh_attempt(..., target, &settings.0, &sink).await ... }`.
   - Fix the one test at ~2005 that destructures `Dial::Ssh { target }` — add `, ..`.

**T080a — `add_runtime_project` from `ProjectAdd`**: blocked on a small design gap found this round:
since `DaemonEntry` carries no `projects` field, there is currently nowhere to persist a
runtime-added project across the recomputations of `actor_specs_answered` (which rebuilds
`DaemonSettings` fresh from `entry` every time via `from_entry`). The shape to build: a new
`BTreeMap<DaemonId, Vec<PathBuf>>` of runtime-added projects, held in whatever app/shell state
already owns `DaemonStates`/`DaemonLinks` (grep for where those live, e.g. `app::State` or
`shell/daemons.rs`), passed into `actor_specs_answered` as a new parameter (same pattern as
`drift`/`boot_owner` today) and applied via `settings.add_runtime_project(path)` for each entry in
its list, right after `DaemonSettings::from_entry(entry)` in both the Host and Ssh arms. The
`Output::ProjectAdd` handler (`daemon_sync.rs:4102`, calls `setup::add_project`) is where to insert
into that map. Needs T027 (and ideally T028) landed first — their `Dial` changes are what make a
settings change actually cause a rewrite. Do this task last.

**T029 — docs**: `docs/user-guide/remote-daemons.md`, not started; straightforward once T026/T028
land (document the `~/.micold/remote/daemon.toml` path, the `secrets/` directory, and that hand
edits are overwritten, per the file's own header comment and contracts/daemon-config.md §3.2).

**Stack profile** (`.specify/memory/tdd-profile.md`): rust, `cargo test` only, no mocks/coverage/
mutation tooling; exemplars `crates/micold-core/tests/workspace_lookup.rs` (unit/integration),
`crates/micold-client/tests/support/mod.rs` (fixtures); single-test command
`scripts/build-lock.sh cargo test --test {file} {name} -- --exact` (needs the `{name}` guard, a
typo'd filter silently passes). Prefer `mise run check`/targeted builds while iterating; `tests/
main.rs` links the whole `micold-client` test binary, so a full `cargo test -p micold-client` run is
expensive — see CLAUDE.md's shared-build notes.

PR: #1659 (re-cuts M1 into M1a/M1b; no code).

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 4 (#1525)" through "Issue 14 (#1535)"; #1525 stays open for follow-up work in PR #1654 and may edit the same section — expect a rebase if the second to merge.
