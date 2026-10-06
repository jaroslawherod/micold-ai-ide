# Contract: the Changes view (UI)

Glue in `crates/micold-client/src/ui/changes.rs` (new); decisions in the render-free
`crates/micold-client/src/features/changes.rs` (new); reads and watch in
`crates/micold-client/src/shell/changes.rs` and `shell/changes_watch.rs` (new). Components from
`ui/material` (Principle VIII); new shared ones are marked **new**.

## Opening and closing

| ID | Behaviour | Spec |
|---|---|---|
| V1 | A worktree row's right-click menu and the Default row's menu have **Review changes**. It opens the Changes view of that entry in the main area, in place of the terminal pane. | FR-001 |
| V2 | The view's header names the entry, the base (`Compared with <branch> at <short sha>`, or the reason it has none), and has a close action. Selecting a session in the sidebar also closes it. | FR-003 |
| V3 | If the entry's worktree is removed while open, the view closes (catalog no longer lists it). | Edge "Removed worktree" |

## File list (left pane of the view)

| ID | Behaviour | Spec |
|---|---|---|
| L1 | Two `LabelledToggle`s, **Committed** and **Uncommitted**, both on at open. For the Default entry the Committed toggle is disabled with the note "The project root has no branch of its own to compare; only uncommitted changes are listed". | FR-004, US1 s2, s9 |
| L2 | One row per changed file in `VirtualRows` (**new**): path, a `Tag` for the kind (Added, Modified, Deleted, Renamed `old → new`, Mode, Binary, Not text), `+a −r` counts, and a pending-comment count when any. | FR-002, FR-008 |
| L3 | Empty states: "No changes against <base>" (US1 s6); "Committed and uncommitted changes are both hidden" (Edge "Both toggles off"); a base reason line when committed changes are unavailable. | US1 s6, Edge |
| L4 | 2,000 files list without a frame over 100 ms (only visible rows are built). | SC-003 |

## Diff (right pane)

| ID | Behaviour | Spec |
|---|---|---|
| D1 | `DiffView` (**new**, `ui/material/diff_view.rs`) shows the selected text file in the layout of `Settings::diff_layout`, with a pair of `ToggleChip`s, **Unified** and **Side by side**. Changing it sends `SettingsSet { diff_layout }`. | FR-006, US1 s4 |
| D2 | Line numbers for both sides in both layouts; added/removed/context rows tinted with roles from the active scheme; syntax spans from R10 painted over them. | FR-006, FR-007, US1 s8 |
| D3 | Binary / not-UTF-8 / mode-only: a message instead of lines ("Binary file — not shown", "Not UTF-8 text — not shown", "Only the file mode changed"), no gutter, no comment action. | FR-008, US1 s5 |
| D4 | A large file shows its counts and a **Show diff** button; pressing it loads and shows the diff. Loading shows `StageProgress::new("Loading diff…", r)` in place of the rows; input keeps working. | FR-009, US1 s7 |
| D5 | Rows are virtualized (`VirtualRows`); a 50,000-line diff never blocks a frame over 100 ms. | SC-003 |

## Commenting

| ID | Behaviour | Spec |
|---|---|---|
| C1 | Clicking a line's gutter number picks that line; shift-click on the same side extends to a contiguous range; a pick never spans sides (a click on the other side starts a new pick). Context lines count as the new side. | FR-011 |
| C2 | With a pick, an **Add comment** action opens the composer under the last picked row: `TextArea` (**new**, `ui/material/text_area.rs`), **Save** (also Ctrl/Cmd+Enter) and **Cancel**. Save sends `ReviewEdit::Add` with the quote taken from the loaded lines. | US2 s1, s2 |
| C3 | Each comment shows under its anchor row as a `ReviewCommentCard` (**new**, `ui/material/review_comment.rs`): text, state (Pending / Sent), Outdated tag when R14 says so, and Edit/Delete for pending comments not in a send. A comment whose anchor is not in the current diff (file gone, toggles hide it) is listed in the file list row count and in a "Not in the current diff" group at the top of that file. | FR-011, FR-013, US4 s2 |
| C4 | The composer and its text survive a refresh of the list or the diff. | Edge "File changes while composing" |

## Sending and tidying (view toolbar)

| ID | Behaviour | Spec |
|---|---|---|
| S1 | **Send to session (n)**: enabled iff the entry has `n ≥ 1` pending comments and `sending` is false. While sending its label reads "Sending…" and it is disabled in every window. | FR-014, FR-018, US2 s5 |
| S2 | Success: a `Snackbar` "Sent n comments to <session label>" (or "Started a session and sent n comments"). Failure: an error `Snackbar` with the service's message; comments stay pending. | FR-017, US3 s2 |
| S3 | **Clear sent** removes sent comments (enabled iff any). **Discard pending…** opens a confirmation `Modal` built with `dialog::body` as `ui/confirm_delete.rs` does ("Discard n pending comments? This cannot be undone.") and sends `DiscardPending` on confirm. | FR-019, US4 s3 |

## Refresh

| ID | Behaviour | Spec |
|---|---|---|
| R1 | While the view is open the watch subscription runs for its entry; a relevant change re-reads the list and the open diff (coalesced, R9). Visible within 2 s. | FR-010, SC-004 |
| R2 | A refresh keeps the selected file if still listed, the scroll position, the pick (dropped if its lines are gone), and the composer. | US4 s1, Edge |

## Showcase

`crates/micold-client/src/showcase/`: poses for `DiffView` (unified, side by side, binary message,
large-file gate, picked range, both schemes), `ReviewCommentCard` (pending, sent, outdated, in a
send), `TextArea` (empty, focused, multi-line) and `VirtualRows` (long list), with catalogue entries.
