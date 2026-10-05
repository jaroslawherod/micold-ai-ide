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
