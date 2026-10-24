# Contract: comments drawer and cards (UI)

**Spec**: FR-010–FR-021, SC-002–SC-004, SC-006 | **Data model**: `review::drawer`,
`review::reply::card_preview`, client `ReviewDrawer`

| ID | Behaviour | Test layer |
|---|---|---|
| D1 | Closed when the Review tab is created; the diff then fills the width beside the file list (FR-011, US3 #1, SC-004). | core unit (`Drawer::default`); geometry gate |
| D2 | When open, a `SideSheet` stacked over the right of the diff area, from under the review header to the top of the bar, `drawer::width(area)` wide (360, at most half). The diff pane's bounds and first visible line are the same open or closed (FR-010, SC-002). | core unit (`width`); geometry gate `review_bar_fixed.rs` |
| D3 | Closes on its ×, on **Comments**, and on Esc when no registered surface is open; Esc with a menu, dialog or select list open closes only that (FR-012, SC-003). | core unit (events); client `on_escape` tests |
| D4 | Two tabs labelled with counts ("Drafts 3", "Replies 4"); Drafts = Draft, Replies = Sent, Applied, Answered; every comment of the entry in exactly one (FR-013, SC-006). | core unit + property over all statuses and both scopes |
| D5 | Rows grouped working tree first, then pull requests by number (a pull request not in the list flagged "(not in the list)"), ordered by file, line, side, created; a row whose file is not in the current diff is marked (FR-014). | core unit (`rows_of`); client state (`drawer_rows`, `drawer_groups`) |
| D6 | Drafts row: file, lines, text, **Edit** and **Delete**, withheld while a send holds the draft. Edit opens the composer in the row (`ComposerPlace::Drawer`) (FR-015). | client state |
| D7 | Replies row: file, lines, start of the comment, status, whole reply wrapped; over 4,000 characters its start and **Show all** (today's dialog); a Sent row offers **Mark applied** and **Answer…** with today's note rules, the note drawn in the row (FR-016). | client state; component test (`ReviewPanelRow` reply mode) |
| D8 | Pressing a row: today's jump (select file, scroll, outline card) and the target line stays left of the drawer's edge; a refused jump shows today's reason in the drawer (FR-017). | client state (jump targets); quickstart §B |
| D9 | Empty tab texts: "No drafts" (+ today's hint when the entry has no comments), "No replies yet" (FR-018). | core unit |
| D10 | Open state, tab, per-tab scroll and composer text belong to the entry's view and survive switching away and back; two windows each keep their own (FR-019, Edge Cases). | core unit (state is a value); client state with #1269's view map |
| D11 | Auto-open on Replies at the first report of a send started in this window, once per send, only while the entry's Review tab is shown and the drawer was not closed since; an open drawer keeps its tab (FR-020). | core unit (`Drawer`, `newly_reported`); client state over `ReviewChanged` |
| D12 | A comment card shows its status and, once replied to, the reply under its text, at most 4 lines at the card's width, then **Show all** only when cut; drawer state does not matter (FR-021, US4). | core unit (`card_preview`); component test (`LineClamped::when_cut`, card) |
| D13 | Counts, tab counts, rows and cards follow any `ReviewChanged` within 2 s; a focused row that disappears loses focus; an open Show all dialog of a removed comment closes (FR-022). | client state |
