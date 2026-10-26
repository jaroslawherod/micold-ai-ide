# Cycle Log: Review UI docs sync — #1276

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: M1's full gate (`scripts/gate.sh`) green at `ea01a8b9`; CI's merge gate green on
  `origin/main`.

## Cycle 1: T003–T005 `review_keys_doc` holds the guide's shortcut table to `REVIEW_KEYS`

- test: `crates/micold-core/tests/review_keys_doc.rs` (3 new: `the_markers_are_there`,
  `the_guides_table_is_the_review_key_table`, `each_primary_chord_names_its_macos_form`), on the
  shared `tests/support/marked_table.rs`
- red: recorded after the fact (review B F1), without editing tracked files: a detached scratch
  worktree at `origin/main` (`02421815`, guide before M1) with only the four test files copied in
  (`review_keys_doc.rs`, `support/marked_table.rs`, `support/mod.rs`, `terminal_only_doc.rs`), then
  `cargo test -p micold-core --test review_keys_doc` -> `0 passed; 3 failed`:
  `the_markers_are_there` panicked at `review_keys_doc.rs:46`:
  ``docs/user-guide/reviewing-changes.md` has the marker `<!-- review-shortcuts:start -->``; the
  other two panicked in `marked_section` (`marked_table.rs:33`) on the same missing marker
- green: the branch's guide (T006–T008 shortcut table between the markers):
  `cargo test -p micold-core --test review_keys_doc --test terminal_only_doc --test
  documentation_is_not_read` -> 3 + 2 + 3 passed, 0 failed; `cargo clippy -p micold-core
  --all-targets -D warnings` clean
- refactor: `terminal_only_doc.rs` moved onto `marked_table.rs`; it passes both against
  `origin/main`'s guide in the scratch worktree (2 passed) and on the branch (2 passed), so the
  refactor kept its behaviour. Review A F1: `key_cell` now builds from `ReviewKey::labels()` (the
  text the `?` sheet renders) instead of re-deriving the key; F3: row count asserted before the zip
