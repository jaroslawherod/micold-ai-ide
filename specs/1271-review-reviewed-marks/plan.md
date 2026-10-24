# Implementation Plan: Review UI — reviewed marks and progress

**Branch**: `claude/project-thread-umhor9` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1271-review-reviewed-marks/spec.md`

## Summary

A **reviewed mark** is a `ReviewMark { scope, path, change: ChangeId }`, kept by the session
service in the entry's `EntryReview` beside its comments, so 482's storage, push and removal paths
carry it (research R3). `ChangeId` is a SHA-1 digest of the file's change: for the working tree,
the file's kind, path, modes and both sides' git object ids, read by a new toggle-independent
`GitCli::whole_change` beside every list read (R1, R2); for a pull request, the file's patch
section (R1). The client derives each row's state (○ / ● / ✓, changed dot) and *n of m reviewed*
from marks, drafts and current ids with pure core functions (`review::marks`), and marks or
unmarks with one idempotent edit, `ReviewEditOp::SetReviewed`, answered by the existing
`ReviewChanged` push, which gains `marks` (R4). Review file 4 only with marks (R5), protocol +1
(R6). D4 (provisional) is one function, `mark_block` (R7). Core, wire, daemon and the client's
receipt of marks build and merge before #1269; the view's controls after it (R8).

## Technical Context

**Language/Version**: Rust stable (via `mise`), iced GUI.

**Primary Dependencies**: existing crates: `micold-core` (`review::{comment, changes, git,
pr_diff, store}`, new `review::marks`, `protocol::{messages, version}`, `git::Git`,
`task_progress::Ratio`), `micold-daemon` (`review.rs`, `state.rs`, `server.rs`), `micold-client`
(`features/changes.rs`, `shell/{changes, daemon_sync}.rs`, `ui/changes.rs`, `icons.rs`,
`ui/material::{ProgressBar, ToggleChip, UnreadMark, Tooltip, Glyph}`). One direct dependency
added to `micold-core`: `sha1` 0.10, already in `Cargo.lock` (R1).

**Storage**: `reviews/<project id>.json` (482), version 4 only while some entry has marks (R5).
Local only.

**Testing**: `mise run gate`; core unit tests inline in `review/marks.rs`, `review/store.rs`,
`review/changes.rs`, `review/pr_diff.rs`; `crates/micold-core/tests/{review_git, protocol_roundtrip,
schema_hash}.rs`; daemon `tests/{review_edit, review_send}.rs`; client reducer tests
`tests/features_changes.rs`, `tests/features_pr_changes.rs`; layout snapshot
(`tests/support/covered_states.rs`, `tests/fixtures/layout_snapshot.txt`); quickstart Part B.

**Target Platform**: Linux, macOS, Windows. Git runs as argument vectors; ids are bytes over
`/` paths.

**Project Type**: desktop app (Rust workspace) with a session service (daemon).

**Performance Goals**: SC-002: a mark shows in the same window within 0.5 s (one local round trip,
no git read) and in another window within 2 s (482's push). Per re-read, the whole-change read adds
three `git diff --raw` reads and one `git hash-object` over dirty and untracked files (R2).

**Constraints**: offline; nothing leaves the device; the mark is judged on the whole change
whatever the toggles (FR-005); no optimistic update (Edge Cases: a failed request leaves the state).

**Scale/Scope**: 30-file reviews (SC-001); marks grow with files marked, one per file per source;
pull request marks are kept until unmarked or the entry goes (FR-016).

## Constitution Check

*Re-checked after Phase 1: unchanged.*

- [x] **I. Test-First**: ids, whole-change assembly, `file_review`, `progress`, `mark_block`,
  `EntryMarks`, the store and the wire are pure core logic, written test-first (FR-014); the git
  read is tested against real temp repos; daemon rules against the in-process daemon with fake
  sinks; the client's state, control and progress models in render-free reducer tests. Only
  `ui/changes.rs` drawing and `shell/` wiring use the GUI-glue exception and carry no decision.
- [x] **II. Multi-Session**: marks are per entry and per source (FR-006) and never cross entries
  (keyed by worktree dir as comments, FR-021 of 482); every window converges on the service's
  state (RM3, RM4); daemon tests cover two attached clients.
- [x] **III. Worktree**: the whole-change read is read-only git in the entry's own directory;
  marks go with the worktree by 482's removal rules (RM5); nothing creates or changes a worktree.
- [x] **IV. Local-First**: marks are in the local review file; ids are computed locally; pull
  request ids come from the diff already read (feature 775) and add no network read.
- [x] **V. Rust + iced**: `ChangeId` is a parsed type (a malformed id cannot be built or decoded);
  `ReviewState` is an enum, so a file has exactly one state; one mark per `(Scope, RelPath)` by
  the map's key.
- [x] **VI. Cross-Platform**: paths are `RelPath` (`/`), the id record is bytes with no platform
  text; git's clean filters apply to `hash-object` as to the diff, so line endings count as the
  view shows them (Edge Cases); CI runs all three platforms.
- [x] **VII. Documentation**: `docs/user-guide/reviewing-changes.md` gains *Marking files reviewed*
  (FR-015); `docs/development/review-redesign.md` notes slice R4 item 1 as done.
- [x] **VIII. Reusable UI**: no new widget: `Glyph` with three new `Icon` variants, `UnreadMark`,
  `ToggleChip`, `Tooltip`, `ProgressBar` (R9).

No violation: Complexity Tracking is empty.

## Requirement map

| FR | Where |
|---|---|
| FR-001 | `marks::file_review` (Reviewed over drafts); client `file_reviews`; `ui/changes.rs::file_row` leading `Glyph`, draft `Tag` unchanged |
| FR-002 | client `mark_control` (`MarkControl`), `Msg::MarkToggled`; `ui/changes.rs` diff header `ToggleChip` + `Tooltip`; D4 via `marks::mark_block` (R7) |
| FR-003 | `ReviewMark.change`; `ChangeId::{of_worktree, of_patch}`; `file_review().changed_since`; `UnreadMark` + tooltip on the row |
| FR-004 | `whole_change` read in `shell/changes.rs::read_list` with every list read; `Msg::ListRead.whole`; states derived at view time, never stored |
| FR-005 | `GitCli::whole_change` ignores toggles (contracts G2); `mark_block` `HiddenKind` |
| FR-006 | `EntryMarks` keyed by `(Scope, RelPath)`; client reads `of_scope(view_scope(view))` |
| FR-007 | no comment operation touches `EntryReview.marks`, `SetReviewed` touches only it (RM5, RM6) |
| FR-008 | `marks::progress`; client `header_progress`; `ui/changes.rs` header slot: `Text` + `ProgressBar` |
| FR-009 | `progress` `None` for no files; `header_progress` `None` while disconnected; fixed-width header slot |
| FR-010 | `EntryReview.marks` stored in `ReviewFile.marks` (S4), pushed by RM3, removed by RM5 |
| FR-011 | RM2 to RM4: idempotent set, serial apply, push to all |
| FR-012 | `store.rs` reads 1 to 3 with no marks (S4) |
| FR-013 | `connected` argument to `mark_control` / `file_reviews` / `header_progress` (R6); `PendingOp::ReviewMark` failure text; older peers refused by the handshake (RM7) |
| FR-014 | all decisions in `micold-core::review::marks` with unit tests |
| FR-015 | `docs/user-guide/reviewing-changes.md`; layout snapshot state `changes-review-marks`; quickstart Part B |
| FR-016 | `EntryMarks::set` is the only remover; RM5 |

## Test strategy

| Requirement | Layer and test |
|---|---|
| FR-003, SC-003 (ids) | core unit `review/marks.rs`: same inputs give the same id; any change of kind, path, rename source, a mode or an object id changes it; working-tree and patch ids of the same bytes differ; `parse` refuses non-hex and wrong length |
| FR-001, FR-003 (states) | core unit `review/marks.rs`: every row of the `file_review` table (data-model.md) |
| FR-008, FR-009 | core unit `review/marks.rs`: `progress` of none, partial, all; `None` for no files |
| FR-002, FR-005 (D4) | core unit `review/marks.rs`: `mark_block` for each origin and toggle pair, the Default entry, no base, `NotRead` (one test group, R7) |
| FR-006, FR-016 | core unit `review/marks.rs`: `EntryMarks` set, replace, remove, repeat answers `false`, scopes kept apart; core unit `review/comment.rs`: `is_empty` with marks only, `clear_sent`/`discard_pending`/`begin_send`/`report` leave marks |
| FR-005, US2 s3 s5 s6, Edge Cases (rename, binary, large, Default entry) | core `tests/review_git.rs` (real temp repos): G2 cases in contracts/marks-wire.md |
| US4 s3 | core unit `review/pr_diff.rs`: `pr_change_ids` changes with the file's patch only |
| FR-010, FR-012, SC-004 | core unit `review/store.rs`: S4 |
| wire | core `tests/protocol_roundtrip.rs`, `tests/schema_hash.rs` (contracts/marks-wire.md § Tests) |
| FR-007, FR-010, FR-011, US3 | daemon `tests/review_edit.rs`, `tests/review_send.rs`: RM1 to RM6 |
| FR-013 (client data path) | client reducer `tests/features_changes.rs`: `ReviewChanged.marks` lands in `State.reviews` per entry and never shows for another entry |
| US1 s1 to s7, FR-002, FR-004, FR-008, FR-013 | client reducer `tests/features_changes.rs`: row states from marks and drafts; `MarkToggled` builds `SetReviewed` with the current id or `None`; control label, pressed, `HiddenKind` (s7), `NotRead`, `Disconnected`; progress after a mark, a toggle, a re-read; a re-read with a changed id gives `changed_since` (US2 s1, s2, s4); the whole-change load kept during a re-read; a failed read keeps the last state |
| US4 s1, s2 | client reducer `tests/features_pr_changes.rs`: PR marks from `pr_change_ids`, separate from the working tree; a mark of an unlisted PR kept and shown when its diff is shown again |
| FR-009, SC-005 (geometry) | layout snapshot: new covered state `changes-review-marks` (three row states, changed dot, chip, partial bar) and `changes-review-marks-unavailable` (no progress; the header's other boxes at the same place), regenerated fixture |
| SC-005 (look) | quickstart Part B visual pass, 1280×800, both themes |

## Milestones (proposed for the tasks unit)

| M | Content | Depends on |
|---|---|---|
| M1 | Core: `review::marks` (ids, `EntryMarks`, states, progress, `mark_block`), `NameStatus` ids, `WholeChange` + `GitCli::whole_change` + `Git` trait and fakes, `pr_change_ids`, `EntryReview.marks`, store v4, `sha1` dependency | none |
| M2 | Wire and service: `SetReviewed`, `ReviewMark`, `ReviewChanged.marks`, protocol bump; daemon `apply_edit`, pushes, removal tests; client receipt only (`Msg::ReviewChanged.marks` into `State.reviews`, `PendingOp::ReviewMark`) | M1 |
| M3 | Client view: whole-change load with the list, PR ids, row state and changed dot, mark control, header progress, reducer tests, icons | M2 **and #1269 merged** |
| M4 | Docs, layout snapshot states, visual pass | M3 |

M1 and M2 touch nothing #1269 changes (`State.open`, `OpenView`, the view's header). If #1269 is
still open when M2 has merged, the run waits on it (or, by the spec's assumption, M3 targets
today's Changes view and is rebased when #1269 lands).

## Project Structure

### Documentation (this feature)

```text
specs/1271-review-reviewed-marks/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/marks-wire.md   # wire, store and git rules RM1–RM7, S4, G2
└── tasks.md                  # /speckit-tasks
```

### Source Code

```text
crates/micold-core/
├── Cargo.toml                 # + sha1 (already in Cargo.lock)
├── src/review/marks.rs        # NEW: ChangeId, WholeFile, WholeChange, ReviewMark, EntryMarks, ReviewState, FileReview, file_review, progress, MarkBlock, mark_block
├── src/review/mod.rs          # + pub mod marks
├── src/review/changes.rs      # NameStatus gains modes and ids
├── src/review/git.rs          # GitCli::whole_change (G2)
├── src/review/pr_diff.rs      # pr_change_ids
├── src/review/comment.rs      # EntryReview.marks, from_parts, set_reviewed, is_empty
├── src/review/store.rs        # version 4, ReviewFile.marks
├── src/git.rs                 # Git::whole_change + fakes
└── src/protocol/{messages,version}.rs  # SetReviewed, ReviewChanged.marks, PROTOCOL_VERSION +1
crates/micold-daemon/src/
├── review.rs                  # apply_edit SetReviewed (RM1, RM2); load/save marks
└── server.rs, state.rs        # pass the new field (pushes)
crates/micold-client/src/
├── features/changes.rs        # ReviewView.marks, Msg fields, OpenView.whole, MarkToggled, file_reviews, mark_control, header_progress
├── shell/daemon_sync.rs       # marks into Msg::ReviewChanged, PendingOp::ReviewMark
├── shell/changes.rs           # read_list runs whole_change; SetReviewed through PendingOp::ReviewMark
├── ui/changes.rs              # row glyph and dot, diff header chip, header progress slot
└── icons.rs                   # ReviewUnseen, ReviewDrafts, ReviewDone
crates/micold-client/tests/support/covered_states.rs, tests/fixtures/layout_snapshot.txt
docs/user-guide/reviewing-changes.md, docs/development/review-redesign.md
```

**Structure Decision**: every decision lives in `micold-core::review::marks` and the render-free
`features/changes.rs`; the daemon stores and pushes; `shell/` runs git and carries fields; `ui/`
draws.

## Complexity Tracking

None.
