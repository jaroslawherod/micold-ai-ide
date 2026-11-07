# Autopilot ledger — #1530 daemon-config-file (Issue 9)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1530` (The daemon's credential helper chooses the token from the configuration); labels: `effort:high`, `flow:feature`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1529 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-008, FR-010, FR-016, FR-030, FR-031, FR-035, `contracts/daemon-config.md` sections `[git]` and `[[github.tokens]]`); the tasks unit only cuts milestones for the "Issue 9" group (T046–T054, T092–T094).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1530
- **Worktree branch**: feat/1530_the-daemon-s-credential-helper-chooses-the-token
- **Started**: 2026-11-04
- **Phase**: M3 (cut by Close)
- **Next step**: implement M3 (T139), then a new Close unit does T140-T147 from tdd/verification.md (Issue 9)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1686 | Design (milestones for Issue 9) | MERGED | 826681cfb63664b82f65026abfd61abb190956c5 |
| #1700 | M1 | MERGED | 3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f |
| #1713 | M2 | MERGED | 86468d87ced75974d0aeca37c7d7675976d1a934 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T046–T052, T093, T054 | full | helper chooses token from config (most specific, ties first), reach cached by fingerprint, per-kind session git env with fallback/clear, per-host rewrite | #1700 | merged |
| M2 | T092, T094, T053 | full | git setup from daemon.kind on every start path, GIT_CONFIG_GLOBAL, gpgsign stripped, grant messages retired + protocol bump | #1713 | merged |
| M3 | T139 | full | container and pod git config only from `[git] config_file`: remove `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount (T094 remainder, I9-2) | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1530? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-008/010/016/030/031/035. Skipped as for #1524/#1525/#1526/#1529/#1534. Blockers #1524 and #1529 are merged (#1529 via PR #1684). | agent-resolved | specs/daemon-config-file/autopilot-1529.md |
| D2 | tasks | Split Issue 9? | Yes: 11 tasks, two stories (rule 3). M1 = helper + env (T046–T052, T093) with T054 guide (rule 4); M2 = kind gating, git config file, retirement T053 (wire change reviewed alone). Both full: security check, sandbox boundary, wire protocol. | agent-resolved | milestones.md |
| D3 | close | Unbuilt T094 half becomes M3? | Yes (close rule 2): T094 ticked, its GitConfig/kube Secret replacement deferred. Test-strength findings I9-1, I9-3..I9-10 (T140-T147) stay for the next Close; mutation testing was not run (build-lock queue), T147 does it. | agent-resolved | tdd/verification.md Issue 9 |

## Review rounds

- M1: review A (code-review high) 10 findings, 4 fixed (host reset now per forge, fallback on unreadable config, absolute config path, token file leading blank line); review B (sonnet) CLEAN, 3 MINOR (F1 fixed). Snapshot 503b13b8:826681cfb.

- M2: review A (code-review high) CLEAN, 3 MINOR (F2 fixed, F1 noted in Follow-ups); review B (sonnet) F1 MAJOR (no cycle-log entry) fixed, F2/F3 MINOR noted. Rebased onto origin/main afterwards; full gate re-run on the rebased tree. A Close unit follows this last milestone.

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks | 1 | fd03d24b9bf4c7aac5fd8b2e77207878cc688dbc:259bcb30e95d60c215c8bba19b45fac7123e8c07 | CHANGES: F1 MAJOR (no guide task in M2) fixed by writing docs into T094; F2 MINOR git_credential_grant.rs added to T053; F3 MINOR noted: matching.rs and git_credential_config.rs are created by T046/T047, no `TokenChoice` exists yet. Fixes are task-text only, no new round |

## Declined review findings

- M1 review A: reach listing capped at 30 pages, URL-form GHE host gets no `/api/v3`, fallback capture uses the daemon's cwd and exact keys, `run_fallback` has no timeout and forwards only `get`, `UreqTransport` duplicates the client's, legacy `GIT_CONFIG_KEY_n` leftovers: accepted for M1 (MINOR in effect; listed in Follow-ups). Review B MINORs F2 (cache lock), F3 (capture cwd): same.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M3 (T139) in progress, uncompiled WIP committed locally (nothing built: the shared build lock was held by other worktrees' gates for 1 h+; no red run exists, tests were written first in the same working tree, see below). Not pushed, no PR.

Design chosen: no opt-in any more. The app always delivers the sanitized `~/.gitconfig` (new `micold_core::scoped_credential::shared_copy(path) -> Result<Option<String>,String>`): container -> `delivery.secrets/gitconfig`, `[git] config_file = /run/micold/secrets/gitconfig` (`deliver_container` got a `git_config: Option<&str>` param before `startup`); pod -> key `gitconfig` of the `<name>-startup` secret (`StartRequest.git_config`, `manifest::STARTUP_GIT_CONFIG_FILE`), `[git] config_file = /run/micold/secrets/startup/gitconfig`. `CredentialShare::GitConfig` and `KubeCredential::GitConfig` removed; stored `git_config` entries are dropped silently (`RawShare::Retired`, `RetiredKubeCredential`); `names.retired_secrets()` is now 3 (adds `-gitconfig`); `holds_retired_credential` recreates a pod whose stamp allows `git_config`; `daemon_env()` lost its param and the `GIT_CONFIG_GLOBAL` insert.

Done (edited, never compiled): core `sandbox/mod.rs`, `sandbox/argv.rs` tests, `kube/{credentials,names,manifest,start,start_tests,lifecycle,settings}.rs`, golden `tests/golden/kube/pod_spec.json`, `scoped_credential/{gitconfig,mod}.rs` (+ test); client `daemon.rs`, `daemon_config_render.rs`, `daemon_config_delivery.rs`, `shell/sandbox.rs` (share_sanitized_git_config replaced by `sanitized_git_config(facts)`, test rewritten), `tests/daemon_config_render.rs`; daemon `tests/sandbox_real_support/mod.rs` (extra `None` arg).

Next: run `scripts/build-lock.sh cargo check --workspace --tests` (or `mise run check`) and fix what is left of `GitConfig`: `crates/micold-client/src/features/daemons_kube.rs` (AllowedKind::GitConfig, drop it), `ui/settings/container_fields.rs` tests (~689-725 use GitConfig; use AiCliAuth-only cases), `features/daemon_sections.rs`, tests `daemons_feature.rs:364,381,626,643`, `features_daemon_sections.rs:256`, `features_settings.rs:135,142`, `features_daemons.rs:1228`, `settings_rail_motion.rs:1065`, `upgrade_single_daemon.rs:167`, `support/covered_states.rs:1870,1951`, core `tests/sandbox_credentials.rs` (101-218, 328, 518, 687-695), `tests/settings_roundtrip.rs:377`, daemon `tests/settings/catalog_adoption.rs:439,463`, `tests/daemons_kube`/`features_daemons_kube.rs`; `deliver_container` callers in `tests/daemon_config_delivery_host.rs` (add the `None` arg; add a test that a `Some` copy lands in `secrets/gitconfig` with `config_file` and `None` removes it); `daemon_config_delivery.rs` unused `CONTAINER_DATA_DIR` check; docs `docs/user-guide/sandboxed-daemon.md` / `settings.md` if they name the Git configuration share. Then record red/green in `tdd/cycle-log.md`: the new tests are `a_stored_git_config_share_is_dropped_without_a_notice` (core sandbox), `allowed_naming_the_removed_items...` (kube settings, now with `git_config`), `the_git_config_is_a_key_of_the_startup_secret_and_no_secret_of_its_own` (start_tests), `the_shared_copy_is_none_without_a_file_and_carries_no_helper`, `the_git_config_is_given_as_a_sanitized_copy_without_any_share`; say honestly red was not run (build queue). Run Verify (`cargo test -p micold-core --lib kube`, `-p micold-client --test daemon_config_delivery_host`, `-p micold-daemon supervisor`), tick T139, reviews A/B, full gate, PR.

Stale leftover: pid 1123420 (`bash -c scripts/build-lock.sh cargo test -p micold-core --lib -- daemon_config::matching ...; ... ALLDONE`) and its queued child 1663635 from the earlier verify worker are still waiting on the build lock in this worktree; I was not allowed to kill them. Kill them first so they do not compile against the half-edited tree.

## Open escalation

None.

## Follow-ups not done

- M1: share `UreqTransport` via core; page cap and timeout for reach/fallback; forward `store`/`erase` to fallback helpers; capture fallback helpers per session cwd and URL-matching keys; `network.ca_file` for the daemon's HTTPS.

None yet.

- M2 follow-up: replacing `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount (T094 text) was not done: the sanitized copy stays the delivery mechanism and container and pod daemons name it as `[git] config_file`. Also `Msg::CredentialStatuses` in the client feature is now without a producer (review A F1, MINOR).
- Review A (high): CLEAN, 3 MINOR (F2 fixed: stale module doc and unused `RENEW_MARGIN_SECS` removed). Review B (sonnet): F1 MAJOR (no cycle-log entry) fixed; F2/F3 MINOR noted.

- Rebase onto origin/main (M2 part 6): shell `scoped_credential` module removed (main moved UreqTransport/check_and_store to `pat_source`); `scoped_token_fake_api` steps 8 and 10 dropped with the retired grant schedule; supervisor git edits keep main's `combine_git_edits`. Windows red test was `spawn::tests::terminate_falls_back...` (78ae518f1 addresses it).
