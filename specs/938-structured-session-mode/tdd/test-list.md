# Test list: 938 structured session mode

Written by the M1 unit (no `/speckit.tdd.plan` list existed; tasks.md already orders each test task
before its implementation). Scope so far: M1 (T001–T010). Later milestones append their rows.

| ID | Behavior | Criterion | Test (file::name) | State |
| --- | --- | --- | --- | --- |
| U1 | The capture script's scrub replaces each secret with its placeholder | FR-024, R11 | `scripts/tests/capture-stream-json.test.sh` (7 cases) | DONE |
| U2 | `SessionKind`: default terminal, kept by the kind constructors, `check` refuses non-Claude structured, persisted without a key when terminal, no schema bump | FR-001–FR-004, SC-004 | `micold-core/tests/session_kind.rs` (10 tests) | DONE |
| U3 | `structured_args()` is the stream-json argv for Claude Code only | FR-005 | `micold-core/src/provider.rs::structured_args_tests::only_claude_code_has_structured_args` | DONE |
| U4 | `LineSplitter`: whole lines across chunks, one `\r` removed, 8 MiB unsplit, `finish()`, lossy UTF-8, empty lines | R4, FR-014 | `micold-core/src/structured/lines.rs::tests` (7 tests), `parse.rs::tests` (2) | DONE |
| U5 | `parse_line`/`pair_tool_calls` over the five recorded fixtures and `tolerance.jsonl` | FR-011–FR-015, FR-024, SC-001, SC-002 | `micold-core/tests/structured_parse.rs` (7 tests) | DONE |
