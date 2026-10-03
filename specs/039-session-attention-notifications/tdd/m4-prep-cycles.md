# M4 prep cycles (feature 039)

Written on branch `feat/notify-session-needs-attention-m4-prep`, built from `origin/main` at the M2
merge while M3 ran. The M4 unit merges these notes into `cycle-log.md` and ticks `tasks.md`; this
branch edits neither.

Batching, as in M1 and M2: the build lock is shared, so the tests of one task are written
together, seen red in one run and made green in one run.

## Cycle M4-1 — U9, U10, U11, U175 — T045, T052

- Tests (`crates/micold-core/src/store.rs`, `mod unread_tests`):
  `a_session_stored_without_unread_reads_as_read`, `a_store_round_trip_keeps_unread`,
  `unread_is_written_to_the_sessions_state_file_and_to_no_other_file`; and
  `crates/micold-core/tests/store_fault_isolation.rs`:
  `a_state_file_that_cannot_be_parsed_leaves_no_session_unread_and_reports_nothing_new` (U175,
  characterization: it needs the field to compile and was green as soon as it did).
- Red (`mise run test-core`): `error[E0609]: no field 'unread' on type 'session::Session'`
  (`store.rs`, three places).
- Green: `Session::unread`, `StoredSession::unread` with `#[serde(default)]`. `mise run test-core`
  exit 0, the four tests `ok`.
- **Deviation from T045's wording**: "written to the catalog file and to no other file". Sessions
  are not in the catalog file (`projects.json`) since the per-project split; `StoredSession` is
  written to the project's state file, where `attention_seq` is. The test asserts that the only
  file under the store directory naming `"unread"` is `project_state_path(project)`.

## Cycle M4-2 — U14 — T046, T053 (field only; the version number is the last commit)

- Tests: `protocol::messages::attention_wire_tests::a_session_summary_carries_unread`;
  `crates/micold-core/tests/schema_hash.rs`: `unread_is_in_the_hashed_source`.
- Red: `error[E0560]: struct 'messages::SessionSummary' has no field named 'unread'`.
- Green: `SessionSummary::unread`; every `SessionSummary` literal of the workspace gains
  `unread: false` (the service's `session_summary` too, until cycle M4-4 makes it carry the
  session's value). `mise run test-core` exit 0.
- `PROTOCOL_VERSION` is **not** bumped in this cycle. `SCHEMA_HASH` is computed by `build.rs`, so
  it has changed with the field. The bump, its doc comment and the pin in `schema_hash.rs` are one
  commit at the end of the branch, because the number is "next free" after M3 merges.
