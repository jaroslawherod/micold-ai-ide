# Drift M1 (fbfa8360e..10f82e095)

- `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/structured.rs`: where the ReadStructuredEvents handler lives (T003-T004 name the request, not the files).
- `crates/micold-core/src/protocol/messages.rs`, `crates/micold-core/src/protocol/version.rs`, `crates/micold-core/tests/protocol_roundtrip.rs`, `crates/micold-core/tests/schema_hash.rs`: the wire change itself plus its round-trip and schema-hash pins.
- `docs/daemon.md`: documents the new request.
- `.github/workflows/ci.yml`: drops unreadable llvm-cov profiles (Follow-ups, corrupt `_1.profraw`).
