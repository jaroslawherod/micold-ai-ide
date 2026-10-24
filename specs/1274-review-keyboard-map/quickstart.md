# Quickstart: Keyboard Map for Reviewing Changes (#1274)

How to show the feature works. Rules: [contracts/keys.md](./contracts/keys.md),
[contracts/jump-to.md](./contracts/jump-to.md); shapes: [data-model.md](./data-model.md).

## Prerequisites

- M1: nothing beyond the workspace (`mise trust` once in a fresh worktree).
- M2: #1269, #1270, #1271 and #1273 merged. M3: #1127 merged.

## §A — Automated

| Milestone | Command | Expect |
|---|---|---|
| M1 | `mise run test-core` | `review::keys`, `review::navigate`, `review::jump`, `review::diff::header_rows` and `quick_switch` tests green; 968's existing `quick_switch` tests unchanged and green |
| M2, M3 | `mise run gate` | `tests/keymap.rs`, `tests/review_keys.rs` (new), `tests/quick_switch.rs`, `tests/overlay_registry.rs`, docs test, layout snapshot and its gates green |

## §B — Recorded pass on Linux (visual-pass, 1280×800, light and dark)

Fixture: an entry with `a.rs`, `b.rs`, `c.rs` (two hunks each), `d.bin` (binary), two files marked
reviewed, and comments on `a.rs:3`, `a.rs:20`, `c.rs:7` plus one outdated comment.

| # | Do | See |
|---|---|---|
| B1 | Review tab shown, `a.rs` selected; `j`, `j`, `k` | `b.rs`, `c.rs`, `b.rs` selected in turn, row in view, diff from the top |
| B2 | `n`, `n`, `n` | first hunk, second hunk of `b.rs` marked current at the top; then `c.rs` first hunk (`d.bin` passed over) |
| B3 | `p` on the first hunk of a file | previous file with hunks, its last hunk current |
| B4 | the current-hunk mark | visible in both themes (FR-005) |
| B5 | `v` twice | mark set, count +1; mark cleared, count −1 |
| B6 | `g` repeatedly | unreviewed files in file order, wrapping; all marked → "All files reviewed" |
| B7 | `]` ×4, `[` | each card in turn, `c.rs` selected for the third; outdated one opens the drawer; wrap; `[` goes back |
| B8 | `c` with picked lines / with a current hunk / with neither | composer on the picked lines / on the hunk's changed lines / note "Pick lines or move to a hunk (n) to comment" |
| B9 | in the composer type `j`, then Ctrl+Shift+Enter | `j` in the text; saved drafts sent, composer open with its text |
| B10 | Ctrl+Shift+Enter with two drafts, outside the composer | sent as **Send review (2)** sends |
| B11 | `?`; `j`; Escape; overflow → **Keyboard shortcuts**; `?` | sheet lists all of `REVIEW_KEYS` in the platform's form; `j` does nothing; closes; opens; closes |
| B12 | hover previous/next hunk, **Mark reviewed**, **Send review** | tooltips name `p`/`n`, `v`, the chord |
| B13 | focus the tree filter and type `jk` | text in the filter, selection unchanged |
| B14 | (M3) jump-to field: empty query; `tests`; a comment word; choose each | review files and comments after locations/sessions, before projects; file selected; card shown |
| B15 | layout snapshot states `review-current-hunk` and `review-shortcut-sheet` | recorded and reviewed (FR-021) |

## §C — macOS and Windows

- macOS: Cmd+Shift+Enter sends; Ctrl+Shift+Enter does not; sheet and tooltips show `Cmd+…`.
- Windows with a German layout: `[`, `]` (AltGr) and `?` (Shift+ß) act; Ctrl+J and Alt+J do nothing.
- Both: a focused terminal receives every one of the review keys unchanged.
