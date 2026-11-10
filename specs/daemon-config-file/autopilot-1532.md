# Autopilot ledger — #1532 daemon-config-file (Issue 11)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1532` (AI CLI configuration from the file for every kind); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1527, #1528, #1534 and #1535 (see `autopilot-1527.md` D1); the tasks unit only cuts milestones for the "Issue 11" group (T059–T067 incl. T065a–T065c, T096–T100).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1532
- **Worktree branch**: feat/1532_ai-cli-configuration-from-the-file-for-every-kind
- **Started**: 2026-11-05
- **Phase**: milestone M3
- **Next step**: implement M3 (T096–T100), reviews A and B, gate, PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1699 | design (tasks + milestones for Issue 11) | merged | 98aa6edcc561301f70c671b74113b9877ce2d9fd |
| #1711 | M1 — each `[[ai.cli]]` field maps to the CLI's launch | merged | 8470f7464ec548474530511495dd9ded211e5ba4 |
| #1772 | M2a — sign-in delivery, read-only on every kind (SC-008) | merged | f585c83c829a751556ded4115f3c3275e7834221 |
| #1822 | M2b — keeping a delivered sign-in valid without a CLI refresh | merged | b4216564604c9b93a3f8c685e9a676113f92f6cc |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | each `[[ai.cli]]` field maps to the CLI's launch; configured args/env add to the app's own | T059–T064 | full | #1711 (merged, `8470f7464ec548474530511495dd9ded211e5ba4`) |
| M2a | sign-in delivery, read-only on every kind (SC-008) | T065 | full | #1772 (merged, `f585c83c829a751556ded4115f3c3275e7834221`) |
| M2b | keeping a delivered sign-in valid without a CLI refresh | T065a, T065b, T065c | full | #1822 (merged, `b4216564604c9b93a3f8c685e9a676113f92f6cc`) |
| M3 | gap-review extras: seeded `.claude.json`, onboarding, seeded settings files, `use_machine_sign_in`, sign-in expiry | T096–T100 | full | — |
| M4 | Settings → Daemons renders `[[ai.cli]]`, and the user guide | T066, T067 | full | — |

Split along milestones.md rule 3 (17 tasks, one user story US4, no sub-stories to split on):
M1 is the field-to-launch mapping (tests + implementation together, rule 7); M2 was originally cut
as the read-only sign-in delivery plus the OAuth-refresh replacements together, on the theory that
T065 could not ship without T065a/T065b — split into M2a/M2b by D8 below once two consecutive units
proved that grouping did not fit a unit's context; M3 is the gap-review (research D15) amendments
that depend on M2a's delivery actually existing; M4 holds the UI back until the behaviour it
configures exists (rule 6, no half-wired UI on `main`) and carries the user-guide task with it
(rule 4).

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
| D8 | milestone M2 | Two consecutive units spent their full context on M2's design (T065, T065a–c together) and wrote no code — does the milestone still fit one unit? | No: split M2 into M2a (T065 only — read-only sign-in delivery, SC-008) and M2b (T065a–T065c — keeping a delivered sign-in valid without a CLI refresh), each its own PR. `tasks.md`'s M2 section replaced with M2a/M2b per `milestone-format.md`; M3's `Depends on` repointed at M2b. This unit's own scope is M2a | agent-resolved | this unit's briefing (two 150k handovers with no code, `autopilot-1532.md`'s own prior two *Handover* sections); `specs/daemon-config-file/tasks.md` M2a/M2b sections |
| D9 | milestone M2b | T065a's bullet says Settings runs `claude setup-token`/takes a pasted token and warns 14 days before expiry; T065b's bullet says the client re-delivers a derived sign-in and schedules a host-side refresh. Neither Settings UI for `[[ai.cli]]` nor a client→daemon secret-push message exists yet (both are M4/T066-T067's and M3/T100's own scope) — does M2b need to build them too? | No, same precedent as D4/D5: scope to M2b's own Deliverable/Verify lines, not the task bullets' UI detail. **T065a**: implement the backend only — `token_file` env wiring at session start (never argv) plus a 14-day expiry warning computed server-side and surfaced through the *existing* `ConfigStatus`/daemon-details-dialog diagnostic text (extending the same loop that already shows GitHub `TokenReach` lines, no new widget) rather than a `[[ai.cli]]`-table Settings UI (M4's, not built yet). **T065b**: implement only the pure, unit-tested relay logic in `crates/micold-client/src/ai_cli_relay.rs` (derive-copy-with-no-refresh-token, expiry/scheduling math, an `apply_relay` that writes the derived copy to a given destination path and never touches the source) — exactly what the Verify line (`cargo test -p micold-client --lib`) and the task's own two named tests ask for. The live notify watcher, the per-daemon destination-path lookup and the actual host/SSH/pod delivery channel are left as named follow-up: no client→daemon protocol message exists to push a secret (confirmed by exploration), the client has no structured read of a target daemon's `[[ai.cli]]` config yet (that is M4's), and pod re-delivery on a host sign-in change is already M3's own stated Deliverable. State this gap plainly in the M2b PR body | agent-resolved | exploration of `ConfigStatus`/`TokenReach` (`crates/micold-core/src/protocol/messages.rs:2433-2506`, `crates/micold-daemon/src/config_watch.rs:123-143`, `crates/micold-client/src/ui/daemon_details_dialog.rs:60-85`) and of client-side watch/scheduling/protocol building blocks (`crates/micold-client/src/shell/changes_watch.rs`, `crates/micold-client/src/shell/subscriptions.rs:286`, `crates/micold-client/src/daemon_config_delivery.rs`) found no existing client→daemon secret-push message and no client-side `[[ai.cli]]` config awareness; `specs/daemon-config-file/tasks.md` M4 line (T066/T067, Settings → Daemons renders `[[ai.cli]]`) and M3 line (T100, "the client re-delivers the pod Secret when the host sign-in changes") |

## Review rounds

Milestone M2b review A (autopilot-reviewer, code-review high): CHANGES round 1 - F1 MAJOR derived copy world-readable/non-atomic: fixed (owner_only::write, 0600/0700, tested); F2 MAJOR dest==source/symlink could overwrite the user's own sign-in: fixed (same_file check, InvalidInput, tested); F3 MAJOR panicking SystemTime add on huge exp: fixed (checked_add, tested); F4 MINOR unexpected JSON shape relayed with refresh token: fixed (RelayError::UnexpectedShape, tested); F5 MINOR status path dropped a token_file read failure silently: fixed (tracing::warn). Uncommitted validate.rs diff (key_variables reuses token_file_env_variable) was a behaviour-neutral refactor belonging to T065a; committed. Review B snapshot `5a5f5daa313c2bd8306c8b864b9afb7ff8eda215:23a74afd8b86554df58f86469e4fb534596f8470` (sonnet): CHANGES. F1 MAJOR cycle-log has no red phase for T065a/b: declined, the log records `TEST_AFTER` openly as the earlier unit's deviation and each review-A fix was proven by a temporary revert; the history cannot be rewritten. F2 MAJOR T065c measurements absent: fixed, annotated in tasks.md and stated in the PR body (no CLIs or container runtime here; the doc table is honest). F3 MINOR T065b tick read as full: fixed, annotated in tasks.md. F4 MINOR `refreshTokenExpiresAt` left in the Claude copy: declined, an expiry timestamp is not a secret. All prose/annotation fixes, no new round.

Tasks review A round 1 (snapshot `08da6558cfb79253faee3b170a8c04b4666d9efa:0144800f6c91e1ce455e0ed636ba5c517d601826`): CHANGES. F1 [MAJOR] T096/T097/T099/T100 named no failing test — fixed, each now leads with "Failing test in `<file>`". F2 [MAJOR] M3's Verify line missed a command for T100's pod-Secret re-delivery — fixed, added `cargo test -p micold-daemon --test config_reload` and `cargo test -p micold-core kube::credentials`. F3 [MINOR] FR-040 looked fully satisfied in M2's Satisfies line — fixed, M2's Deliverable and Satisfies note it is only partial there (T065a's expiry warning), full `expiresAt` display is M3's T100. All three fixed, no new round needed if the fix diff holds (review-rounds.md round 2).

Tasks review A round 2 (sonnet, fix diff only; snapshot `a0459e0031fc4415dd96b496c1c308ca07e11886:c8d8ea65b9e5a5d8c25228f0383c1c912b0538a7`): CLEAN. All three round 1 fixes confirmed to hold; both new Verify targets (`config_reload.rs`, `kube::credentials`) confirmed to exist. (Earlier snapshot `3ad4fced4370a500be17fe9f2d2a4553dfbaba24:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3` went stale when the branch rebased onto `origin/main` past 11 new commits, one of which touched `tasks.md`'s milestones heading line; resolved with no content loss, see note below.)

Milestone M1 review A round 1 (code-review skill equivalent, general-purpose agent, high effort; no snapshot — pre-commit working tree): CHANGES. F1 [BLOCKER] two `StatusContext` literals (`tests/mcp/worktree_guard_sessions.rs`, `tests/settings/plan_usage_setting.rs`) missed the new `api_key_helper` field — fixed, both literals now carry `api_key_helper: None`. F2 [MAJOR] a config-disabled CLI's start refusal reused the "not on PATH" sentence, misleading when the CLI is actually installed — fixed in `state.rs`'s `start_session_gated`: `ai_cli_disabled` is checked first and gets its own reason ("… is disabled in the daemon's configuration"). No other findings. All fixed, confirmed by a green `mise run check`, `cargo clippy --workspace --all-targets -D warnings`, and the scoped gate (one unrelated pre-existing failure, `exclusivity::a_displaced_client_is_not_terminated`, reproduced identically on `git stash`'s clean `98aa6edcc` baseline — not this diff's).

Milestone M1 review A round 2 (sonnet, full diff — round 1 took no snapshot): CHANGES. F1 [BLOCKER] `AiCliTable::configured(...)` (the `version`/`enabled` override, T061) is never called from the real provisioning call sites (`crates/micold-core/src/kube/start.rs`'s `pod_target`, `crates/micold-client/src/daemon.rs`'s `container_target`) — declined, see D5. F2 [MAJOR] `launch_effort`'s model lookup used the session's raw, pre-override model while the sibling `model` field in the same `LaunchSpec` used `effective_model_for`'s overridden one, so an effort level could pass unfiltered for a model that is not what actually launches — fixed at both `LaunchSpec` construction sites (start and respawn paths) in `state.rs`: the effective model is now resolved once and fed to both `launch_effort` and `model`. F3 [MINOR] `launch_overlay`'s `args`/`model` fields are unit-tested but unread in production (`state.rs` re-derives the same mapping through `ai_cli_args_for`/`effective_model_for`) — declined, see D5. F4 [MINOR] the disabled-CLI refusal path (round 1's F2 fix) had no direct test of the refusal reason itself — fixed, added `a_disabled_cli_refuses_the_start_with_its_own_reason_not_not_on_path` to `tests/ai_cli_config.rs`, asserting the reason names "disabled" and never "path", and that the CLI never actually launches. F2 and F4 fixed, confirmed by a green `mise run check`, `cargo fmt --check`, and `cargo test -p micold-daemon --test ai_cli_config --test ai_cli_availability` (24 passed). F1 and F3 declined (D5): no new round required, since review-rounds.md's round limit counts rounds that fix a BLOCKER/MAJOR of *that* round, and this round's remaining open findings are declined, not deferred-but-agreed.

Milestone M2a review A round 1 (code-review skill, high effort, origin/main...HEAD): CLEAN with two MINOR findings, both fixed in fc2bb6994 (doc-comment only, no new round needed). F1 [MINOR] crates/micold-core/tests/sandbox_argv.rs - commit a65b7fef0 had overwritten the tail of a doc clause, dropping "still covers by asserting every container half is a Linux path." - fixed, line restored from origin/main and the T065 paragraph separated with a /// break. F2 [MINOR] crates/micold-core/src/kube/credentials.rs:47 - Mount::sub_path is never constructed as Some(..) anywhere in crates/, so manifest.rs:223's branch is dead - fixed as a doc note explaining why a single-file subPath mount cannot be rewritten by a CLI's own token refresh, and that the manifest still renders the field (removing the field is out of T065's scope). Both confirmed by a green cargo fmt --check -p micold-core and cargo check -p micold-core --all-targets.

Milestone M2a review B (conformance, sonnet, background dispatch): CHANGES on round 1 - F1 [MAJOR] specs/daemon-config-file/tdd/cycle-log.md had no Issue 11 M2a (T065) entry - fixed, prose only: appended a TEST_AFTER entry (baseline, what was removed, each delivery path's green tests, the full-gate result) to specs/daemon-config-file/tdd/cycle-log.md. No new round (review-rounds.md's prose-only exception): the fix adds no code, test or behaviour claim, only documents tests and results review B itself already ran and reported. F2 [MINOR, informational] noted the Verify commands were still compiling at report time; on its second pass review B ran both to completion itself: cargo test -p micold-core sandbox::mod -- credential (103 passed) and cargo test -p micold-core kube::credentials (7 passed, including the_pod_relink_targets_claudes_own_sign_in_directory). Scope, ownership and task-content checks all passed: CredentialShare::writable fully removed, all three sandbox kinds (container, pod, host/SSH) covered, user guide updated, no leftovers, no M2b work leaked in.

## Gate

M2b: full `mise run gate` (MICOLD_DAEMON_CONFIG unset) green on the rebased tree (an earlier run failed only on the unrelated flaky `structured_session::an_exit_mid_turn_ends_the_turn_with_a_note` and a still earlier one went red only on this diff's own new test, an over-strict Claude `u64::MAX` ms assertion, fixed).

Rebased onto `origin/main` (8005e00e9 on top), clean (no conflicts). `mise run check` and `cargo clippy --workspace --all-targets -- -D warnings` both clean at `8005e00e9`. The full `mise run gate` was run twice; both runs failed only on tests outside this diff's files (`exclusivity::a_displaced_client_is_not_terminated`, `history_damaged::*`, `history_stop_request::*`, `mcp::worktree_refresh::*`, `micold-material::ellipsized::*`, and once `micold-core::streaming_child::*`) — none of them touch `daemon_config`, `provider`, `terminal`, `provision`, `state.rs`'s AI-CLI code, `hooks.rs`, or `ai_key.rs`, and the set and content changed between the two runs (classic load-timing flakiness; the machine was building several other worktrees concurrently throughout, per the repeated `build-lock.sh: another build holds the lock` lines). `a_displaced_client_is_not_terminated` was separately reproduced on a clean `git stash`'d checkout of this branch's pre-M1 tip (`98aa6edcc`), confirming it predates this diff. The scoped gate (`scripts/autopilot/scoped-gate.sh`, fmt + clippy + the changed crates' tests) is green except that same pre-existing flake. Per `gate.md`'s fallback, recorded the scoped-gate-passing tree and pushed with "Local gate: scoped only, full gate left to CI" — CI's full-workspace run, on a quieter machine, is the tie-breaker; a red CI finding for one of these six tests is this gate failing (`red-ci.md`), any other red is not.

A third full `mise run gate` at HEAD `2d5d3973c` (after the rebase onto `origin/main` and the red-CI fix `289e83961`) again failed only outside this diff's files — a larger set this time (`micold-daemon::autospawn`, `client_restart_input`, `connection::logging_sink::*`, `daemon_stop::*`, `history_damaged::*`, `history_stop_request`, `idle_race`, `idle_stop::*`, `idle_teardown`, `spawn_fd_hygiene`, `version_mismatch_resolves`, `micold-material::ellipsized`), under `uptime`'s load average 3.6–4.7 from concurrent builds in other worktrees; none touch this diff's files (confirmed: `git diff <merge-base origin/main>..HEAD -- crates/` lists only `daemon_config/ai_cli.rs`, `provision/ai_cli.rs`, `provider.rs`, `terminal.rs`, `state.rs`, `hooks.rs`, `ai_key.rs` and their own tests, all of which passed). The scoped gate (`SCOPE workspace`, since the diff touches `micold-core`) then also failed, once, on a single unrelated test — `micold-client::ui::material::card_grid::tests::a_link_scrolled_out_of_the_body_is_not_under_the_pointer` — under load average 21.3 (another worktree, `feat-1531_gh-token-for-gh-in-sessions-of-every-kind`, holding the build lock); re-run alone once that build finished, it passed in 0.04s, confirming load-induced flakiness, not a regression. Per `gate.md`'s fallback (same precedent as above), recorded `HEAD^{tree}` (`2d5d3973c`'s tree) in this worktree's `autopilot-gate-ok` stamp file. `git push` itself remains blocked — see *Open escalation*.

M2a's first full `mise run gate` (HEAD `bf94af3f4` plus the uncommitted test fixes) failed to compile: `sandbox_credentials.rs`'s two relink tests used `sandbox::relink_pairs`, which T065's commit never added. Fixed in `55f3eab6c` (the pair-filtering loop of `relink_ai_cli_sign_ins`, extracted as a `pub` seam so a test can point `delivered` at a writable tempdir instead of the real `/run/micold` paths).

M2a's second full `mise run gate` (12645 tests, 24 failed): fmt and clippy clean, and the failures split cleanly into this diff's and not.

**This diff's, fixed (4).** `micold-core::main sandbox_argv::{a_unix_host_mounts_every_path_at_itself, a_windows_host_mounts_every_path_under_the_container_root, git_routes_to_the_daemon_exactly_when_the_mounted_paths_differ, only_the_ai_cli_sign_in_is_mounted_writable}`. All four encoded the pre-T065 mount contract: that each CLI's sign-in is mounted *at its own live path* (so an identity mount on Linux, a mapped one under `/mnt/host` on Windows) and *writable*. T065 reverses both halves deliberately — the sign-in now lands read-only at a fixed container path under `CONTAINER_SECRETS_DIR/ai-cli-auth/<suffix>`, and the sandbox symlinks the live path to it at session start, so the CLI's own refresh writes a fresh file in its home instead of writing through a host mount. Fixed by teaching `mapped_volumes` that a `CONTAINER_SECRETS_DIR` container half is a *fixed* path like the state directory's and the token's (so it is not a mapping candidate on any platform, Linux included), which also fixes the git-routing test that compared "do any mounted halves differ" against `is_identity_for`; and by rewriting the writable test as `no_credential_is_mounted_writable_not_even_the_ai_cli_sign_in`, which asserts every credential mount is `ro` *and* that each sign-in's container half is under the secrets directory (so it still asserts something about sign-ins in particular, not just "everything is ro"). All 12 `sandbox_argv` tests then pass.

**Not this diff's, proven (20).** `micold-core::main streaming_child::unix::a_natural_exit_still_ends_what_it_left_running` passed on the first isolated re-run (0.04 s) — load-induced, the class the three M1 gate rounds above already record. The 15 real-daemon tests (`micold-daemon::{autospawn, client_restart_input, connection::logging_sink (2), daemon_stop (2), history_stop_request, idle_race, idle_stop (3), idle_teardown, spawn_fd_hygiene, version_mismatch_resolves}`) all failed on their 10/20/30 s socket-accept or shutdown timeouts, with five other worktrees (`feat-1536`, `feat-1530`, `chore-1537`, `feat-1703`, `feat-1531`) holding the build lock in turn throughout — the same set and the same timeout signature as the second and third M1 gate rounds, which ran on trees that contained no T065 code at all. The remaining five fail fast and so are *not* timing, and were checked properly rather than assumed: `micold-daemon::history_damaged::{a19_a20, a21, a22, a24}` (0.35 s, each failing on one extra `WARN micold_daemon::state "env-include resolution did not succeed"` beyond the one asserted) and `micold-material ellipsized::tests::real_shaping_stops_at_the_last_character_that_fits` (0.01 s, a font-shaping width claim). Both were re-run on a clean detached `origin/main` worktree (`18a4aa800`, checked out under this unit's scratchpad as `mainwt`): **identical failures there** — the same four `history_damaged` tests with the same extra `env-include` warning, and the same `ellipsized` panic with the same two strings. Both are therefore pre-existing and host-environment-dependent (the machine has no env-include script resolvable from the test's cwd, and its installed fonts shape that string differently), not this diff's. The warning in particular cannot be this diff's: it is raised in `env_include_vars_for`, which the diff does not touch, and the diff's two new `state.rs` entry points both return before reaching it on the default host kind (`relink_ai_cli_sign_ins_before_launch` is a no-op unless the daemon kind is Container or Kubernetes; `with_host_config_dir` returns at its first `let … else` without a configured `sign_in_file`).

M2a's third full gate, rebased onto `origin/main` (`18a4aa800`, clean rebase of 11 commits) and run as `env -u MICOLD_DAEMON_CONFIG mise run gate` at HEAD `c1c18b5b1`: fmt and clippy clean, **12656 tests run, 11 failed** — down from 24, because unsetting `MICOLD_DAEMON_CONFIG` fixed 13 of them. All 12 `sandbox_argv` tests pass, so this diff's own four failures are closed.

**Every one of the remaining 11 is proven pre-existing on clean `origin/main`.** The same five already proven above, plus the six re-checked this round, each re-run from the detached `origin/main` worktree (`18a4aa800`) with the same `env -u MICOLD_DAEMON_CONFIG`:

| Test | On this branch | On clean `origin/main` |
|---|---|---|
| `micold-daemon::client_restart_input a_restarted_client_drives_a_session_it_did_not_start` | FAILED ("the seeded session is in the snapshot") | FAILED, same panic |
| `micold-daemon::connection logging_sink::an_inherited_journal_stream_does_not_silence_the_log_file` | FAILED | FAILED |
| `micold-daemon::fatal_startup_is_logged fatal_startup_error_reaches_the_log_file` | FAILED (20 s, "still running after 20s") | FAILED, same |
| `micold-daemon::history_stop_request a_signal_stops_a_real_service_within_five_seconds…` | FAILED | FAILED |
| `micold-daemon::idle_stop an_idle_stop_writes_one_line_naming_inactivity…` | FAILED (got 0 lines, wanted 1) | FAILED, same |
| `micold-daemon::idle_stop a_killed_daemon_leaves_no_stop_line_behind` | FAILED | FAILED |
| `micold-daemon::history_damaged {a19_a20, a21, a22, a24}` | FAILED (extra `env-include` WARN) | FAILED, same extra WARN |
| `micold-material ellipsized::real_shaping_stops_at_the_last_character_that_fits` | FAILED | FAILED, same two strings |

So the gate is green apart from 11 failures this flow neither caused nor owns: they are real-daemon log/spawn/shutdown tests and one font-shaping test, all dependent on the host environment this nested sandbox provides (no journald stream, no resolvable env-include script, different installed fonts). They are left alone deliberately, per the orchestrator's instruction, rather than papered over here.

The scoped gate (`SCOPE workspace`) at the same HEAD confirms it from the other direction: fmt and clippy clean and **every test passes except that one `client_restart_input` failure** — 3345 `micold-client` tests, 1059 and 793 in the core and daemon libs, and all of `micold-core`'s integration tests green.

Per `gate.md`'s "let CI validate instead" fallback (the same precedent M1 and the red-CI fix above used on this branch), recorded `HEAD^{tree}` in this worktree's `autopilot-gate-ok` and pushed with `Local gate: scoped only, full gate left to CI` in the PR body. CI runs on a machine with none of these three environment gaps, so it is the tie-breaker: a red CI naming any test *other* than these 11 is this gate failing (`red-ci.md`).

Not run locally, both noted in the PR body for CI to cover: `mise run image` + `mise run test-sandbox` (T065 does change how a container session mounts its sign-in, but no container runtime is installed here — docker, podman and nerdctl are all absent, so CI's sandbox job runs it), and the macOS cross-check (`aarch64-apple-darwin` is not an installed rustup target; the diff's only three new `cfg(target_os = …)` uses are `#[cfg(target_os = "linux")]` gates on new symlink *tests* in `sandbox_credentials.rs`, which turn tests off on macOS rather than changing any production arm).

Note for the user: the `mainwt` worktree under this unit's scratchpad was created only for the clean-`origin/main` proofs and is no longer needed — `gate-hook.sh` forbids an agent removing a worktree, so please remove it in micold IDE.

M2a's fourth full gate, at the final HEAD before the M2a PR (a630e05ff, env -u MICOLD_DAEMON_CONFIG mise run gate): fmt and clippy clean, 12656 tests run, 11 failed - the identical 11 already proven pre-existing on a clean detached origin/main worktree (client_restart_input, logging_sink, history_damaged x4, fatal_startup_is_logged, history_stop_request, idle_stop x2, ellipsized real_shaping). None of this milestone's own tests (all 12 sandbox_argv, the sandbox_credentials relink tests, kube::credentials) are among them. GATE_EXIT=100 only because the task fails the run on any failed test; the exit code is not itself evidence of a regression once the failing set is checked against the known baseline.

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

### Red CI round 2 (attempt 1/3): PR #1772, sandbox real-runtime compile break

PR #1772, run `42413909844`, job "sandbox against a real runtime (linux)" (`ci complete` also red
as a consequence): **FAILED TO COMPILE**, not a behavioural failure as this milestone's own
Handover had predicted (it worried about `sandbox_real_ai_cli_sessions.rs` and
`sandbox_real_no_user_credentials.rs` needing to call `relink_ai_cli_sign_ins_before_launch`
themselves — checked that against this log and it does not apply here). The real cause:
`crates/micold-daemon/tests/sandbox_real_scoped_token.rs:75,85` called
`ClientMsg::GitCredentialGrant`/`GitCredentialRevoke`, two wire variants that commit `5b9bd6b08`
(#1530, "retire the git credential grant messages") removed from
`crates/micold-core/src/protocol/messages.rs` on **origin/main itself**, before #1532 branched
(`git merge-base origin/main HEAD` == `origin/main`'s own tip). #1530 deleted the other two tests
that drove the grant/revoke path (`git_credential_grant.rs`, `git_credential_pod.rs`) but missed
this one, because `sandbox_real_scoped_token.rs` only compiles under the `sandbox-real-runtime`
feature, which the normal gate never builds — so nobody caught it until a real CI run with a
container tried to. Not a regression of this milestone's own diff; still blocks this PR's merge.

Fixed the code's break, not by patching the test to match a message that no longer exists: deleted
`sandbox_real_scoped_token.rs` and its `[[test]]` entry in `crates/micold-daemon/Cargo.toml`. The
config-file credential path it tested (`#1530`'s own replacement) is already covered by
`crates/micold-daemon/tests/git_credential_config.rs` and `sandbox_real_no_user_credentials.rs`;
`support/git_http.rs` stays (still shared with `mcp/scoped_git_credential.rs`), its doc comment
updated to drop the dead reference. Added a note to
`specs/1002-sandbox-scoped-token/quickstart.md` flagging that its steps 4-8/10 table still names
the retired test and the also-deleted `micold-client::shell::scoped_credential` tests — stale, but
untangling that table is follow-up, not #1532's.

No container runtime is installed in this worktree (no `docker`/`podman`), confirmed by `which`;
`mise run image && mise run test-sandbox` cannot run here, so the fix rests on CI's
`sandbox-runtime` job for real-runtime verification. Locally confirmed instead:
`scripts/build-lock.sh cargo check -p micold-daemon --tests --features sandbox-real-runtime` exits
0 (the actual compile error is gone). Full `mise run gate` (run twice, both times contended by
another worktree's build on the shared lock): 19 then 12 failures, all in daemon spawn/socket-
accept timing (`client_restart_input`, `connection logging_sink`, `daemon_stop`, `history_damaged`,
`history_stop_request`, `idle_stop`/`idle_race`/`idle_teardown`, `fatal_startup_is_logged`,
`streaming_child`, `version_mismatch_resolves`, `spawn_fd_hygiene`) and one
`micold-material::ellipsized` shaping test — none touching `sandbox_real_scoped_token`,
`ai_cli`, `daemon_config`, `Cargo.toml`'s test registration, or anything else in this diff; the
same load-related flakiness class already recorded above for M1/M2a's own gate rounds (also
confirmed `MICOLD_DAEMON_CONFIG` was set in this shell's environment, the documented false-failure
cause from this milestone's Handover note; `env -u MICOLD_DAEMON_CONFIG` cut the failure count from
19 to 12, not to 0 — the rest is the build-lock contention this round also hit, including from a
second worktree, `fix-sandbox-scoped-token-test-stale-clientmsg-variants`, independently fixing the
same `#1530` gap). Scoped gate also ran under contention and hit one of the same flakes
(`client_restart_input`). Per `gate.md`'s fallback, recorded the tree
(`7008388c04321b87189f4453532795555eda15bf`) in `autopilot-gate-ok` and pushed with "Local gate:
scoped only, full gate left to CI".

Pushed commit `56cc12131` to PR #1772's existing branch (no new PR: red-ci.md keeps the fix on the
open PR). Waiting on CI.

### Red CI round 2 (attempt 2/3): PR #1772, the sandbox real-runtime behavioural failure

Run `42475801440`, job "sandbox against a real runtime (linux)": every other real-runtime test
passed; `sandbox_real_no_user_credentials_reach_the_container` failed, panicking at
`sandbox_real_support/mod.rs:308` — `docker start` failed because runc could not create the
mountpoint for `/run/micold/secrets/ai-cli-auth/.claude/.credentials.json`: "read-only file
system". This is the gap the M2a Handover had predicted, but not for the predicted reason (the
test does not need to call `relink_ai_cli_sign_ins_before_launch` itself).

**Root cause (a genuine T065 production bug, not a test-only artifact):** a credential's
`delivered` destination sits *inside* `CONTAINER_SECRETS_DIR`, which both the test harness
(`start_sandbox`'s `secrets_mount`) and production (`argv.rs`'s `delivery.secrets` mount) already
bind-mount there, read-only, as one whole directory, earlier in the same mount list. Nothing
anywhere pre-created the nested `ai-cli-auth/...` mountpoint inside that directory before it went
read-only, so runc's "create mountpoint" step failed exactly as CI showed — a real session start
sharing `ai_cli_auth` would hit the identical failure against a real runtime, confirmed by reading
`MountSet::build` (`crates/micold-core/src/sandbox/mod.rs`), `argv.rs`'s mount loop, and
`daemon_config_delivery::deliver_container` (which only `ensure_dir`s `delivery.secrets` and writes
`startup.env`, never an `ai-cli-auth` placeholder).

**Fix**, mirroring `MountSet::home_files_to_create`'s existing reason (pre-create a mount target so
the runtime never has to), commit `ba6ae6b5e`:
- `crates/micold-core/src/sandbox/mod.rs`: new `MountSet::secrets_files_to_create()`, beside
  `home_files_to_create`, returning `delivery.secrets.join(<delivered's suffix after
  CONTAINER_SECRETS_DIR>)` for every credential with `delivered` set.
- `crates/micold-client/src/shell/sandbox.rs`: calls it right after the existing
  `home_files_to_create` loop, creating each file's parent dir and the (empty, owner-only) file
  before the container is created.
- `crates/micold-daemon/tests/sandbox_real_no_user_credentials.rs` and
  `sandbox_real_ai_cli_sessions.rs`: both call `mounts.secrets_files_to_create()` themselves before
  `start_sandbox`, since they build `mounts` by hand rather than through
  `micold_client::shell::sandbox`.

No container runtime in this worktree (confirmed, `docker`/`podman` absent), so this fix's real
exercise rests on CI's `sandbox against a real runtime (linux)` job. Verified locally instead:
`cargo check -p micold-daemon --tests --features sandbox-real-runtime`, `cargo check --workspace`,
`cargo clippy -p micold-daemon --tests --features sandbox-real-runtime -- -D warnings` (the
default workspace clippy never compiles these `cfg`-gated test files, so this is a separate,
necessary check), and `cargo fmt --all -- --check` — all clean.

Full `mise run gate` (tree `ba6ae6b5e`'s): fmt and clippy clean; `cargo nextest run --workspace`
(`GATE_EXIT=100`) failed 22 tests, every one in the already-documented
`MICOLD_DAEMON_CONFIG`/journald-nesting family this milestone's own Handover named
(`client_restart_input`, `connection::logging_sink`, 4×`history_damaged`,
`fatal_startup_is_logged`, `history_stop_request`, 2×`idle_stop`) plus the pre-existing
`micold-material::ellipsized` shaping test — none in `sandbox`, `ai_cli`, `daemon_config` or this
diff's 4 changed files, and all reproduced running this nested dev session rather than CI's
`build + test (ubuntu-latest)` (confirmed green at run `42442505836`, commit `18a4aa800`). Per
`gate.md`'s fallback (environment-caused, not this diff's), recorded the gate-ok tree marker and
pushed with the exception noted in the PR body.

Pushed commit `ba6ae6b5e` to PR #1772's existing branch. Waiting on CI for attempt 2's real
verdict; attempt 3 is this scope's last before an escalation.

## Declined review findings

- M1 review A round 2, F1 [BLOCKER]: `AiCliTable::configured(...)` not called from the real provisioning call sites (`kube::start::pod_target`, `micold-client::daemon::container_target`). Declined — see D5: neither production call site has a per-CLI `version`/`enabled` value to pass in today (`KubeSettings.providers` and `AiCliSelection` are both a bare `AiCli` set, no version/args/model field exists on the client side yet); that data model is M4's own stated scope ("Settings → Daemons renders `[[ai.cli]]`", T066/T067). `AiCliTable::configured` is exactly what M1's Deliverable and Verify line (`cargo test -p micold-core provision::ai_cli`, core-only) call for: the pure builder, ready for M4 to call once the Settings-side data exists.
- M1 review A round 2, F3 [MINOR]: `daemon_config::ai_cli::launch_overlay`'s `args`/`model` fields unread in production (state.rs re-derives via `ai_cli_args_for`/`effective_model_for`). Declined: `effective_model_for` additionally validates the model string with `ModelName::parse` (`launch_overlay` does not — it is a pure capability-mapping function tested in isolation, not the only place the mapping has to be applied correctly), and `ai_cli_args_for` is a one-line accessor; collapsing them would couple the launch-spec wiring to a type (`LaunchOverlay`) built for a different purpose (env mapping, including the two API-key-reaching-env rules). Left as two small, independently correct functions rather than one with a secondary mode.

## Handover

None. (M2b code is committed and reviewed; the previous unit's plan is spent. Next: wait for the M2b PR's CI and merge, then M3.)

## Open escalation

None.
