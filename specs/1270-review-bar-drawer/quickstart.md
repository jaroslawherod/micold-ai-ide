# Quickstart: Review bar and comments drawer (#1270)

**Plan**: [plan.md](./plan.md) | Contracts: [review-bar-ui](./contracts/review-bar-ui.md),
[comments-drawer-ui](./contracts/comments-drawer-ui.md) | Data model:
[data-model.md](./data-model.md). Reuses 946's fixture and fake AI CLI
(`specs/946-review-replies/quickstart.md`).

## Prerequisites

- `mise trust` once; `git` on `PATH`.
- #1269 merged (the Review tab) for §B and §C; §A's M1 and M2 parts run without it.
- For §B: the `visual-pass` skill's private display and data directory; 946's echoing fake AI CLI
  connected to the app's tool server, so reports can be made with `report_review_comment` by hand.

## §A — Automated

```sh
mise run test-core   # review::{bar,drawer,reply}, tokens::anatomy (M1)
mise run gate        # fmt, clippy, workspace tests: material components (M2), features_changes
                     # and app escape tests (M3), layout_snapshot + review_bar_fixed gate (M4), docs
```

Expected: green, every test of the plan's strategy seen failing first. The layout snapshot is
re-recorded in M4 only, deliberately: `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test
layout_snapshot`, and the diff of `crates/micold-client/tests/fixtures/layout_snapshot.txt` is
reviewed for only the intended changes (the Review tab's states).

## §B — Recorded pass on Linux (visual-pass), 1280×800, light and dark

Fixture: 946's project P and worktree `wt`, a session running in `wt`.

1. Open `wt`'s Review tab. The bar above the status bar reads "No review comments yet",
   **Send review (0)** unavailable; the drawer is closed and the diff fills the width (Edge Cases,
   US3 #1).
2. Add three drafts. The bar reads "3 drafts", **Send review (3)** filled (US1 #1).
3. Open the overflow: **Clear sent** unavailable, **Discard drafts…** available. Esc closes the
   menu (US2 #3, #4).
4. Press **Comments**: the drawer opens over the diff's right side; the diff's lines do not move;
   **Send review (3)** has not moved (US3 #2, US1 #6). Drafts tab "Drafts 3", Replies "No replies
   yet". Edit a draft in its row, close the drawer with Esc, reopen: the edit text is still there.
5. Close the drawer. **Send review (3)**: "Sending…", then the success snackbar; the bar reads
   "3 sent" (US1 #3, #4).
6. Report the first comment Applied with a short reply and the second Answered with a reply of
   about ten wrapped lines. Within 2 s the drawer opens itself on **Replies**; the cards show the
   status and reply, the long one cut at 4 lines with **Show all**; the Replies row shows the
   whole reply (FR-020, US4, US3 #5).
7. Report the third comment: the drawer stays as it is (once per send). Close it, report again:
   it stays closed.
8. Press a Replies row: the diff jumps to the card, outlined, left of the drawer's edge (US3 #6).
9. Switch to the session tab and back: the drawer is open on Replies at the same scroll (US3 #8).
10. Stop the session's daemon connection (or make the send fail with the fake CLI's refusal) and
    send a new draft: the bar shows "Couldn't send the review: …" until dismissed with × (US1 #5).
11. Shrink the window to its minimum width: the counts shorten to "1 draft", then disappear;
    **Comments**, ⋮ and **Send review** stay visible; the drawer is at most half the diff area.

Screenshots of steps 1, 3, 4, 6 (both tabs) and 11, light and dark, go in `visual-pass/`.

## §C — macOS and Windows

On each: steps 3, 4, 6 and 9 of §B (overflow menu, Esc, drawer, auto-open). The bar, menu and
drawer are the app's own widgets on every platform (Principle VI).
