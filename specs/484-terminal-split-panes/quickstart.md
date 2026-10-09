# Quickstart: validate terminal split panes

Prereqs: `mise trust`, a project with two sessions. Gate: `mise run gate`.

## Part A: automated
1. `mise run test-core` — layout tree, store degradation, wire types.
2. `mise run test` — daemon multi-attach, client routing/resize tests.
3. `mise run gate` before pushing.

## Part B: visual pass (`visual-pass` skill, private display)
1. Split vertically (`Ctrl+Shift+D`): two panes, new one empty with a picker (US1).
2. Show session A's AI CLI left and a regular terminal of A right; both update live.
3. Type: only the focused pane receives; the focus mark is visible in light and dark and not colour-only (US2).
3b. Select text and follow a link in each pane; scrollback in one pane does not move the other (FR-003).
4. Click the unfocused pane: focuses, no input delivered.
5. Run `stty size` in each pane, drag the divider, re-run: each matches its pane (US5).
6. Double-press the divider: equal. Drag a header onto the other pane: swap. Close a pane: session keeps running (US4).
7. Reach 6 panes; a 7th split shows the refusal reason.
8. Restart the app: layout, ratios, focus restored; per project (US6). Corrupt the `pane_layout` field by hand: starts with one pane.
9. Idle: 6 panes, no output, compare CPU with one pane (≤ +10%, SC-004).
