# Cycle Log: Environment include script — BUG-454

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: not run locally. A `scripts/build-lock.sh cargo test --workspace` on `8f5afc15` (cold
  target dir) was started and killed while still compiling dependencies, because the cycles' edits
  landed in the same tree it was building. Baseline taken as CI's green merge gate on
  `origin/main` at `107ab7ed` (`8f5afc15` adds only spec files).

## Cycle 1: U1 the failure list round-trips through JSON

- test: `crates/micold-core/tests/protocol_roundtrip.rs::per_directory_env_include_failures_round_trip_in_the_catalog_snapshot` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test protocol_roundtrip env_include_failures`
  -> `assertion left == right failed: json round-trip mismatch` with `env_include_failures: []` on
  the left (field declared `#[serde(skip)]` as the stub)
- green: `#[serde(default, skip_serializing_if = "Vec::is_empty")]`; file 15 passed, 0 failed
- refactor: none needed
- notes: `PROTOCOL_VERSION` 29 -> 30 with its doc line and `tests/schema_hash.rs`'s pin, as a
  structural step of T043 in the same commit

## Cycle 2: U2 a snapshot without the field reads back empty

- test: `crates/micold-core/tests/protocol_roundtrip.rs::a_catalog_snapshot_without_env_include_failures_reads_back_with_none` (new)
- red: none on arrival (passed under the cycle-1 stub). Deliberate mutant: drop `default` from the
  serde attribute -> `json decode: Error("missing field `env_include_failures`")` (1 failed);
  restored from a copy.
- green: no implementation change beyond cycle 1
- refactor: none needed

## Cycle 3: U3 `directory_failure_lines`

- test: `crates/micold-client/tests/features_settings.rs::directory_failure_lines::*` (4 new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test features_settings directory_failure_lines`
  -> 3 failed, `left: []` / `right: [Caution("Exited with an error in /work/other"), Note(..)]`
  (stub returned `Vec::new()`); `no_failures_say_nothing` passed on the stub
- green: filter out entries equal to `last`, reuse `lines_011` per entry and suffix the caution
  with ` in <dir>`; file 69 passed, 0 failed
- refactor: none needed

## Cycle 4: A1 + A2 the service reports, broadcasts and does not log the output

- test: `crates/micold-daemon/tests/env_include_failure_report.rs::{a_directory_whose_resolution_failed_is_reported_while_its_failure_is_cached, the_log_names_the_failed_directory_without_the_scripts_output}` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test env_include_failure_report`
  -> A1 `assertion left == right failed`, `left: []` at line 136; A2 `the script's output reached
  the log: ... outcome=NonZeroExit { code: 3, diagnostic: "...BUG454-OUTPUT" }`
- green: `ResolvedEnv::outcome`, `snapshot_locked` projects failed cells sorted by directory,
  `spawn_env_for` broadcasts after filling a failed cell and logs `env = ?attempted` only,
  `invalidate_env_include` / `set_env_include` broadcast when they removed a failure; file 2 passed
- refactor: none needed
- notes: A1 and A2 share one implementation step (T044). Workspace not yet compiled with the new
  field: other `CatalogSnapshot` literals still to update (T044 remainder, handover).
