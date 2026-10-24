# Cycle Log: 1289 — sandbox project setup

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation. Commands ran with `CARGO_INCREMENTAL=0` and debuginfo 0 (disk was short);
output is copied from the session's logs.

## M1 cycle 1: A1 the toolchain `PATH` order (T001 → T002)

- test: `crates/micold-core/tests/toolchain_path.rs`, all 14 rows of contract toolchain-path P3 and
  the FR-003/FR-004 cases (`p3_offered_bins_then_local_bin_then_default_shims_then_rest`,
  `p3_mise_data_dir_names_the_shims_folder`, `p3_xdg_data_home_names_the_shims_folder_without_mise_data_dir`,
  `p3_a_home_folder_already_in_rest_moves_to_its_place`, `p3_no_offered_bins_still_adds_the_home_folders`,
  `mise_data_dir_wins_over_xdg_data_home`, `empty_mise_data_dir_and_xdg_data_home_count_as_unset`,
  `an_empty_home_counts_as_unset`, `without_home_both_home_folders_are_left_out`,
  `without_home_mise_data_dir_still_names_the_shims_folder`, `a_folder_repeated_in_rest_keeps_only_its_first_position`,
  `an_offered_bin_repeated_in_rest_is_dropped_from_rest`, `an_empty_rest_adds_no_empty_entry`,
  `folders_are_named_whether_or_not_they_exist`)
- red: `scripts/build-lock.sh cargo test -p micold-core --test toolchain_path` ->
  `error[E0432]: unresolved import micold_core::sandbox::toolchain_path` at
  `crates/micold-core/tests/toolchain_path.rs:8:27` (`could not find toolchain_path in sandbox`), exit 101
- green: T002 adds `core/src/sandbox/toolchain_path.rs`; same command ->
  `test result: ok. 14 passed; 0 failed`
- commit: `65b4a48c`

## M1 cycle 2: A2 (a), (b), (c) and A3 the daemon gives every sandbox spawn that `PATH` (T003, T004 → T005)

- tests (`crates/micold-daemon/src/state.rs`, `offered_tests`):
  A2 (a) `a_sandbox_spawn_path_puts_the_home_folders_after_the_offered_bins`,
  A2 (b) `an_empty_offer_still_adds_the_home_folders`,
  A2 (c) `without_an_offer_the_resolved_path_is_unchanged`,
  A3 `a_home_copy_of_a_cli_is_not_offered_ahead_and_a_shim_resolves`
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib offered` -> `4 passed; 4 failed`:
  - A2 (a) `state.rs:8487:9` `left: "/tmp/.tmp2VAGiH/claude-bin:/usr/local/bin:/usr/bin:/bin"`,
    `right: "/tmp/.tmp2VAGiH/claude-bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin"`
  - A2 (b) `state.rs:8512:9` `left: "/usr/bin:/bin"`, `right: "/home/u/.local/bin:/opt/mise/shims:/usr/bin:/bin"`
    (the early return on an empty offer)
  - A2 (c) `state.rs:8456:14` `a PATH in the spawn environment`. A2 (c) checks FR-008, which the
    code before T005 already met, so this failure is not evidence for T005: its red is the mutant
    in M1 cycle 6
  - A3 `state.rs:8563:9` `left: None`, `right: Some("/tmp/.tmpIF6Pap/home/.local/share/mise/shims")`
- green: T005 `with_sandbox_path`; same command -> `test result: ok. 8 passed; 0 failed`
- commit: `65b4a48c`

## M1 cycle 3: review A-F1 a folder holding the separator drops only itself

- tests (`crates/micold-core/tests/toolchain_path.rs`):
  `a_home_holding_the_separator_leaves_out_only_the_home_folders` (`HOME=/a:b`),
  `an_xdg_data_home_holding_the_separator_leaves_out_only_the_shims_folder` (`XDG_DATA_HOME=/a:/b`)
- red (tests added, `toolchain_path.rs` as at `65b4a48c`):
  `scripts/build-lock.sh cargo test -p micold-core --test toolchain_path` -> `14 passed; 2 failed`:
  - `toolchain_path.rs:171:5` `left: "/usr/local/bin:/usr/bin:/bin"`, `right: "/s/ai-clis/claude/bin:/usr/local/bin:/usr/bin:/bin"`
  - `toolchain_path.rs:178:5` `left: "/usr/local/bin:/usr/bin:/bin"`, `right: "/s/ai-clis/claude/bin:/home/u/.local/bin:/usr/local/bin:/usr/bin:/bin"`
- green: each candidate is kept only if `join_paths([folder])` succeeds; same command ->
  `test result: ok. 16 passed; 0 failed`

## M1 cycle 4: review A-F3 an offered CLI must be in its own `bin` folder

- test: `state.rs` `an_offered_cli_missing_from_its_bin_is_not_offered_through_a_home_copy`
  (offered bin empty, then missing; `claude` only in `$HOME/.local/bin`)
- red (`only_offered` as at `65b4a48c`): `scripts/build-lock.sh cargo test -p micold-daemon --lib offered`
  -> `8 passed; 1 failed`, `state.rs:8621:9` panicked: `bin lacks it`
- green: `only_offered` keeps a CLI only when `provider().is_available(bin)`; the cycle 5 run below
  -> that test `ok`

## M1 cycle 5: review A-F2 the A2 (a) and A3 tests do not depend on the test process's `MISE_DATA_DIR`

- tests: A2 (a) and A3 above; their scripts now export `MISE_DATA_DIR=` (empty counts as unset)
- red (without that export, `MISE_DATA_DIR=/inherited-mise` in the test process):
  `MISE_DATA_DIR=/inherited-mise scripts/build-lock.sh cargo test -p micold-daemon --lib offered`
  -> `7 passed; 2 failed`:
  - A2 (a) `state.rs:8509:9` `left: ".../claude-bin:/home/u/.local/bin:/inherited-mise/shims:/usr/local/bin:/usr/bin:/bin"`,
    `right: ".../claude-bin:/home/u/.local/bin:/home/u/.local/share/mise/shims:/usr/local/bin:/usr/bin:/bin"`
  - A3 `state.rs:8589:9` `left: None`, `right: Some("/tmp/.tmpvxHSLW/home/.local/share/mise/shims")`
- green: with the export, same command -> `test result: ok. 9 passed; 0 failed`

## M1 cycle 6: A2 (c) a daemon without `offered.json` keeps the resolved `PATH` (FR-008), mutant red

- test: `state.rs` `without_an_offer_the_resolved_path_is_unchanged`
- red (mutant: `with_sandbox_path`'s `let Some(offered) = self.offered_clis() else { return resolved; }`
  -> `let offered = self.offered_clis().unwrap_or(Offered { version: 1, clis: Vec::new() })`):
  `scripts/build-lock.sh cargo test -p micold-daemon --lib without_an_offer_the_resolved_path_is_unchanged`
  -> `state.rs:8555:9` `left: "/home/u/.local/bin:/home/u/.local/share/mise/shims:/a:/usr/bin:/bin"`,
  `right: "/a:/usr/bin:/a:/bin"` (1 failed). Restored from a copy; `git diff` unchanged
- green: no production change
