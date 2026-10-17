# Implementation Plan: Review Comments the Agent Answers — Review Panel, Stable Batch, Replies and Re-anchoring

**Branch**: `claude/project-thread-jjniz9` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/946-review-replies/spec.md` (issue #946), extending
feature 482 (`specs/482-worktree-changes-review/`).

## Summary

Comments gain four statuses (Draft, Sent, Applied, Answered) and an agent reply, all in core
`review::comment` and persisted by the daemon in the review file (version 2). The batch becomes a
documented, byte-stable preamble plus one self-contained block per comment, each with a short
reference (`C-` + 8 hex of the comment's UUID). The agent answers each comment through a new
`report_review_comment` tool on the daemon's agent tool server (no confirmation; own entry only);
the user can mark a Sent comment by hand. A pure `review::anchor::locate` moves a comment to where
its quoted code now is (nearest single exact match, tie or absence → outdated); the client applies
it on display and stores moves, the daemon applies it to drafts at send. A review panel beside the
diff lists every comment of the entry, built from a new shared `ReviewPanelRow`. Delivery stays
482's terminal path only (D3).

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`, MSRV 1.97

**Primary Dependencies**: iced 0.14 (existing); no new crate

**Storage**: `reviews/<project id>.json` (482-R3), `version` 1 → 2, reads both (research R7)

**Testing**: `mise run test-core` (core units and `crates/micold-core/tests/`), `scripts/gate.sh`
(daemon integration tests with the fake CLI, client render-free state tests, geometry gates,
docs), quickstart §B visual pass for the render glue

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (workspace: `micold-core`, `micold-daemon`, `micold-client`)

**Performance Goals**: 200 comments: panel open and send with no frame over 100 ms (SC-006); a
report visible in every window within 2 s (SC-005)

**Constraints**: batch byte-identical across platforms, no `\r` (FR-012); reply ≤ 20,000 chars,
shown shortened over 4,000; reference ≤ 12 chars; offline, local only

**Scale/Scope**: one entry's comments (hundreds), one file's lines for re-anchoring (482's diff
size limits)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
all PASS, no deviation.

- [x] **I. Test-First**: every rule lives in tested render-free code: core `comment`, `anchor`,
  `prompt`, `reply`, `store`, `mcp::tools`/`policy`; daemon integration tests for the tool, edits
  and send; client `features::changes` for anchors, panel rows, counts, jumps and actions. Only
  `ui/changes.rs` glue and the material components' drawing use the §B pass; the components keep
  their in-crate geometry gates.
- [x] **II. Multi-Session**: comments stay per entry; a session reports only on its own entry's
  comments, resolved from the tool server's caller identity (FR-004); concurrent reports are
  serialised by the daemon's single writer, last wins.
- [x] **III. Worktree Integration**: entries are worktrees or Default; the report tool changes no
  worktree, so a Default session may use it on the Default entry only; files are read from the
  entry's own root.
- [x] **IV. Local-First**: all state in the local review file; the batch goes only to a local CLI
  session (482).
- [x] **V. Rust + iced**: Rust and iced only; `CommentState` makes a reply on a Draft or Sent
  comment unrepresentable; `Report` is built only by validating constructors.
- [x] **VI. Cross-Platform**: core logic platform-agnostic; `\r` normalised in batch, replies and
  anchoring; the P1 golden test runs on all three CI platforms.
- [x] **VII. Documentation**: `docs/user-guide/reviewing-changes.md` (panel, statuses, marking,
  following code, the batch format with the contract's example) and `docs/user-guide/agent-tools.md`
  (the tool) ship in the same PRs as the behaviour.
- [x] **VIII. Reusable UI**: new `ReviewPanelRow` in `ui/material/` with a builder; the existing
  `ReviewCommentCard`, `Tag`, `virtual_rows`, `dialog`, `line_clamp` are extended or reused; both
  appear in the showcase.

## Project Structure

### Documentation (this feature)

```text
specs/946-review-replies/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/  batch-format.md  agent-tool.md  review-wire-delta.md
└── tasks.md   (tasks unit)
```

### Source Code (repository root)

```text
crates/micold-core/src/review/
├── comment.rs     CommentState/Status/Report/Outcome/Reporter, CommentRef, EntryReview::{report,mark,reanchor}
├── anchor.rs      NEW locate / Located / is_outdated
├── prompt.rs      new batch layout, REPORT_TOOL
├── reply.rs       NEW display / shortened
└── store.rs       version 2, v1 compat, reference collision re-id on load
crates/micold-core/src/mcp/{tools.rs,policy.rs}       Operation::ReportReviewComment, catalog entry
crates/micold-core/src/protocol/{messages.rs,version.rs}  ReviewEditOp::{Mark,Reanchor}; 48 → 49
crates/micold-daemon/src/review.rs        Reviews::{report,mark,reanchor}; send-time re-anchoring
crates/micold-daemon/src/state.rs         review_send reads drafts' files off the lock
crates/micold-daemon/src/mcp/tools.rs     report_review_comment handler
crates/micold-daemon/tests/{review_edit.rs,review_send.rs,review_reports.rs (new)}
crates/micold-client/src/features/changes.rs   anchors, panel, jump, mark, reanchor effect
crates/micold-client/src/ui/changes.rs         panel pane, card states (glue)
crates/micold-client/src/ui/material/{review_comment.rs,review_panel_row.rs (new),mod.rs}
crates/micold-client/src/showcase/sections/review.rs
crates/micold-client/tests/features_changes.rs
docs/user-guide/{reviewing-changes.md,agent-tools.md}
```

**Structure Decision**: the existing three-crate split. Rules in `micold-core` (pure), the single
writer and the tool in `micold-daemon`, render-free view state in `micold-client/src/features/`,
drawing in `ui/`.

## Test strategy by layer

| Layer | Where | Covers |
|---|---|---|
| Core unit | `review/comment.rs` | FR-001–006 transitions and refusals, reference format and uniqueness, clear sent, reanchor compare-and-set |
| Core unit | `review/anchor.rs` | US4 s1–6, FR-020–025, CRLF, clamping, tie → ambiguous, SC-004 property over shifted files |
| Core unit | `review/prompt.rs` | contracts/batch-format.md P1–P9 |
| Core unit | `review/reply.rs`, `review/store.rs` | control characters, shortening; v1 load, v2 round-trip, collision re-id |
| Core integration | `tests/mcp_tools_catalog.rs`, `mcp_policy.rs`, `protocol_roundtrip.rs` | tool listing, parsing, validation, `Proceed`, wire round-trips |
| Daemon integration | `tests/review_reports.rs`, `review_edit.rs`, `review_send.rs` | agent-tool.md tests; W14–W16; restart persistence; two clients see pushes |
| Client state | `tests/features_changes.rs` | anchors drawn and outdated marks (FR-024), Reanchor effect only for the working tree, panel rows/grouping/counts, jump vs hidden reason, draft-only edit/delete, manual mark composer, 200-comment row build |
| Manual | quickstart §B, §C | glue in `ui/changes.rs`, look of the panel and cards |

## Delivery order (for the tasks unit)

1. **M1 core**: statuses, reference, anchor, batch, reply, store v2, wire ops, MCP operation.
2. **M2 daemon**: report tool, Mark, Reanchor, send-time re-anchoring; agent-tools.md.
3. **M3 client**: card states and replies, anchors on display, panel and shared row, manual mark,
   showcase; reviewing-changes.md.

Each milestone leaves the app working: M1 changes the batch text and store (docs for the format go
with it), M2 makes reports possible (visible as text in the existing cards' state until M3).

## Complexity Tracking

No constitution violation to justify.
