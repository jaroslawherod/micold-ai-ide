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
| U6 | `EventHistory`: seq from 0, eviction past either bound counted in `omitted`, `bytes` = retained line text, defaults 5,000 / 64 MiB | R5, FR-018 | `micold-core/src/structured/history.rs::tests` (6 tests) | DONE |
| U7 | Wire delta: `SessionCreate.kind`/`SessionSummary.kind` default `Terminal`; `StructuredEvents` round-trips every event variant (JSON and postcard) | FR-001, FR-016, FR-020 | `micold-core/tests/structured_wire.rs` (3 tests) | DONE |
| U8 | Structured session over the daemon: argv/cwd/env = terminal's behind the seven structured args, no permission override; fixtures reach two viewers typed and in order; live before the result; tolerance lines kept; stderr to the log only; `SessionCreate` default terminal, Copilot structured refused, summaries carry the kind; two sessions isolated | FR-003, FR-005, FR-006, FR-006a, FR-010, FR-014, FR-016, FR-020, SC-001, SC-002, SC-006 | `micold-daemon/tests/structured_session.rs` (8 tests) | DONE |
| U9 | Structured lifecycle = terminal's: stop/start/crash restart/kill states, kill ends the tree, crash-loop give-up after as many starts, missing CLI fails alike, events kept after exit | FR-009 | `micold-daemon/tests/structured_lifecycle.rs` (4 tests) | DONE |
