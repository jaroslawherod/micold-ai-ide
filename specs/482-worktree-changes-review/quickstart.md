# Quickstart: Review a Worktree's Changes and Send Comments to Its Session

**Feature**: 482 | **Plan**: [plan.md](./plan.md) | Contracts:
[review-wire](./contracts/review-wire.md), [review-prompt](./contracts/review-prompt.md),
[changes-view](./contracts/changes-view.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree; `git` on `PATH`.
- For §B: the `visual-pass` skill's private display, pinned binaries and private data directory,
  and a fake AI CLI that echoes its input (the one `crates/micold-daemon/tests/mcp_create_session.rs`
  installs works) set as the default AI CLI, so delivered prompts can be read in the terminal.
- For §C: a macOS bundle from `mise run app` and an installed Windows build.

## §A — Automated

```sh
mise run test-core     # review::{changes,diff,comment,prompt,base,watch,target} + real-git tests
mise run gate          # daemon review tests, client state tests, geometry gates, fmt, clippy, docs
```

Expected: both green. Each test named in [plan.md](./plan.md#test-strategy-by-layer) exists and
failed before the change it covers (cycle log).

## §B — Recorded pass on Linux (visual-pass)

Fixture project P (a git repository with `main`) and worktree `wt` on branch `wt` from `main`:

- commit on `wt`: edit `src/a.rs` (lines 10–12 changed, line 20 removed);
- on disk, uncommitted: edit `src/b.rs`; edit `src/a.rs` line 30; new untracked `notes.md`; a
  changed binary `logo.png`; `big.txt` with 6,000 added lines; `crlf.txt` whose only change is
  LF → CRLF on one line;
- Default entry: an uncommitted edit to `README.md` in the root.

Record a screenshot for every step that names a look, in the light and the dark scheme.

| # | Do | Expect | Covers |
|---|---|---|---|
| B1 | Right-click `wt` → Review changes. | Changes view in the main area; header "Compared with main at <sha>"; list: `big.txt`, `crlf.txt`, `logo.png` (Binary), `notes.md` (Added), `src/a.rs`, `src/b.rs`, each with kind and `+a −r`; `src/a.rs` once. | US1 s1, s3, FR-002, FR-003 |
| B2 | Uncommitted off. Then uncommitted on, committed off. Then both off. | Only `src/a.rs` (its committed change); then all but the committed-only part; then the "both hidden" message. | US1 s2, FR-004, Edge |
| B3 | Select `src/a.rs`; switch Unified ↔ Side by side; select `src/b.rs`; quit and restart; reopen. | Same changed lines in both layouts with both line numbers and Rust colouring; layout kept for the next file and after restart. | US1 s4, FR-006, FR-007 |
| B4 | Select `logo.png`. | Listed; diff area says binary; no gutter, no comment action. | US1 s5, FR-008, SC-005 |
| B5 | Select `big.txt`. Move the mouse and scroll the list while it waits. Press Show diff, scroll to the end. | Counts and Show diff, no lines; the window responds throughout; the diff appears and scrolls smoothly. | US1 s7, FR-009, SC-003 |
| B6 | Select `crlf.txt`. | The one line shows as removed + added with no visible `\r`. | Edge Cross-platform |
| B7 | In `src/a.rs` click line 11's new-side number, write "rename this", Save. Pick removed line 20 and comment "why removed?". In `src/b.rs` pick 3 lines with shift-click, comment. | Three pending cards under their anchors; list rows show pending counts; Send to session (3) enabled. | US2 s1, s2, FR-011 |
| B8 | Edit the `src/b.rs` comment's text; delete and re-add the removed-line one. | Cards show the edited text; the deleted one is gone. | US2 s7 |
| B9 | No session running in `wt`: Send to session. | A session starts in `wt` and its terminal shows one prompt naming `src/a.rs` lines 11 (current) and 20 (removed; base line 20), `src/b.rs` lines with quotes and texts, as in the prompt contract; cards become Sent; snackbar "Started a session and sent 3 comments". | US3 s1, US2 s3, s4, FR-015, FR-016, SC-001 |
| B10 | Add one more comment; Send again (session now running). | The running session receives a prompt with only the new comment. | US2 s6, US3 s4 |
| B11 | Open a second window on P with the same view. Add a comment in window 1, then press Send and immediately look at window 2. | Window 2 shows the comment; its Send is disabled while window 1's send runs; the comment arrives once. | FR-018, US3 s3, Edge Isolation |
| B12 | From a terminal, edit `src/b.rs` lines the comment quotes; then `git commit -am x` in `wt`. | Within 2 s each time the list and diff update; the comment is marked Outdated and keeps its quote; the composer opened beforehand keeps its text. | US4 s1, s2, FR-010, FR-013, SC-004 |
| B13 | Clear sent; then Discard pending… → Cancel, then confirm. | Sent cards gone, pending stay; after confirming, none left. | US4 s3, FR-019 |
| B14 | Add a comment, quit, restart, reopen the view. | Same comments and states. | US4 s4, FR-020 |
| B15 | Default row → Review changes. | `README.md` only; Committed toggle disabled with its note. | US1 s9, FR-003 |
| B16 | Make the default AI CLI unavailable; with no session in `wt`, Send. | Error snackbar; comments stay pending. | US3 s2, FR-017 |
| B17 | Delete worktree `wt` (with comments) while its view is open. | View closes; restarting shows no comments for any entry. | US4 s5, FR-020, Edge Removed worktree |
| B18 | Showcase (`mise run showcase`): DiffView, ReviewCommentCard, TextArea, VirtualRows entries, both schemes. | Every pose readable; added/removed/context distinguishable with syntax colours. | US1 s8, Principle VIII |

## §C — Other platforms

On macOS and Windows: B1, B3, B7, B9, B12 (refresh within 2 s via FSEvents /
ReadDirectoryChangesW) and B6 on Windows with `core.autocrlf=true`. Compare the delivered prompt
text of B9 with Linux's byte for byte (SC-006; the core test P1–P10 already pins the bytes).
