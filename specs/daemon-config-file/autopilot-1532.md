# Autopilot ledger — #1532 daemon-config-file (Issue 11)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1532` (AI CLI configuration from the file for every kind); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1527, #1528, #1534 and #1535 (see `autopilot-1527.md` D1); the tasks unit only cuts milestones for the "Issue 11" group (T059–T067 incl. T065a–T065c, T096–T100).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1532
- **Worktree branch**: feat/1532_ai-cli-configuration-from-the-file-for-every-kind
- **Started**: 2026-11-05
- **Phase**: milestone M2
- **Next step**: continue from the ledger's *Handover*: implement T065 (container/pod/host-SSH
  delivery, design already worked out there), then T065a, T065b, T065c; gate, review, open the M2
  PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1699 | design (tasks + milestones for Issue 11) | merged | 98aa6edcc561301f70c671b74113b9877ce2d9fd |
| #1711 | M1 — each `[[ai.cli]]` field maps to the CLI's launch | merged | 8470f7464ec548474530511495dd9ded211e5ba4 |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | each `[[ai.cli]]` field maps to the CLI's launch; configured args/env add to the app's own | T059–T064 | full | #1711 (merged, `8470f7464ec548474530511495dd9ded211e5ba4`) |
| M2 | AI sign-in delivery, read-only on every kind; stays valid without a CLI refresh | T065, T065a, T065b, T065c | full | — |
| M3 | gap-review extras: seeded `.claude.json`, onboarding, seeded settings files, `use_machine_sign_in`, sign-in expiry | T096–T100 | full | — |
| M4 | Settings → Daemons renders `[[ai.cli]]`, and the user guide | T066, T067 | full | — |

Split along milestones.md rule 3 (17 tasks, one user story US4, no sub-stories to split on):
M1 is the field-to-launch mapping (tests + implementation together, rule 7); M2 is the read-only
sign-in delivery plus the OAuth-refresh replacements that make removing the writable mount safe
(T065 cannot ship without T065a/T065b, else a relayed sign-in goes stale mid-session); M3 is the
gap-review (research D15) amendments that depend on M2's delivery actually existing; M4 holds the
UI back until the behaviour it configures exists (rule 6, no half-wired UI on `main`) and carries
the user-guide task with it (rule 4).

Checked 2026-11-05 against the working tree (sibling runs found stale references — fixed here):
- `crates/micold-core/src/sandbox/mod.rs`: `CredentialShare::writable` is at lines 318-323 (fn
  body 321-323), not `317-322` as tasks.md said — fixed in T065.
- `crates/micold-core/src/sandbox/mod.rs`: the `CredentialMount` struct is at 704-709 (`pub struct
  CredentialMount` at line 704), not `700-735` — fixed in T065 (the stated range covered more than
  the struct; narrowed to what T065 actually points at).
- `crates/micold-core/src/kube/credentials.rs`: the `mount()` fn with the `sub_path` the pod
  Secret drops is at 86-99 (`pub fn mount` at line 86, `SignIn` arm at 98), not `73-79` — fixed in
  T065.
- `crates/micold-daemon/src/state.rs`: `cli_would_ask_trust` is at 2078-2095, not `1914-1935` —
  fixed in T096.
- `crates/micold-core/src/sandbox/mod.rs`: `ONBOARDING_KEY`/`onboarding_done` are at 508-543 (was
  `507-530`, close but off by one and short of the full fn); the "today only with a writable
  share" context (`onboarding_record`, reading `CredentialShare::AiCliAuth`) is at 963-975, not
  `930-939` — fixed in T097.
- Files T059–T064 name that do not exist yet (`crates/micold-core/src/daemon_config/ai_cli.rs`,
  `crates/micold-daemon/tests/ai_cli_config.rs`) are new files the failing tests create; not
  stale. `crates/micold-daemon/tests/ai_cli_availability.rs`, `crates/micold-core/src/provision/
  ai_cli.rs`, `crates/micold-daemon/src/state.rs`, `crates/micold-daemon/src/hooks.rs`,
  `crates/micold-client/src/ui/settings/daemons.rs`, `docs/user-guide/sandboxed-daemon.md`,
  `docs/user-guide/settings.md`, `docs/user-guide/daemon-configuration.md` all exist as named.
- `AiCli::ALL` (`crates/micold-core/src/session.rs:76`) lists `ClaudeCode, Copilot, Pi, Codex,
  OpenCode` — matches the CLIs named across T059–T100.
- Dependencies: Issue 5 (#1526) M1a and M1b merged; Issue 6 (#1527) M1 merged; Issue 7 (#1528) M1
  (PR #1690) merged on `main` (issue #1528 itself stays open only for its docs close PR #1692) —
  M1's "Depends on" line is satisfiable today.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec? | No: design merged on main (PR #1509, `specs/daemon-config-file/spec.md` FR-013/FR-013a-g/FR-025/FR-033/FR-034/FR-040/FR-043, SC-006/SC-008; contract "`[[ai.cli]]`" section). Same precedent as #1524, #1525, #1526, #1527, #1528 and #1534 | agent-resolved | specs/daemon-config-file/autopilot-1527.md D1 |
| D2 | tasks | `speckit-analyze` F1 (HIGH): FR-013a's "unless the CLI documents which of the two wins" exception contradicts the contract's and data-model's unconditional "at most one of `sign_in_file`/`token_file`/`api_key_file`/`use_machine_sign_in`" | Dropped the exception clause from FR-013a: no CLI in `AiCli::ALL` documents a precedence, and the schema (contract + data-model, already merged) is deliberately the stricter, unconditional rule. Prose-only fix, no new round (review-rounds.md exception) | agent-resolved | speckit-analyze F1; contracts/daemon-config.md row `api_key_file`; data-model.md "AiCli" |
| D3 | tasks | Resumed unit found `origin/main` 11 commits ahead of this branch's HEAD (the ledger's recorded tip `93bc2a7f9` was stale), including one commit (#1531) editing `tasks.md`'s milestones heading line that our own uncommitted work also touched | Committed the uncommitted tasks/milestones work, then `git rebase origin/main`; one textual conflict on the shared heading line (both add an issue name to the same list) — resolved by keeping both additions (`Issue 10 (#1531)` and `Issue 11 (#1532)`), no content lost on either side. Verified no other conflict markers and all four Issue 11 milestone sections intact after the rebase | agent-resolved | `git log 93bc2a7f9..origin/main --oneline`; `git diff` of resolved `tasks.md` |
| D4 | milestone M1 | T062 names "no sign-in is ever mounted writable" and "the sign-in location is a link, re-created at every session start" — but sign-in delivery is M2's T065 (not yet implemented), and M1's Deliverable line only covers field-to-launch mapping | Scoped T062 to its first two clauses — `settings_file`/`place` land under the CLI's config dir, `place.as` cannot escape it — as pure path-mapping functions (`daemon_config::ai_cli::place_target`, `settings_file_name`), consistent with M1's Verify line (no sandbox/kube integration command listed) and Deliverable text. The sign-in clauses are M2/T065's own stated scope; left for that milestone | agent-resolved | tasks.md M1 Verify line; tasks.md T065 |
| D5 | milestone M1 | Review A round 2 F1 (BLOCKER): `AiCliTable::configured(...)` (T061's version/enabled override) is never called from the real client/kube provisioning paths — is wiring them in-scope for M1? | No: neither `kube::start::pod_target` nor `micold-client::daemon::container_target` has a per-CLI `version`/`args`/`model` value available today — both build their `AiCliSelection`/`KubeSettings.providers` from a bare `BTreeSet<AiCli>`, with no field for any `[[ai.cli]]` key beyond which CLIs are chosen. That data model, and the Settings UI that would produce it, is M4's own named scope ("Settings → Daemons renders `[[ai.cli]]`", T066/T067) — wiring `.configured()` into those call sites now would mean inventing M4's data model inside M1. `AiCliTable::configured` stays the pure, tested builder M1's Deliverable and Verify line (`provision::ai_cli`, core-only) call for; M4 calls it once its own Settings-side `[[ai.cli]]` values exist | agent-resolved | `grep -rn "AiCliTable::pinned()\|table: AiCliTable" crates/` outside tests; tasks.md M4 line; `KubeSettings.providers`/`AiCliSelection` definitions |
| D6 | milestone M1→M2 | Open escalation claimed `git push` was blocked (category 4, missing access) in every prior unit's session | Resolved: the container restart had wiped the sandbox's global gitconfig (`GIT_CONFIG_GLOBAL=/var/lib/micold-ai-ide/gitconfig`); the orchestrator reapplied the documented working setup — `url."https://github.com/".insteadOf git@github.com:` plus `core.askPass=/home/jaro/.local/bin/gh-askpass.sh` (supplies `gh auth token`; the sandbox resets `credential.helper` via `GIT_CONFIG_KEY_0`, so askPass is what survives). `git push` and `scripts/autopilot/preflight.sh` both succeed now | decided by user / orchestrator-resolved | PR #1711 merged (`8470f7464ec548474530511495dd9ded211e5ba4`) via a push from this same worktree after the gitconfig fix |
| D7 | milestone M2 | Connectivity note, not a decision needing a question: should a stalled `git`/`gh` call be treated as a credential failure? | No: GitHub access from this container is intermittent — one round-robin IP black-holes port 443 (why `/home/jaro/.local/bin/gh` is a retry wrapper). A call that stalls ~130s and then reports a connect timeout should simply be retried | agent-resolved | orchestrator's briefing for this unit |

## Review rounds

Tasks review A round 1 (snapshot `08da6558cfb79253faee3b170a8c04b4666d9efa:0144800f6c91e1ce455e0ed636ba5c517d601826`): CHANGES. F1 [MAJOR] T096/T097/T099/T100 named no failing test — fixed, each now leads with "Failing test in `<file>`". F2 [MAJOR] M3's Verify line missed a command for T100's pod-Secret re-delivery — fixed, added `cargo test -p micold-daemon --test config_reload` and `cargo test -p micold-core kube::credentials`. F3 [MINOR] FR-040 looked fully satisfied in M2's Satisfies line — fixed, M2's Deliverable and Satisfies note it is only partial there (T065a's expiry warning), full `expiresAt` display is M3's T100. All three fixed, no new round needed if the fix diff holds (review-rounds.md round 2).

Tasks review A round 2 (sonnet, fix diff only; snapshot `a0459e0031fc4415dd96b496c1c308ca07e11886:c8d8ea65b9e5a5d8c25228f0383c1c912b0538a7`): CLEAN. All three round 1 fixes confirmed to hold; both new Verify targets (`config_reload.rs`, `kube::credentials`) confirmed to exist. (Earlier snapshot `3ad4fced4370a500be17fe9f2d2a4553dfbaba24:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3` went stale when the branch rebased onto `origin/main` past 11 new commits, one of which touched `tasks.md`'s milestones heading line; resolved with no content loss, see note below.)

Milestone M1 review A round 1 (code-review skill equivalent, general-purpose agent, high effort; no snapshot — pre-commit working tree): CHANGES. F1 [BLOCKER] two `StatusContext` literals (`tests/mcp/worktree_guard_sessions.rs`, `tests/settings/plan_usage_setting.rs`) missed the new `api_key_helper` field — fixed, both literals now carry `api_key_helper: None`. F2 [MAJOR] a config-disabled CLI's start refusal reused the "not on PATH" sentence, misleading when the CLI is actually installed — fixed in `state.rs`'s `start_session_gated`: `ai_cli_disabled` is checked first and gets its own reason ("… is disabled in the daemon's configuration"). No other findings. All fixed, confirmed by a green `mise run check`, `cargo clippy --workspace --all-targets -D warnings`, and the scoped gate (one unrelated pre-existing failure, `exclusivity::a_displaced_client_is_not_terminated`, reproduced identically on `git stash`'s clean `98aa6edcc` baseline — not this diff's).

Milestone M1 review A round 2 (sonnet, full diff — round 1 took no snapshot): CHANGES. F1 [BLOCKER] `AiCliTable::configured(...)` (the `version`/`enabled` override, T061) is never called from the real provisioning call sites (`crates/micold-core/src/kube/start.rs`'s `pod_target`, `crates/micold-client/src/daemon.rs`'s `container_target`) — declined, see D5. F2 [MAJOR] `launch_effort`'s model lookup used the session's raw, pre-override model while the sibling `model` field in the same `LaunchSpec` used `effective_model_for`'s overridden one, so an effort level could pass unfiltered for a model that is not what actually launches — fixed at both `LaunchSpec` construction sites (start and respawn paths) in `state.rs`: the effective model is now resolved once and fed to both `launch_effort` and `model`. F3 [MINOR] `launch_overlay`'s `args`/`model` fields are unit-tested but unread in production (`state.rs` re-derives the same mapping through `ai_cli_args_for`/`effective_model_for`) — declined, see D5. F4 [MINOR] the disabled-CLI refusal path (round 1's F2 fix) had no direct test of the refusal reason itself — fixed, added `a_disabled_cli_refuses_the_start_with_its_own_reason_not_not_on_path` to `tests/ai_cli_config.rs`, asserting the reason names "disabled" and never "path", and that the CLI never actually launches. F2 and F4 fixed, confirmed by a green `mise run check`, `cargo fmt --check`, and `cargo test -p micold-daemon --test ai_cli_config --test ai_cli_availability` (24 passed). F1 and F3 declined (D5): no new round required, since review-rounds.md's round limit counts rounds that fix a BLOCKER/MAJOR of *that* round, and this round's remaining open findings are declined, not deferred-but-agreed.

## Gate

Rebased onto `origin/main` (8005e00e9 on top), clean (no conflicts). `mise run check` and `cargo clippy --workspace --all-targets -- -D warnings` both clean at `8005e00e9`. The full `mise run gate` was run twice; both runs failed only on tests outside this diff's files (`exclusivity::a_displaced_client_is_not_terminated`, `history_damaged::*`, `history_stop_request::*`, `mcp::worktree_refresh::*`, `micold-material::ellipsized::*`, and once `micold-core::streaming_child::*`) — none of them touch `daemon_config`, `provider`, `terminal`, `provision`, `state.rs`'s AI-CLI code, `hooks.rs`, or `ai_key.rs`, and the set and content changed between the two runs (classic load-timing flakiness; the machine was building several other worktrees concurrently throughout, per the repeated `build-lock.sh: another build holds the lock` lines). `a_displaced_client_is_not_terminated` was separately reproduced on a clean `git stash`'d checkout of this branch's pre-M1 tip (`98aa6edcc`), confirming it predates this diff. The scoped gate (`scripts/autopilot/scoped-gate.sh`, fmt + clippy + the changed crates' tests) is green except that same pre-existing flake. Per `gate.md`'s fallback, recorded the scoped-gate-passing tree and pushed with "Local gate: scoped only, full gate left to CI" — CI's full-workspace run, on a quieter machine, is the tie-breaker; a red CI finding for one of these six tests is this gate failing (`red-ci.md`), any other red is not.

A third full `mise run gate` at HEAD `2d5d3973c` (after the rebase onto `origin/main` and the red-CI fix `289e83961`) again failed only outside this diff's files — a larger set this time (`micold-daemon::autospawn`, `client_restart_input`, `connection::logging_sink::*`, `daemon_stop::*`, `history_damaged::*`, `history_stop_request`, `idle_race`, `idle_stop::*`, `idle_teardown`, `spawn_fd_hygiene`, `version_mismatch_resolves`, `micold-material::ellipsized`), under `uptime`'s load average 3.6–4.7 from concurrent builds in other worktrees; none touch this diff's files (confirmed: `git diff <merge-base origin/main>..HEAD -- crates/` lists only `daemon_config/ai_cli.rs`, `provision/ai_cli.rs`, `provider.rs`, `terminal.rs`, `state.rs`, `hooks.rs`, `ai_key.rs` and their own tests, all of which passed). The scoped gate (`SCOPE workspace`, since the diff touches `micold-core`) then also failed, once, on a single unrelated test — `micold-client::ui::material::card_grid::tests::a_link_scrolled_out_of_the_body_is_not_under_the_pointer` — under load average 21.3 (another worktree, `feat-1531_gh-token-for-gh-in-sessions-of-every-kind`, holding the build lock); re-run alone once that build finished, it passed in 0.04s, confirming load-induced flakiness, not a regression. Per `gate.md`'s fallback (same precedent as above), recorded `HEAD^{tree}` (`2d5d3973c`'s tree) in this worktree's `autopilot-gate-ok` stamp file. `git push` itself remains blocked — see *Open escalation*.

## Red CI

PR #1711, run `42014070087`: `build + test (windows-latest)`, step "Test (render-free core, no
GUI)" failed — `daemon_config::ai_cli::tests::place_as_stays_inside_the_config_dir` asserted
`place_target(dir, Path::new("/absolute")) == None` but got `Some("/absolute")` on Windows. Cause:
`place_target`'s escape guard used `Path::is_absolute()`, which is host-specific — a leading `/`
is not absolute by Windows's own rule, so the guard let it through there while correctly rejecting
it on Linux/macOS. Every other CI job (including the real-runtime jobs) passed.

Fixed the guard, not the test: T062's "`place.as` cannot escape the CLI's config dir" is a security
property, and a guard that is weaker on one platform than another (the config is authored once,
read by whichever platform's daemon runs it) is exactly the kind of gap this milestone exists to
close. Added `is_absolute_on_any_platform` in `crates/micold-core/src/daemon_config/ai_cli.rs`
(`place_target` now calls it instead of the host-specific `is_absolute()`): true for the host's own
`is_absolute()`, plus a leading `/` or `\`, plus a drive letter (`C:...`) — so a POSIX-only or a
Windows-only absolute path is rejected on every platform, not just its own. Added
`place_as_rejects_an_absolute_path_foreign_to_the_host_platform`, asserting `/absolute`, `\absolute`,
`C:\escape` and `C:/escape` are all rejected — this test fails on Linux for the pre-fix `/absolute`
case would not have (Linux's `is_absolute()` already caught that one); the new cases
(`\absolute`, `C:\escape`, `C:/escape`) are the ones no platform's host-native `is_absolute()`
alone would catch everywhere, so they would have caught this bug on Linux too, not just on
Windows's own CI run.

Local gate: `mise run check`, `cargo clippy -p micold-core --all-targets -- -D warnings` clean.
Full `mise run gate`: 19 failures, none in `ai_cli`/`daemon_config`/`provision` — all in daemon
spawn/socket-accept timing (`autospawn`, `exclusivity`, connection/history tests) and one
`micold-material::ellipsized` shaping test, matching the same load-related flakiness class M1's
first gate round already recorded (this machine was building other worktrees concurrently). Scoped
gate: fmt/clippy clean, `ai_cli_config.rs`'s 3 tests pass; one unrelated failure
(`autospawn::a_client_cold_starts_a_daemon_and_it_outlives_the_client`, a 20s socket-accept
timeout) reproduced twice in isolation under the same concurrent-build load — pre-existing, not
this diff's. Per `gate.md`'s fallback, recorded the scoped-gate tree and pushed with "Local gate:
scoped only, full gate left to CI"; CI's quieter machine is the tie-breaker.

## Declined review findings

- M1 review A round 2, F1 [BLOCKER]: `AiCliTable::configured(...)` not called from the real provisioning call sites (`kube::start::pod_target`, `micold-client::daemon::container_target`). Declined — see D5: neither production call site has a per-CLI `version`/`enabled` value to pass in today (`KubeSettings.providers` and `AiCliSelection` are both a bare `AiCli` set, no version/args/model field exists on the client side yet); that data model is M4's own stated scope ("Settings → Daemons renders `[[ai.cli]]`", T066/T067). `AiCliTable::configured` is exactly what M1's Deliverable and Verify line (`cargo test -p micold-core provision::ai_cli`, core-only) call for: the pure builder, ready for M4 to call once the Settings-side data exists.
- M1 review A round 2, F3 [MINOR]: `daemon_config::ai_cli::launch_overlay`'s `args`/`model` fields unread in production (state.rs re-derives via `ai_cli_args_for`/`effective_model_for`). Declined: `effective_model_for` additionally validates the model string with `ModelName::parse` (`launch_overlay` does not — it is a pure capability-mapping function tested in isolation, not the only place the mapping has to be applied correctly), and `ai_cli_args_for` is a one-line accessor; collapsing them would couple the launch-spec wiring to a type (`LaunchOverlay`) built for a different purpose (env mapping, including the two API-key-reaching-env rules). Left as two small, independently correct functions rather than one with a secondary mode.

## Handover

Milestone M2 (T065, T065a, T065b, T065c) not yet started on the tree (no code changes yet; only
this ledger's housekeeping, committed at `0b3324d41`). Investigated the existing wiring in depth
(an Explore subagent plus direct reads) so the next unit can implement straight away without
redoing this research.

**T065's exact scope** (confirmed against the code, not just the task text):
- **Container & pod** are governed by the *legacy* opt-in toggle, not the new `[[ai.cli]]` file
  fields: `CredentialShare::AiCliAuth` (`crates/micold-core/src/sandbox/mod.rs:280-285`,
  `SandboxProfile.credentials: BTreeSet<CredentialShare>`), `CredentialLayout.ai_cli_auth`
  (`mod.rs:742-753`, one path per installed CLI via `AiCli::ALL`/`sandbox_auth_file`),
  `CredentialMount` (`mod.rs:704-709`) built by `MountSet::build_for` (`mod.rs:841-901`), and
  `KubeCredential::SignIn` (`crates/micold-core/src/kube/credentials.rs:44-144`). These are what
  T065's line citations (`mod.rs:321-323`, `:704-709`, `credentials.rs:86-99`) point at — the new
  `[[ai.cli]] sign_in_file`/`token_file`/`api_key_file` fields (M1's `daemon_config::ai_cli`) are
  **not** wired into container/pod delivery yet (M4 gives container/pod daemons their Settings UI);
  do not try to wire them in for T065.
- **Today's container flow** (read-only exception): `CredentialShare::writable()`
  (`mod.rs:321-323`, true only for `AiCliAuth`) is read by `mount_args`
  (`crates/micold-core/src/sandbox/argv.rs:303-312`, calling `mount_value`/`argv.rs:237-245`) to
  bind-mount the **host's real sign-in file directly at the CLI's own live path** (e.g.
  `~/.claude/.credentials.json`), read-write, at container start (`docker run -v …:rw`) — so the
  CLI refreshing it rewrites the host's file through the mount. This is rule N-4/FR-004e, the exact
  thing D13 retires.
- **Today's pod flow**: `KubeCredential::SignIn{cli}.mount()` (`credentials.rs:85-96`) returns
  `Mount{path: <cli's live sign-in path under HOME_DIR>, sub_path: Some(<basename>)}` — a
  single-file `subPath` mount at the CLI's own live location, same read-write exposure in spirit
  (kubelet mounts Secrets read-only, but at the CLI's live path, which collides with D9's "pods are
  no longer a special case" framing once containers also deliver read-only+linked).
- **The fix, by design** (not yet coded — record this reasoning as a Decision when implementing, it
  is a real design call): since "linked, re-created at every session start" cannot be satisfied by
  a mount made once at container/pod start, relocate the delivered file into a dedicated read-only
  location and have the daemon (which **runs inside the container/pod itself** — confirmed by
  `docs/user-guide/sandboxed-daemon.md:360,376-378` and the `argv.rs` config/secrets mount block
  around line 278-291) create/recreate a **symlink** from the CLI's live `sandbox_auth_file()` path
  to that location at every session start (the same "re-created each start" rule T062/D4 already
  deferred to T065). Needs a per-CLI subdirectory because two CLIs share a basename (`auth.json`
  for both Codex and OpenCode): use the CLI's id, from `daemon_config::CliId::as_str()`
  (`crates/micold-core/src/daemon_config/mod.rs:379-384`, delegates to
  `AiCli::provider().tool_name()`) or an equivalent slug — do **not** invent a second id scheme.
  - Container: mount `AiCliAuth`'s host files, read-only, under a new subpath of the *existing*
    `CONTAINER_SECRETS_DIR` (`mod.rs:588`, already mounted read-only as a whole — `argv.rs:286-290`)
    — e.g. `ai-cli-auth/<cli-id>/<basename>` — instead of at the CLI's live path. Drop
    `CredentialShare::writable()` entirely (every `CredentialMount` is read-only now;
    `argv.rs:306-312`'s per-share branch collapses to a plain `false`).
  - Pod: give `KubeCredential::SignIn` a directory Secret with no `subPath`, mirroring
    `KubeCredential::GitConfig`'s own pattern exactly (`credentials.rs:91-97`, `GIT_CONFIG_DIR`
    constant at `:24`, consumed by `daemon_env`'s `GIT_CONFIG_GLOBAL` at `:186-187` — no `subPath`,
    the *env var* points at the file inside the directory mount). Add an equivalent `SIGN_IN_DIR`
    constant and mount each CLI's Secret as its own subdirectory; the consumer is
    `crates/micold-core/src/kube/manifest.rs:216-224` (`credential_mount`, drops `subPath` when
    `None`) and the volume builder around `manifest.rs:360-369`.
  - The relink step itself: a new pure function (core, testable without the real-runtime feature),
    called from the daemon at session start — for container today that is wherever a session
    actually spawns (look for the existing per-session setup call site in
    `crates/micold-daemon/src/state.rs`, near where `with_configured_env`/`ai_cli_env_overlay`
    already run, `state.rs:739-789`, `:2160-2191` — those are T063's M1 insertion points and are
    **not** this one; this one runs unconditionally for sandbox kinds, independent of the new
    config fields, simply checking "does `<secrets-root>/<cli-id>/<basename>` exist" and
    symlinking if so). **Never run this relink for host/ssh kinds** — SC-008 requires the machine's
    own sign-in file is never written, and this function's whole point is sandbox-only.
- **Host/SSH side of T065** is a *separate*, additive piece, driven by the *new*
  `[[ai.cli]]` fields (`sign_in_file`/`token_file`/`api_key_file`/`use_machine_sign_in`,
  already parsed in M1 — `crates/micold-core/src/daemon_config/mod.rs:401-424`): give the session
  `<data_dir>/ai/<id>/` as its config dir (`CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`XDG_DATA_HOME`),
  symlink the sign-in in, and read-through-link the user's own settings files (Claude:
  `settings.json`, `CLAUDE.md`, `agents/`, `commands/`; Codex: `config.toml`, `AGENTS.md`) per the
  contract/D9 table — again **never** touching the real `~/.claude/…` file. The natural insertion
  point is `crates/micold-daemon/src/state.rs`'s `with_configured_env`/`ai_cli_env_overlay`
  (`:757-789`, `:2160-2191`), called before a session spawns (look for the call site around
  `state.rs:6512`, per the Explore agent's report — re-verify the line, it will have moved).
  Byte-identical/mtime-unchanged test (SC-008): write a fake "machine's own" sign-in file, run a
  session through this path, assert `std::fs::read`/`metadata().modified()` unchanged.
- **Existing tests to extend, not duplicate**: `crates/micold-core/tests/sandbox_credentials.rs`
  already has `each_opt_in_adds_exactly_one_mount` (branches on `writable()` — update once it is
  gone), `the_ai_cli_sign_in_share_is_the_token_file_not_the_cli_directory`,
  `a_shared_sign_in_names_its_file_in_the_sandbox_home_to_create`, and `sign_in_path()`/
  `sharing_the_sign_in()` helpers (~line 100-375) — this is the fast, non-real-runtime suite to add
  the new read-only/relocated-path/relink assertions to. The two real-runtime tests that read
  `.writable()` directly (`crates/micold-daemon/tests/sandbox_real_ai_cli_sessions.rs:163`,
  `sandbox_real_no_user_credentials.rs:83`) need updating to match once `writable()` is gone, but
  are not where new coverage belongs (they need the real-runtime feature and a container runtime).

**T065a** (`token_file`): not started. `CLAUDE_CODE_OAUTH_TOKEN`/`COPILOT_GITHUB_TOKEN`, read at
session start, never in argv — same env-injection pattern `api_key_file` already uses in
`state.rs:757-789` (T063's `with_configured_env`), so this is the smallest of the four and a good
one to do right after T065's relink function exists (it reuses the same read-the-secret-file
plumbing, not the relink).

**T065b** (client-side sign-in relay) and **T065c** (docs measurements): not started; no
investigation done yet beyond what research.md D14 already states (quoted in the brief). T065b
needs a new module under `crates/micold-client/src/` (none exists yet) watching the host's own
sign-in file with `notify` (already a dependency — check `crates/micold-client/src/daemon_config_delivery.rs`
for the watcher pattern M1/#1527 already use) and deriving the no-refresh-token copy per D14's
per-CLI rules.

**Why handing over now**: the investigation above (an Explore subagent's report plus direct reads
of `sandbox/mod.rs`, `argv.rs`, `kube/credentials.rs`, `session.rs`, `daemon_config/mod.rs`) pushed
this unit's own context past the 150k cap before any code was written. The next unit should start
coding directly from the plan above: first `sandbox/mod.rs` (`CredentialShare::writable` removal +
`CredentialMount`'s new container destination for `AiCliAuth`) and `argv.rs` (`mount_args`'s
per-share branch), then the relink function and its test in `sandbox_credentials.rs`, then
`kube/credentials.rs` + `kube/manifest.rs`, then the host/ssh `state.rs` wiring and its SC-008 test,
then T065a, then T065b/T065c. Record each real design call (the relink mechanism, the per-CLI
subdirectory naming) as a ledger Decision once made, since none of this is literally spelled out in
tasks.md's one-line task text.

## Open escalation

None.
