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

## Cycle M4-3 — U59, U60 — T047, T055 (the `set_view` part)

- Tests (`crates/micold-daemon/src/attention.rs`): `a_report_returns_the_session_that_came_into_view`,
  `a_report_naming_the_same_session_or_none_returns_nothing`.
- Red (`cargo test -p micold-daemon --lib attention`): `error[E0308]: mismatched types`, five
  places (`set_view` returned `()`).
- Green: `Views::set_view` returns `Option<SessionId>`. `--lib attention` 12 passed. T004's tests
  keep their assertions and ignore the return value.

## Cycle M4-4 — U84 to U96, A14, A15, A19 to A22, A24 to A26, A31 to A36 — T048, T054, T055

- Tests: `crates/micold-daemon/tests/unread_state.rs` (new), 14 tests, harness copied from
  `attention_events.rs`.
- Red: `test result: FAILED. 2 passed; 12 failed`. The two that passed assert that a session is
  *not* unread (`a_change_while_a_window_has_the_session_in_view_does_not_set_unread`,
  `a_restart_of_the_service_keeps_a_read_session_read`); they hold trivially before the feature
  and constrain it afterwards.
- Green: `Catalog::mark_attention` sets `unread`; `Catalog::mark_read`; `session_summary` carries
  it; `DaemonState::set_window_view` reads the session and broadcasts.
  `unread_state` 14 passed, `attention_events` 13 passed, `attention_claims` 5 passed.
- **Deviation from T054's wording** ("`mark_read(session)` clears it and persists"): `mark_read`
  changes memory only and `set_window_view` sets `attention_unsaved`, so the supervisor tick's
  `persist_attention` writes it, as M1 did for `mark_attention` (the state lock is held on the
  async runtime, and a store write is blocking I/O). The restart tests call `persist_attention`
  as U77 does.
- **No test** observes the tick's write of a read (the M1 gap for the attention event, unchanged).

## Cycle M4-5 — U125, U126, U127, A14, A36 — T051, T056

- Tests: `crates/micold-client/tests/unread_rows.rs` (new), 5 tests.
- Red: the client crate did not compile (cycle M4-6's errors came first in the same run);
  `row_unread` and `in_view` did not exist in `features::attention`.
- Green: `features::attention::row_unread(&Session, Option<SessionId>)`,
  `features::attention::in_view(&State)` (the session of the last view report sent), `unread`
  copied in both arms of `reconcile_catalog`. `unread_rows` 5 passed.
- **Not done: `--test unread_rows` in `.github/workflows/ci.yml`** (T051's last sentence). M3 owns
  the workflow files while this branch was written; the M4 unit adds the line.

## Cycle M4-6 — U145 to U151 — T049, T050, T057, T058

- Tests: `ui/material/unread_mark.rs` (3), `ui/material/tree_view.rs` (4, the file's first test
  module), `composition_contrast.rs::the_unread_mark_is_legible_on_every_fill_a_host_draws_it_on`,
  `anatomy_size.rs::an_unread_mark_is_8dp_in_both_axes` and
  `an_unread_tree_row_stands_at_its_densitys_height`.
- Red (`cargo test -p micold-client --lib --test unread_rows`): 19 compile errors, among them
  `cannot find 'UnreadMark' in 'super'`, `cannot find function 'fill' in module
  'super::unread_mark'`, `no method named 'unread' found for struct 'tree_view::TreeItem'`.
- Green: `UnreadMark::new(roles)` with `.count(n)` and `.worded(bool)`; `TreeItem::unread(bool)`.
  `cargo test -p micold-client` exit 0, 152 result lines `ok`, none failed.
- Design choices the M4 unit and M5 should know:
  - The mark is a `container` 8dp square with the `full` corner, filled by
    `unread_mark::fill(roles)` (`primary`), not a glyph: a glyph's dot is smaller than its em box.
  - The count and the word are drawn in `TypeRole::Label` in the theme's default text colour.
    Contract U1 says "the host's label role and colour"; no M4 host shows text, so no setter for
    the role exists yet. M5 adds it with its hosts.
  - The emphasised weight of an unread row is the view's `selected_label_role` (the sidebar's
    `SidebarSessionCurrent`); a view that sets none draws the label as the others. No new setter.
  - The contrast test covers four fills: the sidebar, a selected row (`secondary_container`), a
    menu panel (`surface_container`) and the app bar (`surface`).

## T059, T060, T061 (no behavior id)

- T059: `ui/sidebar.rs::session_tree_item` takes the session in view and calls
  `.unread(row_unread(session, in_view))`.
- T060: catalogue entry `material/unread_mark.rs` / `UnreadMark`, rendered by
  `sections::atoms::unread_mark`: a session row with the mark above one without. The showcase
  gates (`showcase_completeness`, `showcase_captions`, `material_builder_api`) passed in the run
  above. **No visual pass was run** on this branch.
- T061: `docs/user-guide/worktrees-and-sessions.md`, new section *Unread sessions*, placed before
  *Being told when a session needs you* so that it does not touch the lines M3 edits.

## Protocol version (T046's pin, T053's bump) — last commit

- `PROTOCOL_VERSION` 23 → 24 in `version.rs`, `FEATURE_026_PROTOCOL_VERSION` 24 in
  `schema_hash.rs`, `docs/daemon.md` "version 24 today". One commit, the last code commit of the
  branch. If M3 or another feature takes 24 first, this commit is the only one that moves.
