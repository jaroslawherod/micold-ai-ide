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
