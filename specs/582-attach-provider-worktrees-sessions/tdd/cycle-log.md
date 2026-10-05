# TDD cycle log: 582 attach provider worktrees and sessions

No `tdd/test-list.md` exists for this feature (the design phase did not run `tdd.plan`); the
test-first tasks of tasks.md (T003 before T004, T005 before T006, T007 before T010, T008 before
T011) served as the list. Red evidence below is the real failure of each test before its code.

## M1

- **T003/T004** `micold-core/tests/protocol_roundtrip.rs` (`every_client_message_json_round_trips`,
  `every_daemon_message_json_round_trips`). Red: `cargo test -p micold-core --test protocol_roundtrip`
  -> `error[E0599]: no variant named AttachDiscover found for enum ClientMsg`. Green: messages added,
  PROTOCOL_VERSION 27 -> 28, `schema_hash.rs` constant moved to 28 (hash is recomputed, not pinned).
- **T007/T010** `micold-core/tests/attach_discovery.rs`. Red: `unresolved import micold_core::attach::attachable_worktrees`.
  Green: 6 passed.
- **T005/T006** `micold-daemon/tests/attach_apply.rs` (catalog level). Red: `no method named attach_worktrees found for struct Catalog`.
  Green: 5 passed.
- **T008/T011** same file (daemon level). Written with the T011 handlers not yet present
  (`AttachDiscover` unhandled); green after `state.attach_discover/attach_apply` and the server arms: 9 passed.
- **T009/T012** `micold-client/tests/attach_dialog.rs`. The reducer and its tests were written in one
  pass, so there is no recorded red for these; the tests were then checked to fail on the specific
  behaviour only by inspection (summary strings, in-flight guard, project guard). Recorded as a gap.
  Green: 13 passed, plus `features_attach.rs` for the isolation guard.

## M2

- **T017/T022** `micold-core/tests/attach_discovery.rs` (`mod resumable`). Red: `unresolved import
  micold_core::attach::discover_resumable`. Green: `discover_resumable(&[StoreView], &DiscoverInput)`.
- **T018/T021** `micold-core/tests/provider_store_dirs.rs`. Red: `no method named store_dirs found`.
  Green: `AiCliProvider::store_dirs` (default: root and listed worktrees; Claude also lists store
  directories under the managed worktree folder) and `last_activity`.
  Deviations from the plan: `store_dirs` takes a third argument `worktrees: &[PathBuf]` and returns
  notes with the directories; `last_activity` is a new trait method; `DiscoverInput.only` lets an
  apply look up one id without reading every title.
- **T019/T023** `micold-daemon/tests/attach_apply.rs`. Red: `no method named attach_session found
  for struct Catalog`, then (state level) the discovery and session-apply tests failed with an empty
  `sessions` list. Green: 17 passed. Two tests seed the transcript after the client attaches,
  because feature 026's adoption pass (kept, plan P3) takes sessions of the root and of startable
  worktrees into the catalog on attach, which correctly removes them from the resumable list.
- **T020/T024** `micold-client/tests/attach_dialog.rs`. Red: `no variant named Resume found for enum
  Msg`, `no method footer_notes`, `no field sessions` (16 compile errors). Green: 21 passed.
- Wire: `RefuseReason::AlreadyRunning` was added. `PROTOCOL_VERSION` stays 28 (M1 already moved it for
  this feature and the schema hash is recomputed); `schema_hash` and `handshake` tests pass.
