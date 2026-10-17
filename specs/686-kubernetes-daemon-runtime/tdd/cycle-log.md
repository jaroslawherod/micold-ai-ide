# Cycle Log: Kubernetes session daemon runtime

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 6966 passed, 6 failed,
  12 ignored, 531 test binaries
- commit: `270a530f`
- recorded: cycle 0, before any change

## Notes and deviations

- The baseline ran in a cloud container as root (uid 0). Root ignores file permissions, so six
  tests that make a path unreadable or unremovable and expect a refusal fail here, and only here:
  `settings_refuses_save_over_failed_read::a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`,
  `settings_write_is_logged::a_refused_write_is_logged_as_a_failure_with_the_reason`,
  `worktree_leftovers::{a_directory_that_cannot_be_emptied_names_what_survived, the_leftover_report_is_capped}`,
  `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path_still_archives_and_reports`,
  `settings_service_write_refuses_failed_read::a_service_write_over_an_unreadable_settings_file_is_refused`.
  None touches this feature's components. The loop treats them as pre-existing reds: run the
  loop as a non-root user, or exclude these six when judging "suite green".

## Cycle 1 (batch, M1 core types): U1–U12, U18–U21, U23–U25, U26 — RED, handed over

- deviation: one batch of behaviours per source file rather than one per cycle, to fit the milestone
  (38 tasks) into unit-sized contexts; each test still written and observed failing before its
  implementation. Stubs only (bodies return the wrong value), no implementation yet.
- tests: `crates/micold-core/src/kube/{settings,names,classify}.rs`, `daemon_image/mod.rs`,
  `protocol/auth.rs` (`listener_auth_tests`), all `#[cfg(test)]`.
- red: `scripts/build-lock.sh cargo test -p micold-core --lib -- kube:: daemon_image:: listener_auth_tests`
  -> `test result: FAILED. 4 passed; 18 failed` (assertion failures, e.g.
  `kube::names::tests::the_key_is_eight_hex_of_the_nul_joined_triple panicked at crates/micold-core/src/kube/names.rs:125:9`).
- passed on first run against the stubs (need the deliberate-mutant check after green):
  `a_toleration_with_another_effect_is_refused_naming_tolerations`, `each_of_the_four_can_be_set_alone`,
  `defaults_are_amd64_no_storage_class_and_nothing_allowed`, `an_empty_path_on_loopback_needs_no_token`.
- additions to the data model (recorded for review): `KubeFailure::Unreachable`/`Forbidden`/`Conflict`
  carry `context`/`namespace` for their contract text; new `VolumeTooSmall { claim }` (T021's
  smaller-volume refusal) and `Api { code, reason, message }` (unclassified answers until M6).
- commit: the handover commit (red; next unit makes it green).
