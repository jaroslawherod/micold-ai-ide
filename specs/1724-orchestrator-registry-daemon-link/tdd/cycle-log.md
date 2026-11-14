# TDD cycle log, #1724

## M1 (T001-T016)

Gap, recorded honestly: no red phase was captured. Tests and implementation were written together
and uncompiled (other worktrees held the shared build lock for over 40 minutes, so no `cargo` ran in
the writing session). They were compiled and run afterwards.

Green evidence: `cargo test -p micold-core --lib -- orchestrator daemon_config` 146 passed;
`cargo test -p micold-daemon --test orchestrator` 11 passed; clippy `-D warnings` clean.

Compile errors found when first built (the closest thing to a red): a `todo!` left in
`preamble_expired`, and untyped `.into()` on codec sends in `link.rs` and the test support.

Review A found `no_child_process_is_spawned` vacuous (it read only the main thread's children); it
now sums the children of every task. M2 onward: write each behaviour's test, see it fail, then pass.

## M2 (T017-T025)

Gap, recorded honestly: no red phase was captured for T017-T022. The tests and code were written
together by an earlier unit and compiled afterwards (the shared build lock was held by other
worktrees for long stretches). T022's client test `a_link_ping_is_ignored` passes without the new
arm too, because the match keeps its `_ => {}` wildcard; the explicit arm is documentation.

Review A found a real bug the first tests missed: after `Expired` a closed transport never ended
the link loop, and a read error or `Goodbye` ended it without waiting for the deadline. Fixed in
`link.rs`; `a_link_that_drops_after_going_offline_can_be_relinked` covers the first case.

## M3 (T026-T030)

Red runs (tests written first, stubs only, then green):

- `registry::tests` (core, 26 passed / 8 failed against a stub `claim_failed` and the old `claim`):
  `a_needs_update_daemon_that_relinks_on_the_current_version_is_online` ("not reserved:
  Refused(AlreadyLinked)"); `needs_update_keeps_the_last_seen_of_the_record_it_replaces`
  (left `(Online, 100)`, right `(NeedsUpdate, 100)`); also red: `an_older_or_newer_claim_with_no_link_...`,
  `a_mismatched_claim_on_a_live_id_...`, `a_mismatched_claim_on_a_silent_link_...`,
  `a_failed_reservation_for_the_version_...`, `build_mismatch_auth_rejected_and_dropped_...`.
- `announce::tests::the_refusal_text_names_both_versions`: red against an empty message.
- `tests/orchestrator/version_mismatch.rs` (7 tests; 5 passed once `claim` recorded `needs-update`):
  `..._an_older_daemon_is_refused_naming_both_versions_and_listed_needs_update` (`left: Null, right: 81`
  on the record's `orchestrator_version`); `..._a_different_schema_hash_is_refused_at_hello_and_listed`
  (record `None`: a `Hello` refusal recorded nothing).

Green: `cargo test -p micold-core --lib -- orchestrator` 57 passed; `--test orchestrator` 22 passed.
Not on the wire: `AuthRejected` (the orchestrator holds no token, so `Hello` cannot be refused for
it); the unit test `build_mismatch_auth_rejected_and_dropped_...` covers it. The golden old-format
preamble is `announce::tests::an_old_announcement_still_decodes` plus the wire test
`..._an_old_format_preamble_is_still_refused_and_listed`. `claim_failed` cannot meet a live link in
M3's one-slot model (a reservation excludes one); its stale-token test covers the never-over-live rule,
and M4 revisits it when a reservation can sit beside a silent link.
