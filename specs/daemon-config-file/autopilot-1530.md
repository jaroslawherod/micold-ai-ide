# Autopilot ledger — #1530 daemon-config-file (Issue 9)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1530` (The daemon's credential helper chooses the token from the configuration); labels: `effort:high`, `flow:feature`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1529 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-008, FR-010, FR-016, FR-030, FR-031, FR-035, `contracts/daemon-config.md` sections `[git]` and `[[github.tokens]]`); the tasks unit only cuts milestones for the "Issue 9" group (T046–T054, T092–T094).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1530
- **Worktree branch**: feat/1530_the-daemon-s-credential-helper-chooses-the-token
- **Started**: 2026-11-04
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1686 | Design (milestones for Issue 9) | MERGED | 826681cfb63664b82f65026abfd61abb190956c5 |
| #1700 | M1 | MERGED | 3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f |
| #1713 | M2 | MERGED | 86468d87ced75974d0aeca37c7d7675976d1a934 |
| #1775 | M3 | MERGED | 658807d3515cd47a7979c9c5de36d32bf5304528 |
| (close) | Close Issue 9 | OPEN | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T046–T052, T093, T054 | full | helper chooses token from config (most specific, ties first), reach cached by fingerprint, per-kind session git env with fallback/clear, per-host rewrite | #1700 | merged |
| M2 | T092, T094, T053 | full | git setup from daemon.kind on every start path, GIT_CONFIG_GLOBAL, gpgsign stripped, grant messages retired + protocol bump | #1713 | merged |
| M3 | T139 | full | container and pod git config only from `[git] config_file`: remove `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount (T094 remainder, I9-2) | #1775 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1530? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-008/010/016/030/031/035. Skipped as for #1524/#1525/#1526/#1529/#1534. Blockers #1524 and #1529 are merged (#1529 via PR #1684). | agent-resolved | specs/daemon-config-file/autopilot-1529.md |
| D2 | tasks | Split Issue 9? | Yes: 11 tasks, two stories (rule 3). M1 = helper + env (T046–T052, T093) with T054 guide (rule 4); M2 = kind gating, git config file, retirement T053 (wire change reviewed alone). Both full: security check, sandbox boundary, wire protocol. | agent-resolved | milestones.md |
| D3 | close | Unbuilt T094 half becomes M3? | Yes (close rule 2): T094 ticked, its GitConfig/kube Secret replacement deferred. Test-strength findings I9-1, I9-3..I9-10 (T140-T147) stay for the next Close; mutation testing was not run (build-lock queue), T147 does it. | agent-resolved | tdd/verification.md Issue 9 |

## Review rounds

- M1: review A (code-review high) 10 findings, 4 fixed (host reset now per forge, fallback on unreadable config, absolute config path, token file leading blank line); review B (sonnet) CLEAN, 3 MINOR (F1 fixed). Snapshot 503b13b8:826681cfb.

- M3: review A (code-review high, snapshot 5e2ddf9c:e2e690bc) CHANGES: F1 MAJOR (daemon still falls back to the state-dir gitconfig copy) fixed by removing `legacy_identity_exists` and the fallback arm, round 2 not needed beyond the gate (fix is local, covered by supervisor tests). Review B (sonnet, snapshot 1cff3b64:6ab641d5) CHANGES: F1 MAJOR (no red evidence) fixed by recording in the cycle-log that the new tests do not compile on origin/main (no behavioural red run possible; code and tests landed together), F2 MINOR fixed; prose only, no round 2.
- M2: review A (code-review high) CLEAN, 3 MINOR (F2 fixed, F1 noted in Follow-ups); review B (sonnet) F1 MAJOR (no cycle-log entry) fixed, F2/F3 MINOR noted. Rebased onto origin/main afterwards; full gate re-run on the rebased tree. A Close unit follows this last milestone.

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Close | 1 | bc845dea87574dd174dc18af75b0f2b640099818:b9cb9954ae561d72d738fc411cf4514a65c734dc | CLEAN, 3 MINOR (F1 test renamed, F2 noted in cycle-log, F3 task text reworded; prose only) |
| Tasks | 1 | fd03d24b9bf4c7aac5fd8b2e77207878cc688dbc:259bcb30e95d60c215c8bba19b45fac7123e8c07 | CHANGES: F1 MAJOR (no guide task in M2) fixed by writing docs into T094; F2 MINOR git_credential_grant.rs added to T053; F3 MINOR noted: matching.rs and git_credential_config.rs are created by T046/T047, no `TokenChoice` exists yet. Fixes are task-text only, no new round |

- Close (resumed unit): rebased onto origin/main; found a leftover mutant (`Some(()).map` in `daemon_config_delivery.rs`) in the working tree and restored it; T147-T151 done: 7/7 mutants caught (cycle-log, verification.md), I9-11..I9-15 closed; spec Status extended for Issue 9.

## Declined review findings

- M1 review A: reach listing capped at 30 pages, URL-form GHE host gets no `/api/v3`, fallback capture uses the daemon's cwd and exact keys, `run_fallback` has no timeout and forwards only `get`, `UreqTransport` duplicates the client's, legacy `GIT_CONFIG_KEY_n` leftovers: accepted for M1 (MINOR in effect; listed in Follow-ups). Review B MINORs F2 (cache lock), F3 (capture cwd): same.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Close work done and committed locally (HEAD 29c835c93 plus this note), rebased on origin/main, full `mise run gate` green, review CLEAN. NOT pushed, no PR: `git push` fails because the repo's `core.askpass` (`/home/jaro/.local/bin/gh-askpass.sh`, set in the main checkout's .git/config) does not exist; `gh` itself is logged in. Need: the script restored (or the push done by someone with credentials), then `git push --force-with-lease -u origin HEAD` and `gh pr create --base main --title "docs(1530): close the spec (#1530)"` with body ending `Refs #1530`; record the PR in the table row `(close)`.

## Open escalation

None.

## Follow-ups not done

- M1: share `UreqTransport` via core; page cap and timeout for reach/fallback; forward `store`/`erase` to fallback helpers; capture fallback helpers per session cwd and URL-matching keys; `network.ca_file` for the daemon's HTTPS.

None yet.

- M2 follow-up: replacing `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount (T094 text) was not done: the sanitized copy stays the delivery mechanism and container and pod daemons name it as `[git] config_file`. Also `Msg::CredentialStatuses` in the client feature is now without a producer (review A F1, MINOR).
- Review A (high): CLEAN, 3 MINOR (F2 fixed: stale module doc and unused `RENEW_MARGIN_SECS` removed). Review B (sonnet): F1 MAJOR (no cycle-log entry) fixed; F2/F3 MINOR noted.

- Rebase onto origin/main (M2 part 6): shell `scoped_credential` module removed (main moved UreqTransport/check_and_store to `pat_source`); `scoped_token_fake_api` steps 8 and 10 dropped with the retired grant schedule; supervisor git edits keep main's `combine_git_edits`. Windows red test was `spawn::tests::terminate_falls_back...` (78ae518f1 addresses it).

- M3: full gate on the rebased tree: 12648/12649; `micold-daemon::ai_cli_config a_configured_model_is_claude_codes_default_launch_model` fails here in code this diff does not touch (recorded in the #1537, #1703, #1723, #1728 ledgers: depends on real HOME `~/.claude.json`). Fixed in the gate round: `RawShare::Unknown` (an unknown credential entry no longer loses the list), `container_projects` drops the git config mount. Shell suites pass.
