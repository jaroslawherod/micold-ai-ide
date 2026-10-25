# Visual pass — M4 layout (#1270)

**Date**: 2026-10-25 · **Route**: the window (`ui::view_with`) of feature 1270's covered states and
variants of them (a 40-line diff of `src/lib.rs` read in, a send error, the smallest window),
rendered in process by iced's CPU rasteriser (tiny-skia) at 1280×800 (and 640×480 for step 11),
light and dark, entrance animations settled and the overlay drawn; a scratch test, not committed.
Steps checked elsewhere, not drawn here: 5, 6's timing and 7 need a live session and are M3's
client-state tests (send, watch, auto-open once); 8's "left of the drawer's edge" is the geometry
gate (`review_bar_fixed`, FR-017: the drawer's left edge is at or right of the diff area's middle);
4's and 9's survival (edit text across Esc and reopen, drawer tab and scroll across switching) are
`features_changes` tests (`the_drawer_and_its_composer_text_survive_switching_and_go_with_the_tab`,
`a_drawer_composer_is_in_its_row_while_shown_and_on_its_card_otherwise`,
`showing_the_drawers_list_scrolls_it_back_to_the_tabs_offset`).

To regenerate: a test binary in `crates/micold-client/tests/` with `mod support;` that builds each
state from `support::covered_states` (`review_shown`, `review_overflow`, `drawer_drafts`,
`drawer_replies`, with a diff read in as `gates/review_bar_fixed.rs`'s `with_diff` does), lays
`support::layout::view_of` out at the window, settles it (`support::layout::settle`), draws the
base and the overlay with the theme of `state.settings.theme_pref`, and writes
`renderer.screenshot(..)` per state and scheme.

| Quickstart §B | Check | Result | Evidence |
|---|---|---|---|
| 1 | Bar above the status bar reads "No review comments yet", **Send review (0)** unfilled and unavailable; drawer closed, the diff fills the width beside the list; the header holds the entry and base only (FR-009) | pass | `visual-pass/m4-1-bar-empty-{light,dark}.png` |
| 2, 3 | Bar "3 drafts · 2 sent · 1 applied · 1 answered", **Send review (3)** filled; ⋮ opens **Clear sent** / **Discard drafts…** hung from the ⋮, rising above the bar, nothing clipped | pass | `visual-pass/m4-3-overflow-{light,dark}.png` |
| 4 | **Comments** shown on; the drawer over the diff's right from under the header to the bar, title **Comments** with ×; tabs "Drafts 3" / "Replies 4"; draft rows with Edit and Delete; the diff's lines and **Send review (3)** unmoved; Edit opens the composer under its row | pass | `visual-pass/m4-4-drawer-drafts-{light,dark}.png` |
| 6 | Replies: Sent row with Mark applied and Answer…, Applied row with its reply, Answered row with its whole reply wrapped; Answer… opens the note under its row | pass | `visual-pass/m4-6-drawer-replies-{light,dark}.png` |
| 10 | "3 drafts", then "Couldn't send the review: …" in the error colour with its ×, before the trailing group | pass | `visual-pass/m4-10-send-error-{light,dark}.png` |
| 11 | 640×480: counts gone, **Comments**, ⋮ and **Send review (3)** whole; the drawer half the diff area | pass | `visual-pass/m4-11-min-window-{light,dark}.png` |

Fixed during the pass: the drawer rows ran under the list's scrollbar (their right border hidden);
the rows now keep `spacing::MD` clear of it.

Seen and not fixed: at 640×480 the drawer is ~125 px wide, so its tab labels wrap ("Drafts / 3")
and **Replies 4** is cut — the spec's half-the-area cap at the smallest window (SC-007 holds at
1280×800). The list pane's **Files** chip is cut at 1280 wide, as M3 recorded; the list keeps its
old share of the width (SC-004), so this is unchanged by M4.
