# Implementation Plan: Daemon-qualified refs and the routed envelope (protocol bump)

**Branch**: `feat/1723_contract-daemon-qualified-refs-and-routed-envelope` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1723-daemon-qualified-refs-routed-envelope/spec.md`

## Summary

Add to `micold_core::protocol` a validated daemon id, daemon-qualified refs for session, worktree and
project, a derived project identity, and a routed envelope carried as a new frame `Kind` on the
existing framed stream. Rows (`SessionSummary`, `WorktreeSnapshot`, `ProjectSnapshot`) gain the
owning daemon's id, `ProjectSnapshot` also its identity. `PROTOCOL_VERSION` 77 -> 78 and the schema
hash covers the new source file. The daemon fills the id from a new optional `daemon.id` setting
(default `local`). No orchestrator, routing or link (#1724+). Decisions and rejected alternatives:
[research.md](research.md).

## Technical Context

**Language/Version**: Rust, workspace edition as in `Cargo.toml`.

**Primary Dependencies**: existing only: `serde`, `serde_json`, `tokio_util::codec`, `bytes`. No new crate.

**Storage**: N/A. The daemon id is configuration; nothing is persisted or sent off the device (Principle IV).

**Testing**: `cargo test -p micold-core` (unit, `tests/protocol_roundtrip.rs`, `tests/handshake.rs`,
`tests/schema_hash.rs`), `micold-daemon` integration tests. No GUI is touched: no geometry gate, no
quickstart §B visual pass.

**Target Platform**: Linux, macOS, Windows (ids and refs are text and integers).

**Project Type**: Rust workspace, render-free `micold-core` (shared wire) plus `micold-daemon` and `micold-client`.

**Performance Goals**: none; one extra JSON object per routed frame.

**Constraints**: exact-match handshake, no compatibility shim (`version.rs` rule); one framed stream, not split.

**Scale/Scope**: 1 new core module, 1 new `Kind`, 3 row types gain fields (about 76 grep hits for
the struct names, mostly test fixtures; the tasks unit adds one fixture constructor per row type that
defaults `daemon` to `local`, extending existing test builders where they exist), 1 daemon config key, 2 docs.

## Constitution Check

| Principle | Result |
|---|---|
| I. Test-First | PASS. Tests lead each piece; list in Test strategy. The tdd skills derive the order in the tasks unit. |
| II. Multi-Session | PASS. A session ref is `(daemon, SessionId)`; nothing is shared between sessions. |
| III. Worktree Integration | PASS. No git or worktree operation changes; worktree ref is `(daemon, project, dir_name)`, the pair the wire already uses. Reads local git config for the project identity, never writes. |
| IV. Local-First (NON-NEGOTIABLE) | PASS. Identity is derived from local git config only (`remote_list`, one local-config read); no fetch, no service. |
| V. Rust + iced | PASS. Newtypes with validating constructors (`WireDaemonId`, `ProjectIdentity`, `OverrideValue`) make invalid ids unrepresentable; no iced code. |
| VI. Cross-Platform | PASS. Ids are validated ASCII text, counters are integers, a Windows path never enters a remote-derived identity. Tests are platform-neutral. |
| VII. Documentation | PASS. `docs/daemon.md` section and the contract under `specs/orchestrator/contracts/` ship in the same PR (FR-009). |
| VIII. Reusable UI | N/A. No UI. |

Complexity Tracking: none.

## Requirement map

| FR | Where |
|---|---|
| FR-001 | `WireDaemonId` in `protocol/routing.rs`: [data-model.md](data-model.md) |
| FR-002 | `SessionRef`, `WorktreeRef`, `ProjectRef` in `routing.rs`, derived `Eq`/`Hash` |
| FR-003 | `daemon` field on the three row types in `messages.rs`; `qualified()` builders on each |
| FR-004 | `RoutedEnvelope<M>`, `ConnectionId`, `Sequence` in `routing.rs` |
| FR-005 | `RoutedEnvelope::from_json_slice` checks the target first; `RoutingError` |
| FR-006 | `Kind::Routed = 2` in `envelope.rs`; `Frame::Routed` and a routing mode in `codec.rs` |
| FR-007 | `PROTOCOL_VERSION` 78; `build.rs`/`hashing.rs` hash `routing.rs`; refusal already names both |
| FR-008 | `core/src/project_identity.rs` (`ProjectIdentity::derive`); override in git local config key `micold.projectId` (R3) |
| FR-009 | `docs/daemon.md`; `specs/orchestrator/contracts/routed-envelope.md` (new file only) |
| FR-010 | `Routing::Plain` stays the default of `WireCodec::new()`; a daemon test drives an unwrapped client |

## Design

### Wire types (`crates/micold-core/src/protocol/routing.rs`, new)

Hashed into `SCHEMA_HASH` (R4). Contains `WireDaemonId`, `SessionRef`, `WorktreeRef`, `ProjectRef`,
`ConnectionId(u64)`, `Sequence(u64)`, `RoutedEnvelope<M>`, `RoutingError`. Field-level shapes in
[data-model.md](data-model.md). `WireDaemonId` is not `daemons::DaemonId` (a client-local `u32`
registry slot, `crates/micold-core/src/daemons.rs`); the name differs on purpose.

### Frame codec (`envelope.rs`, `codec.rs`)

- `Kind::Routed = 2`; body is JSON, `encoding` byte must be `Json`.
- `Frame<C>` gains `Routed(RoutedEnvelope<C>)`. Matches on `Frame` in `micold-daemon/src/server.rs`
  (line 884) and `micold-client/src/daemon.rs` (line 1154) get an arm: the daemon rejects a routed
  frame on a plain connection with a close, the client treats one as a protocol violation.
- `WireCodec` gains `Routing { Plain, Routed }`, default `Plain`, set by `with_routing`. `Plain`
  rejects `Kind::Routed`; `Routed` rejects `Kind::Control`; grid frames pass in both. The mismatch
  is `CodecError::WrongRouting { expected, got }`. Routing errors: `CodecError::Routing(RoutingError)`.
- Handshake: `Hello`, `Welcome` and `Refused` are `Frame::Control` (`connect.rs`), so a routed link
  handshakes on a `Plain` codec and calls `WireCodec::set_routing(Routing::Routed)` after `Welcome`;
  the refusal path (FR-007) is therefore unchanged. A `Hello` is never wrapped. Test: a routed link
  completes the handshake plain, then rejects a `Control` frame and accepts a `Routed` one.
- `Routed` is encodable and decodable only where `Out`/`In` is `ClientMsg` (a `DaemonCodec` decodes
  it; a `ClientCodec` encodes it); a `ClientCodec` that receives one gets `WrongRouting`. Other
  `Frame` matches (`connect.rs`, `provision/mod.rs`, `kube/fake_host.rs`, daemon test supports) are
  found by the compiler in the tasks unit; each gets a rejecting arm or already uses `if let`.
- Routed frames travel the `ClientMsg` direction only here; the daemon-to-client wrap belongs to #1725.

### Rows and the daemon id source

`daemon.id` is added to `[daemon]` in `crates/micold-core/src/daemon_config/mod.rs`, validated by the
`WireDaemonId` constructor in `validate.rs`; absent means `local`. The daemon passes it to
`catalog.rs` (`session_summary`, the `ProjectSnapshot`/`WorktreeSnapshot` builders at lines 365, 380,
2220) and `state.rs` line 2419. The orchestrator will overwrite it with the registry/certificate id
(M2); a daemon's self-claim is never trusted there (spec Key Entities).

### Project identity (`crates/micold-core/src/project_identity.rs`, new)

`ProjectIdentity::derive(remotes, override, daemon, project_path)`: override value wins; else the
`origin` remote (first remote in config order when there is no `origin`), normalised by the FR-008
steps using the existing `github::split_remote` (made `pub(crate)`); a local-path remote counts as no
remote (including a drive-letter path such as `C:/x` or `C:\x`, which `split_remote` would read as
scp host `C`: a single-letter host is rejected before it is called); no remote gives `local:<daemon>:<path>`. Tagged strings (`remote:`, `override:`, `local:`)
keep the three forms from colliding. The override is read through a new `Git::config_local_get(repo,
key)` (local config only, like `remote_list`); implementors: `GitCli` (git.rs:413, pattern `remote_list` at 574) and `FakeGit` (git.rs:1284).

### Handshake (FR-007)

`RefusalReason::VersionMismatch` already carries both versions; the new test runs the handshake with
`PROTOCOL_VERSION - 1` and asserts both numbers in the refusal text.

## Test strategy

| Requirement | Layer | Test |
|---|---|---|
| FR-001, edge cases for ids | core unit (`routing.rs`) | valid/invalid ids, JSON and postcard round trip |
| FR-002, SC-001 | core integration `tests/protocol_roundtrip.rs` | every ref and row type round trips; same local ref on two daemons is unequal |
| FR-003 | core + daemon | row builders carry the id; a ref built from a row equals the decoded ref |
| FR-004, FR-006 | core integration | envelope around each `ClientMsg` variant through `WireCodec`; plain/routed mix-ups rejected both ways; plain handshake then switch |
| FR-005, SC-002 | core unit | missing, empty, malformed, double-wrapped target cases; payload not interpreted on bad target |
| FR-007, SC-003 | `tests/handshake.rs`, `tests/schema_hash.rs` | previous-version refusal names both; hash covers `routing.rs` |
| FR-008 | core unit (`project_identity.rs`) | URL equivalences of User Story 4, override, no remote, two no-remote repos differ, `C:/x` and `C:\x` give no remote |
| FR-009, SC-004 | core test reads the contract file | worked-example frame in the contract decodes; docs headings checked by the docs gate |
| FR-010, SC-005 | daemon integration | unwrapped client against one daemon still works; existing suites pass with the field added |
| Geometry gates, quickstart §B | n/a | no UI |

## Project Structure

```text
specs/1723-daemon-qualified-refs-routed-envelope/
  spec.md plan.md research.md data-model.md quickstart.md contracts/routed-envelope.md  (pointer)
specs/orchestrator/contracts/routed-envelope.md        (new file; the contract)
crates/micold-core/src/protocol/routing.rs             (new)  + envelope.rs codec.rs messages.rs version.rs hashing.rs mod.rs
crates/micold-core/src/project_identity.rs             (new)  + lib.rs git.rs github.rs daemon_config/
crates/micold-core/build.rs                            (hash input)
crates/micold-daemon/src/{catalog,state,server}.rs     (fill daemon, routed-frame arm)
crates/micold-client/src/daemon.rs                     (routed-frame arm) + fixtures
docs/daemon.md
```

**Structure Decision**: new files hold new concepts; the existing files change only where the spec
forces it (rows, `Kind`, version, hash input).
