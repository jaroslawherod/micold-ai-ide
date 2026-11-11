---
feature: 1728-orchestrator-config-section
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 653769a81
behaviors: 0 # no tdd/test-list.md; audited against spec.md FR/SC/US
proven: 0
likely: 0
test_after: 2 # M1 and M2: tests and source in one commit each, no red recorded
no_test: 0
high_smells: 0
criteria_total: 4 # SC-001..SC-004 (plus FR-001..FR-011)
criteria_covered: 3
mutation_score: unmeasured # no mutation tool; 6 deliberate mutants, 4 caught, 1 survivor, 1 equivalent
mutants_survived: 2
suite: "micold-core daemon_config: 106 passed, 0 failed (lib), 1m48s wall incl. build-lock wait"
---

# TDD Verification: [orchestrator] config section and config check

**Verdict: FAIL.** There is no test list, cycle log or recorded red for either milestone (fail-closed: test-after), and a deliberate mutant on rule O4 survived. The user-asked criteria are built and tested (see Traceability), but the spec's "url with credentials or a fragment is refused" edge case is not built and has no test.

The audit is independent: the auditor did not write these tests. Scope: `crates/micold-core/src/daemon_config/` (tests.rs, validate.rs, resolved.rs, files.rs, mod.rs), `crates/micold-daemon/tests/config_check.rs`, `crates/micold-daemon/tests/config_reload.rs`. Commits: `1ef59d87c` (M1), `1a6c50ba5` (M2). `eaaca02fd..HEAD` also contains unrelated features (#1532, #1536, #1723); those were not audited.

## Test-first evidence

| Unit | Class | Evidence |
| --- | --- | --- |
| M1 table, O1-O5, V4, type, view, docs | TEST_AFTER (unprovable) | `1ef59d87c` adds tests.rs (+190), config_check.rs (+27) and source in one commit; `tdd/` directory, test-list.md and cycle-log.md do not exist, so no red is recorded |
| M2 --files V14/V15, no-leak, reload | TEST_AFTER (unprovable) | `1a6c50ba5` adds tests (+128, +26) and files.rs (+24) in one commit; no red recorded |

Existing tests: `git show 1ef59d87c -- tests.rs` removes no lines; no assertion removed, loosened, skipped or ignored in the in-scope paths. No weakened existing test.

`tasks.md`: not cross-checked against a test list (none exists).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | No test list, cycle log or red evidence for either milestone; test-first cannot be shown | `specs/1728-orchestrator-config-section/tdd/` absent; commits `1ef59d87c`, `1a6c50ba5` |
| 2 | HIGH | Surviving mutant M1: O4 changed to `token && cert` (a key_file plus token, no cert) passes all 106 tests. Rule O4 is only tested with the full pair | `validate.rs:1041` (`if token && (cert \|\| key)`); `tests.rs:1180-1191` only uses `ORCH_PAIR` + token |
| 3 | HIGH | Spec edge case unbuilt: "`url` with credentials, a fragment ... refused as an invalid URL". `check_https_url` strips userinfo and fragment and accepts `https://user:pw@host/#f`. A URL with a password in it is also a secret held in the file, against FR-002 | `validate.rs:330-343`; spec.md Edge Cases; no test in `tests.rs:1160-1168` |
| 4 | MED | Surviving mutant M3: removing `host.starts_with(':')` (so `https://:8443` is accepted) passes. Empty host with a port is untested | `validate.rs:339`; `tests.rs:1160-1168` |
| 5 | MED | Rules O3/O4/O5 and file/mode checks are tested only in `micold-core`; the daemon entry point (`config_check.rs:254`) tests only O2. FR-009 / US3.4 (`--files`) for `[orchestrator]` has no daemon-level test; the existing `--files` tests (`config_check.rs:173,203`) do not name orchestrator keys | `crates/micold-daemon/tests/config_check.rs` |
| 6 | MED | SC-002 leak test is core-level only; `config check` stdout/stderr (the real entry point) is never asserted free of a sentinel for an `[orchestrator]` file | `tests.rs:1271-1304`; `config_check.rs:254-268` |
| 7 | MED | Assertion looseness: `.pem (missing)` matched anywhere in the view, so one of three keys satisfies it (assertion roulette) | `tests.rs:1073-1078` |
| 8 | LOW | Conditional setup inside a test (`if key == "ca_file"`); both arms assert, so not vacuous | `tests.rs:1222-1226` |
| 9 | LOW | Redundant `error.message.contains("https")` after `expect(..., "https", ...)`; repeated inline TOML literals instead of one builder (Duplicated setup) | `tests.rs:1163, 1161-1167, 1172-1176` |
| 10 | LOW | Edge cases without tests: empty-string `url`, duplicate `[orchestrator]` table, wrong type on keys other than `url` | spec.md Edge Cases |

No secret appears in any test or fixture other than the sentinels `s3cr3t-value` / `s3cr3t-sentinel` (not credentials).

Properties: tests use `tempfile`, no sleeps, no network; the 106-test module runs in 0.01s; deterministic. Style matches the module's `expect`/`errors`/`host` helpers (the exemplar kind `unit_src`). Unix-only gating (`#[cfg(unix)]`) is deliberate.

## Mutation results (deliberate mutants, no tool in profile)

Six single mutants on changed non-test files, each restored with `git checkout -- <file>` (tree verified clean after; HEAD suite was 106 green before the run). Sampled, not exhaustive.

| Mutant | Result | Judgment |
| --- | --- | --- |
| M1 `validate.rs` O4 `token && (cert \|\| key)` to `token && cert` | SURVIVED | Real gap, finding 2 |
| M2 `validate.rs` O5 `!token && !cert && !key` to `!token && !cert` | SURVIVED | Equivalent: key-only is already reported as O3 |
| M3 `validate.rs` drop `host.starts_with(':')` | SURVIVED | Real gap, finding 4 |
| M4 `files.rs` V14 secret set drops enrollment token | caught (`orchestrator_files_v14_...`) | |
| M5 `resolved.rs` view drops `enrollment_token_file` | caught (3 tests) | |
| M6 `validate.rs` O3 anchors at the missing key | caught (`o3_...`) | |

Score on this sample: 3 of 5 non-equivalent mutants caught (60%); not comparable to a tool score. Coverage not available (profile).

## Traceability

| Criterion | Tests | Real entry point | Status |
| --- | --- | --- | --- |
| (1) parses; unknown key refused (US1.1-3, FR-001, FR-010, SC-003) | `orchestrator_a_complete_table_parses...`, `orchestrator_an_unknown_key_is_refused_with_its_position` (V1 at 12:1), `orchestrator_no_table_changes_nothing` (4 daemon kinds); `config_check.rs:254` exit 0 | partly: exit 0 via `config check`; unknown key via core parser only | covered |
| (2) each rule has a test naming file:line:col (SC-001, FR-006) | O1 `tests.rs:1154`, O2 `1160`, O3 both directions `1170`, O4 `1180`, O5 `1194`, V4 `1220`, type `1242`, header `1214`; `expect` asserts rule, key, line and column; daemon-level only O2 at `daemon.toml:6:7` | core for all; daemon for one | covered; gap: finding 5 |
| (3) no secret in Debug/Display; view hides it (SC-002, FR-007, US3.1-3) | `tests.rs:1107` (mode, no content), `1140` (missing), `1271` (Debug, `{:#?}`-free Debug, Display, view, findings with sentinel) | core only | covered; gap: finding 6 |
| US3.4, FR-009 | `tests.rs:1307,1331,1348,1381` | core only | covered at unit level |
| FR-011 reload keeps last config | `config_reload.rs` `an_invalid_orchestrator_table_keeps_the_last_configuration` | real runtime | covered |
| FR-008 / SC-004 docs | `docs/user-guide/daemon-configuration.md:167-202` lists O1-O5 and examples | n/a | present, not test-enforced here |
| Edge: url credentials/fragment | none | n/a | UNBUILT (finding 3) |

Acceptance criteria (1)-(3) are built. One spec edge case (url with credentials or fragment) is unbuilt.

## What was not audited

- Full workspace suite (per instruction); only `micold-core` lib `daemon_config` (106 passed) was run. `config_check` and `config_reload` daemon tests were read but not executed.
- Mutation tool: none exists; sampled 6 mutants only. Coverage unavailable.
- Client rendering (`micold-client` `daemon_config_render.rs`, `daemon_config_validate.rs`) and non-orchestrator content in the range (#1532, #1536, #1723 commits).
- Windows behavior; docs accuracy beyond presence of rule ids.
- Flakiness under repeated runs; Debug/Display of types outside `Orchestrator`/`SecretPath`.

## Re-verification after M3 (2026-11-11, against main 9bc969097)

Findings 2, 3, 4, 5, 6 are closed by PR #1826: `check_https_url` refuses userinfo, a fragment and an empty host (`validate.rs:329-348`, test `o2_a_url_with_credentials_a_fragment_or_no_host_is_reported_at_its_value`); O4 has tests for a token with a certificate pair, a key alone and a certificate alone; `crates/micold-daemon/tests/config_check.rs` covers `--files` and the no-leak case, including a url password never echoed. Finding 1 is waived (see the ledger, T016). Findings 7-10 are LOW/MED test style and stay as follow-ups. No unbuilt behaviour remains.
