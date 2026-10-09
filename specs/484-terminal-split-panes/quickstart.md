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

## Idle CPU record (step 9, SC-004)

Debug build, lavapipe on Xvfb, shells only, 5 s settle, `utime+stime` per 30 s window (100 ticks = 1 s).
Cause of the earlier +81 %: the OS-theme poll sent a message every 500 ms even when the scheme was
unchanged, so each tick re-composed and redrew the whole window (about 0.14 s of software rasterising
per redraw). Fix: the poll sends only a changed scheme (`changed_scheme`, M5 T040).

| Build | 1 pane | 6 panes |
|---|---|---|
| Before (close unit) | 313/314 ticks (10.4 %) | 569/568 ticks (19.0 %) |
| Before (M5 profile) | 873 ticks (29 %) | 883 ticks (29 %) |
| After | 89/90 ticks (3.0 %) | 92/93 ticks (3.1 %) |

Six panes are +3 % over one pane (limit +10 %); total idle CPU fell about 85 %.

## Results (T036)

- Part A: `mise run gate`-equivalent raw commands green on every milestone PR (#656–#660); the only local failures were six unrelated permission tests that fail when run as root (named in `tdd/verification.md`) and pass in CI.
- Part B: visual pass run on a private Xvfb display against the real client; evidence in `visual-pass-m3.md` (M3: header strip, single-pane refusal) and `visual-pass-close.md` (close unit and M5: refusal text, empty-pane focus, idle CPU; all checks pass after M5).
- Step 9 (idle CPU, SC-004): see the record above, 6 panes +3 % over one pane (limit +10 %).
