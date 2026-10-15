# Implementation Plan: Session Tree for Agent-Spawned Sessions

**Branch**: `feat/551_show-sessions-spawned-by-an-agent-as-a-tree-under` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/551-session-spawn-tree/spec.md`

## Summary

A session gains one optional, write-once fact: the session it was started from (its **origin**).
Three things follow from it, none of which stores a tree:

1. **Record** the origin from a native signal only. The one source is a provider-reported signal
   read by a new `AiCliProvider::reported_origin` seam when the daemon adopts or re-reads a
   provider-started session (FR-001). The tool server is not a source (FR-019, research R5):
   feature 776 refuses a second session in a worktree, and the user decided that agent-created
   sessions nest one level under the worktree's user session. The default of the seam is "no
   signal"; a provider is only given a reader once its signal is confirmed (research R2), so FR-002
   and the "keeps today's behaviour" story hold by construction.
2. **Carry** it: `StoredSession.origin` (additive serde default, no schema bump: FR-015, FR-006),
   `Session.origin`, `SessionSummary.origin` on the wire (protocol 46 to 47), and an `origin`
   field in the MCP `list_sessions`/`get_session` rows (FR-013).
3. **Show** it: a pure `SessionTree` in `micold-core` derived from a worktree's sessions, consumed
   by the client's render-free `features/sidebar.rs` and rendered by recursion in
   `ui/sidebar.rs` through the existing `TreeView`/`TreeItem` (depth is already a parameter).
   Fold state is a new additive settings field (research R6).

The user guide gains a provider table (FR-014).

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`

**Primary Dependencies**: existing only: `iced` (client), `serde`/`serde_json`, `uuid`, `tokio`
(daemon). No new crate.

**Storage**: local JSON only. `projects.json` per-project catalog (`StoredSession.origin`,
optional) and the settings file (`folded_origins`, optional). Both additive with `serde(default)`.

**Testing**: `cargo test` via `mise run test-core`, `mise run gate`; daemon integration tests
(`crates/micold-daemon/tests/`), client reducer tests (`crates/micold-client/tests/`), layout
snapshot gates (`layout_snapshot.rs`), `visual-pass` for quickstart Part B.

**Target Platform**: Linux, macOS, Windows (Principle VI). No platform branch: ids are UUIDs,
provider stores are read through the providers' existing path derivation.

**Project Type**: desktop-app: `micold-core` (render-free logic, protocol, MCP schema),
`micold-daemon` (single catalog writer, MCP server), `micold-client` (iced).

**Performance Goals**: tree build is O(n) in a worktree's sessions; dozens of children stay
single-frame (SC in Edge Cases).

**Constraints**: offline; origin immutable once recorded; no client write to `projects.json`.

**Scale/Scope**: tens of sessions per worktree, dozens of children under one origin.

## Constitution Check

*Gate before Phase 0, re-checked after Phase 1 design.*

| Principle | Result | How |
|---|---|---|
| I. Test-First | PASS | Every production unit has a failing test first; `tasks.md` orders test tasks before code (test list under Test strategy). Sidebar view glue in `ui/sidebar.rs` that only recurses over the tested `SessionTree` falls under the constitution's GUI-wiring exception and is covered by the layout gate plus quickstart Part B. |
| II. Multi-Session | PASS | Origin is per-session, durable and restorable. It names another session by id but shares no state: deleting or stopping the origin does not touch a child (FR-018). Every window derives the same tree from the same snapshot (FR-012). |
| III. Worktree | PASS | An origin must be a session of the same project and worktree (FR-009), so no cross-location link exists; Default sessions nest under Default sessions only. No worktree operation is added. |
| IV. Local-First | PASS | Reads provider stores already on disk; nothing remote. |
| V. Rust + iced | PASS | Rust and iced only. `OriginStatus` and `SessionTree` make a cycle or a self-parent unrepresentable in the built tree (a node has either a parent in the same map or none); the write path rejects them (R4). |
| VI. Cross-Platform | PASS | No OS branch; CI matrix unchanged. |
| VII. Documentation | PASS | `docs/user-guide/worktrees-and-sessions.md` section plus provider table; `docs/user-guide/agent-tools.md` row fields. |
| VIII. Reusable UI | PASS | Reuses `TreeView`/`TreeItem` (`expandable`, `annotation`, `badge`). The child marker and the folded-count/attention summary are added as `TreeItem` builder methods (shared primitive), not a sidebar-local widget; showcase entry updated. |

No violations; Complexity Tracking is empty.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002 | `AiCliProvider::reported_origin` (provider.rs), `Catalog::record_origin`; R2 |
| FR-003, FR-004, FR-005 | `micold_core::origin::SessionTree`, `features/sidebar.rs::worktree_tree`, `ui/sidebar.rs` |
| FR-006, FR-015 | `StoredSession.origin`; data-model.md; `store.rs` round trip |
| FR-007 | child rows reuse `session_tree_item`; marker via `TreeItem::annotation`/new `child_marker` |
| FR-008, FR-016, FR-018 | `OriginStatus` (Known / Removed) on the wire, `Pending` derived in `SessionTree`; data-model.md |
| FR-009, FR-010 | `origin::validate_origin`, `Catalog::record_origin`; R4 |
| FR-011, FR-017 | `TreeItem::expandable`, `Settings.folded_origins`; R6 |
| FR-012 | derived from `SessionChanged`/snapshot; no new broadcast; contracts/origin-protocol.md |
| FR-013 | `SessionSummary.origin`, MCP rows; contracts/origin-protocol.md, contracts/mcp-origin.md |
| FR-014 | user guide; contracts/provider-signals.md |
| FR-019 | `Catalog::create_session` and the tool take no origin; `record_origin` is the only writer; R5. Touches 776's `create_session` path only by not adding to it; 776's refusal is unchanged (issue #802 owns any change) |

## Project Structure

### Documentation (this feature)

```text
specs/551-session-spawn-tree/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── origin-protocol.md      # wire + durable + settings shapes
│   ├── mcp-origin.md           # list_sessions / get_session row additions
│   └── provider-signals.md     # per-provider reader contract and signal table
└── tasks.md                    # next unit
```

### Source Code

```text
crates/micold-core/src/
├── origin.rs            # NEW  validate_origin, SessionTree, OriginStatus (pure)
├── session.rs           # Session.origin
├── store.rs             # StoredSession.origin + origin_rejected (serde default, skip if none/false)
├── provider.rs          # AiCliProvider::reported_origin (default None) + confirmed readers
├── settings.rs          # Settings.folded_origins (additive)
├── protocol/{messages,version}.rs   # SessionSummary.origin, PROTOCOL_VERSION 47
└── mcp/tools.rs         # row schema text
crates/micold-daemon/src/
├── catalog.rs           # record_origin, archive paths emit child SessionChanged, session_summary overlay
├── state.rs             # discover_external_sessions: reported_origin for adopted sessions; origin-removed broadcast
└── mcp/tools.rs         # session_row.origin (create_session unchanged)
crates/micold-client/src/
├── features/sidebar.rs  # worktree_tree nests via SessionTree; fold msgs/state
├── ui/sidebar.rs        # recursion in group_items/build_default_item; summary on folded rows
└── ui/material/tree_view.rs   # TreeItem child marker + folded summary builder methods
docs/user-guide/{worktrees-and-sessions,agent-tools}.md
crates/*/tests/          # see Test strategy
```

**Structure Decision**: logic in `micold-core` (testable without GUI), the daemon as the only
writer, the client derives the tree; no new crate or file format.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-009, FR-010 (validate, cycle, once) | core unit: `origin.rs` tests |
| FR-003, FR-004, FR-005, FR-008, FR-016, depth, order | core unit: `SessionTree`; client reducer test `sidebar_tree.rs` |
| FR-006, FR-015 | core `store_roundtrip.rs` (old file loads, new field survives); daemon test restart via `Catalog::load` |
| FR-001, FR-002 | core provider tests with fixtures per confirmed provider; a "no signal" provider test proving a name/path/timing never yields an origin |
| FR-019, FR-009 | daemon `mcp_create_session.rs` (a created session has no origin and no input names one; rows show a recorded origin) |
| FR-013 | core `protocol_roundtrip.rs`, `mcp_tools_catalog.rs`; daemon MCP row test |
| FR-012 | daemon: two subscribers see the same summary |
| FR-011, FR-017 | client reducer: fold, counts, attention rollup; settings round trip (core) |
| FR-007, SC-004 | layout gate `layout_snapshot.rs` rows (child marker present on children only); quickstart Part B |
| FR-014, SC-005 | docs CI link check; a test that the provider table lists every `AiCli` variant |
| US1 Default location, US2.2 | client reducer test on `build_default_item` tree; daemon restart test: live child becomes interrupted-resumable and stays nested |
| FR-018 | daemon: single delete (`archive_session`), bulk archive and worktree archive; delete/stop origin leaves child present and top-level; a subscribed client receives the child as `Removed` without reconnecting |
| FR-017 (restart) | settings round trip; reducer: fold, restart, snapshot arrives, still folded (no pruning) |

## Risks

- **No Claude Code signal** (research R2, spike T025): Claude Code ships as a documented gap, and
  the live path is the providers that report (OpenCode, Codex). Claude Code is the provider the
  issue names; closing its gap needs a recorded `claude --bg` session record with a parent field,
  which the spike host could not read. This is the spec's own fallback (US3).
- Protocol bump is breaking for a mismatched client and daemon; the existing handshake handles it
  (R7).
