---
feature: 1723-daemon-qualified-refs-routed-envelope
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset found; extension copy used
verified_at: 4b2a22c13
behaviors: 10
proven: 0
likely: 0
test_after: 10
no_test: 0
high_smells: 1
criteria_total: 9 # acceptance scenarios (US1 x3, US2 x3, US3 x1, US4 x2); FR-001..FR-010 and SC-001..SC-005 also mapped
criteria_covered: 9
mutation_score: unmeasured # no mutation tool; 6 deliberate mutants, 5 confirmed caught, 1 inconclusive (see below)
mutants_survived: 0
suite: scoped run 246 passed, 0 failed (full workspace suite not run); wall 3836s, almost all of it waiting on the shared build lock
---

# TDD Verification: Daemon-qualified refs and the routed envelope

**Verdict: FAIL.** There is no cycle log and no recorded red for any behavior, and every feature commit
lands test and source together, so nothing can be classed `PROVEN` or `LIKELY` (rubric: no red recorded
is `TEST_AFTER`). Separately, one HIGH vacuous assertion (the FR-007 refusal test). The tests that exist
are otherwise sound: the sampled mutants were caught, no existing test was weakened, and every
acceptance scenario has a test.

Audit independence: the auditor did not write these tests. Not run through a subagent; every cited line
was read in this session.

## Test-first evidence

No `tdd/test-list.md` and no `tdd/cycle-log.md` exist (autopilot ledger: "No separate tdd test-list";
M2 B review: "cycle-log not added"). The test list is the Test strategy table in `plan.md`; behavior ids
below are assigned here. History: `42ef44c1e`, `d297a0c65`, `5f69d7b48`, `21e79a578` each add tests and
source in one commit; the `fix(1723)` commits add source and tests together or tests only. Git cannot
show ordering, and the ledger records no red for any cycle (the only reds it records are CI flakes and
an E0004 in `sandbox_real_mise_setup.rs`, which are not TDD reds).

| Id | Behavior (plan Test strategy row) | Class | Evidence |
|---|---|---|---|
| B1 | FR-001 `WireDaemonId` valid/invalid, JSON+postcard | TEST_AFTER | `42ef44c1e` test+source together; no red recorded |
| B2 | FR-002 / SC-001 refs round trip, unequal across daemons | TEST_AFTER | `42ef44c1e`; no red |
| B3 | FR-003 rows carry daemon (session, worktree: `42ef44c1e`; project + identity: `5f69d7b48`) | TEST_AFTER | no red |
| B4 | FR-004 / FR-006 envelope through `WireCodec`, plain/routed mix-ups | TEST_AFTER | `d297a0c65`; no red |
| B5 | FR-005 / SC-002 bad target cases, payload not read | TEST_AFTER | `d297a0c65`, `202fe5b0f`; no red |
| B6 | FR-007 / SC-003 previous version refused, hash covers `routing.rs` | TEST_AFTER | `42ef44c1e`; no red |
| B7 | FR-008 project identity derivation, override, no remote, `Git::config_local_get` | TEST_AFTER | `5f69d7b48`; no red |
| B8 | FR-009 / SC-004 contract example decodes | TEST_AFTER | `21e79a578`; no red |
| B9 | FR-010 / SC-005 unwrapped client still served | TEST_AFTER | `42ef44c1e`; no red |
| B10 | `daemon.id` config (T004/T005) | TEST_AFTER | `42ef44c1e`; no red |

Existing tests: no assertion removed, loosened, skipped or excluded by the 1723 commits. What was
removed or edited: `assert_eq!(PROTOCOL_VERSION, 77)` became a pin on the new number (`protocol_roundtrip.rs`,
later `the_protocol_version_is_80`, legitimate: the bump is the behavior); `schema_hash.rs` call sites gained
the fourth `routing` argument mechanically; ~25 daemon test `match` arms gained `Frame::Routed(_)` (`202fe5b0f`,
mechanical, no check dropped). The `GitCredentialGrant` test deletions in `protocol_roundtrip.rs` between
`3530aeba0` and HEAD belong to #1530, not this feature. `tasks.md`: all T001-T032 are `[x]`, none has a
behavior id on a list to contradict; no inverse case.

## Findings

| # | Sev | Tag | Finding | Evidence |
|---|---|---|---|---|
| 1 | HIGH | tests/docs-only | No red evidence for any behavior; all ten are `TEST_AFTER` by the rubric. Cannot be repaired after the fact; needs a recorded waiver or reconstructed reds. | no `tdd/` directory before this report; ledger `autopilot.md` Review rounds |
| 2 | HIGH | tests/docs-only | Vacuous assertion: the previous-version refusal test checks `format!("{err:?}")` contains `"79"` and `"80"`. `RefusalReason::VersionMismatch` also Debug-prints `client_hash` and `daemon_hash` (32 `u8` each), so either number can be satisfied by a hash byte, and the user-facing message is not what is checked. Should match `VersionMismatch { client, daemon, .. }` with `client == PROTOCOL_VERSION - 1 && daemon == PROTOCOL_VERSION`, and assert on the rendered refusal text. | `crates/micold-core/tests/handshake.rs:211-224`, `crates/micold-core/src/protocol/handshake.rs:118-125` |
| 3 | MED | behaviour-code | Untested behavior in `Catalog::project_identity`: the 30 s identity cache (`IDENTITY_TTL`), "a failed git read is not cached" (the `240827a3c` fix) and `set_daemon_id` clearing the cache. Nothing references them; `Catalog::snapshot_with(&dyn Git)` was added as an injection seam and is not called by any test. Needs a daemon test with a `FakeGit` whose read fails once, then succeeds, asserting the second snapshot shows the remote identity, and one asserting the cache serves within the TTL. | `crates/micold-daemon/src/catalog.rs:218-265`, `:414-417`; `crates/micold-daemon/tests/project_identity_row.rs` |
| 4 | MED | tests/docs-only | Eager test: `the_handshake_is_plain_then_the_codec_switches_to_routed` stands a `Ping` control frame in for the handshake and covers four behaviors (plain accepts control, switch, control rejected, routed accepted). The plan asked for a plain handshake then switch; no real `Hello`/`Welcome` is used. | `crates/micold-core/tests/routed_codec.rs:80-103` |
| 5 | MED | tests/docs-only | Re-implemented/inventory expectation: `every_envelope_field_and_error_variant_is_listed` parses `routing.rs` source text to find variants and asserts `len() == 5`; a sixth variant breaks it on a count, not on the missing documentation. Acceptable intent (keeps contract and code in sync) but coupled to formatting. | `crates/micold-core/tests/contract_example.rs:60-85` |
| 6 | LOW | tests/docs-only | Weak second assertion: `error.to_string().contains("id")` is satisfied by almost any config error; the first (`contains("daemon id")`) already pins it. | `crates/micold-core/src/daemon_config/tests.rs:1255-1257` |
| 7 | LOW | tests/docs-only | Bypassed test utility: `project_identity_row.rs` hand-rolls `git(dir, args)` while `crates/micold-daemon/tests/support/mcp.rs:27` and `support/runs.rs:46` already provide one. | `crates/micold-daemon/tests/project_identity_row.rs:13-25` |
| 8 | LOW | tests/docs-only | `a_malformed_target_is_invalid` covers a bad string and a number only; boolean, array and object targets (the three other `InvalidTarget` branches) are unpinned. | `crates/micold-core/src/protocol/routing.rs` tests, `from_json_slice` match arms |
| 9 | LOW | tests/docs-only | SC-004 says the contract states the remote normalisation steps and the US4 examples; `contract_example` checks only fields, errors and the example frame. Normalisation prose is unverified. | `crates/micold-core/tests/contract_example.rs` |
| 10 | INFO | tests/docs-only | Working tree carried uncommitted edits to `specs/.../autopilot.md` and `quickstart.md` that this audit did not make. | `git status` at audit end |

Properties: tests are deterministic (tempdirs, no clock; the only wait is a 10 s timeout on a closed
connection that resolves at once), isolated, and fast (core lib 0.03 s, core main 0.06 s, daemon routed
test 2 s for 22 tests). Failures are specific (one behavior per name) except finding 4. Insensitive to
refactoring apart from finding 5. No secrets found.

## Mutation results

No mutation tool in the profile (`mutation: null`, no `cargo-mutants`), so deliberate mutants, one at a
time, scoped to the two new core files, run against `cargo test -p micold-core --lib -- routing::
project_identity::`. Each was restored with `git checkout -- <file>` and `git diff` confirmed clean;
each later run showed the other 19 tests green, which confirms the restore. Sample: 6 mutants across 3 of
10 behaviors' implementation files (B1, B5, B7); the daemon catalog and codec were not mutated.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `routing.rs` `owns`: `==` to `!=` | B5 | No | `owns_refuses_a_row_of_another_daemon` failed |
| `routing.rs` id length `<=` to `<` | B1 | No | `valid_ids_are_accepted` failed, 63-char boundary pinned |
| `project_identity.rs` drive-letter `count() == 1` to `== 2` | B7 | No | `a_local_path_remote_counts_as_no_remote` failed |
| `project_identity.rs` `origin` to `upstream` | B7 | No | `origin_wins_else_the_first_remote_in_config_order` failed |
| `routing.rs` nested `msg` check to `if false` | B5 | No | `a_double_wrapped_envelope_is_nested` failed |
| `routing.rs` empty-target guard `is_empty()` to `len() > 100` | B5 | inconclusive | The run reported a failure but my script mislabeled it as a compile error and the log was not kept; not re-run (time cap). Treat as unverified. |

Score: not computed (no tool). 5 of 5 conclusive mutants caught; survivors inside a `DONE` behavior: none
found, but finding 3 names code no test can kill.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 AC1 (rows round trip with daemon id) | `protocol_roundtrip::rows_and_qualified_refs_round_trip...`, `a_project_row_carries...`, `unwrapped_client::rows_carry_*`, `project_identity_row` | Yes (daemon catalog snapshot) |
| US1 AC2 (ref from row equals decoded ref) | same `protocol_roundtrip` tests | Wire level only; no consumer exists yet |
| US1 AC3 (same local ref, two daemons, unequal) | `the_same_local_ref_under_two_daemons_is_unequal_and_hashes_apart`, `project_identity_row::a_repository_without_a_remote...` | Yes |
| US2 AC1 (envelope round trip, every `ClientMsg`) | `routed_codec::an_envelope_around_each_client_message_round_trips`, `routing::conn_and_seq_round_trip...` | Yes (`WireCodec`) |
| US2 AC2 (missing/invalid target rejected, nothing delivered) | `routing::a_missing_or_empty_target...`, `a_malformed_target...`, `routed_codec::a_routed_frame_without_a_target...`, `handshake_flow::a_routed_frame_on_a_plain_connection...` | Yes (daemon connection closes, `Ping` not answered) |
| US2 AC3 (plain/routed mix-up rejected) | `routed_codec::*wrong_routing`, `handshake_flow` test | Yes for routed-on-plain at the daemon; reverse at codec only |
| US3 AC1 (previous version refused naming both) | `handshake::the_previous_protocol_version_is_refused...` | `evaluate` only; see finding 2 |
| US4 AC1 (rule stated with reason and failure case) | `contract_example` (fields, errors, example); prose unverified | Partial (finding 9) |
| US4 AC2 (URL equivalences, no-remote differ) | `project_identity` unit tests, `project_identity_row` | Yes (real `Catalog::snapshot`, real git) |
| FR-007 hash change | `schema_hash::editing_routing_rs_changes_the_hash`, `baked_hash_matches...` | Yes |
| FR-010 / SC-005 unwrapped client | `unwrapped_client` | Yes; "existing suites pass unchanged" not measured here |

Untested criteria: none. Tests tracing to nothing: none. A `traces` column does not exist (no list), so
the mapping above is by name and plan row.

## What was not audited

- The full workspace suite (5000+ tests, 429 binaries) was not run; only the feature's tests: core `--lib`
  (`routing::`, `project_identity::`, `config_local_get`, `daemon_config`), core `--test main`
  (`routed_codec::`, `protocol_roundtrip::`, `handshake::`, `schema_hash::`), `--test contract_example`,
  daemon `unwrapped_client`, `project_identity_row`, and `--test main` (`handshake_flow::`, `config_reload::`).
  SC-005 ("existing suites pass unchanged") therefore rests on CI of the merged PRs, not on this run.
- The 126 other test files edited for the new `daemon`/`identity` fields (client and daemon fixtures) were
  diff-checked for removed assertions, not read in full.
- Mutation was limited to 6 mutants in `routing.rs` and `project_identity.rs`; the codec, `catalog.rs`,
  `git.rs` `config_local_get` and `GitCli` were not mutated. The time cap was exceeded in waiting for the
  shared build lock, not in mutation work.
- No coverage tool exists (`coverage: null`); `sandbox_real_*` acceptance tests were not run.
- Docs were checked only through `contract_example` and a grep for protocol numbers in `docs/daemon.md`
  and the contract (both state 80 correctly); prose quality was not assessed.
- No remediation tasks were appended to `tasks.md`: the caller restricted edits to this file.

## Suggested remediation (not applied)

1. (finding 2, HIGH) Replace the Debug-substring checks with a `matches!` on `VersionMismatch { client, daemon, .. }` and an assertion on the rendered refusal text. Proof: `scripts/build-lock.sh cargo test -p micold-core --test main handshake::`.
2. (finding 1, HIGH) Record a waiver in the ledger, or reconstruct reds by reverting each behavior's source in a scratch branch and recording the failing command in a `tdd/cycle-log.md`.
3. (finding 3, MED) Add the `FakeGit`-driven cache tests in `crates/micold-daemon/tests/project_identity_row.rs`. Proof: `scripts/build-lock.sh cargo test -p micold-daemon --test project_identity_row`.
4. Findings 4-9: tighten as described; none blocks.

## Close-unit disposition (2026-11-11)

- #2, #8: fixed in the close PR (core tests).
- #1 (no red evidence; test and source landed in one commit per milestone): waived. The merged milestones are history and cannot be reshuffled.
- #3 (identity cache has no test; needs a seam in `catalog.rs`), #4, #5, #6, #7, #9: not done in this PR; listed in the ledger's *Follow-ups not done*. #3 would add behaviour code (a seam), so it is outside a close unit.
- Mutation run covered 6 mutants in `routing.rs` and `project_identity.rs`: 5 killed, 1 (empty-target guard) inconclusive, none survived.
