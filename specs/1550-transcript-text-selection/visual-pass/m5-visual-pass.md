# Visual pass: M5 copy controls, and the feature end to end (quickstart Part B, steps 6–9)

The real client and daemon (release build of the M5 branch; the pre-M5 client built at 84bca0374
for comparison) on a private Xvfb display (lavapipe Vulkan), driven with xdotool (XTest, buttons
held ~100 ms), clipboard read with `xclip`. The structured sessions ran against a `claude` stand-in
on a private `PATH` replaying fixtures: `bash` (a Bash card), `fail` (a failed turn), `edit` (the
`edit-turn` fixture: Write and Edit cards with diffs), `md` (`markdown-turn`), `cjk` (CJK and emoji
prose) and `stream` (a reply cut after its last text delta, so it stays streaming). Linux only;
macOS and Windows come from the CI-built clients (Principle VI).

| Step | Check | Result | Evidence |
|---|---|---|---|
| 6 | Pointer presses on an unfolded Bash card's input, result and header copy controls | pass: 9 of 9 tries copied (`{"command":"<CWD>/notes.txt"}`, the numbered result, `<CWD>/notes.txt`); "Copied" drawn on the pressed control only | `m5-6-tool-card-header-and-result-copied.png` |
| 6 | Edit card: the diff path, input and result controls; #940's code-block control | pass: each copied by pointer; the code-block control still copies and shows "Copied" | `m5-6-diff-path-copied.png` |
| 6 | Keyboard: Tab to a tool card's control, Enter | not conclusive: Tab moved focus to the top Bash card's header, no ring visible at 40 % screenshots, Enter did not copy in that session; the renderer tests drive Enter/Space through focus traversal and the earlier debug pass saw Enter copy the input | — |
| — | Tool-card darkening: hovering a card's chevron or copy control darkens the whole card, cumulatively, and it stays | **pre-existing, not M5**: the pre-M5 client does the same (unfolded Bash card, mean grey 0.853 → 0.772 after hovering the chevron → 0.727 after the pointer left, persisting), and its sidebar went fully black the same way; fold+unfold resets it. A translucent state layer accumulating on lavapipe, recorded for a separate issue | `m5-tool-card-darkening.png` (M5), `m5-darkening-pre-m5.png` (pre-M5) |
| 7 | Select a span of a streaming reply | pass: the highlight stays on the same span and the region is pixel-identical 4 s later while the turn keeps **Working** | `m5-7-streaming-selection.png` |
| 8 | Light theme: one selection from a diff line through the tool result into prose; a failed turn's footer and error text | pass: highlight visible and text readable on the diff, the code surface, prose and the failed footer; the copy ends with `The number in notes.txt is **42**.` | `m5-8-selection-diff-result-prose-light.png`, `m5-8-failed-turn-selection-light.png` |
| 8 | Dark theme: the same selections | pass: switching theme keeps the selection; highlight readable on every surface | `m5-8-failed-turn-selection-dark.png`, `m5-8-selections-dark.png` |
| 8 | CJK and emoji: drag across `晴れた空は青い 🌤️ 天空是蓝色的，하늘은 파랗다 👍🏽 emoji 🇵🇱 and CJK 漢字かなカナ.`, Ctrl+C | pass: the clipboard holds that exact line (ZWJ/skin-tone/flag sequences whole); on screen no broken glyphs (the flag draws as two regional-indicator boxes, a font coverage limit, selected or not) | `m5-8-cjk-emoji-light.png` |
| 9 | Two panes side by side on two sessions: each keeps its own selection | partial: the right pane's selection stayed while the left pane was pressed, and the menu's Copy copied the right pane's selection; a later selection in the left pane was made but both were not captured together | — |
| 9 | A copy key acts on the focused pane | **fail (M4b behaviour, not M5's code)**: in the split, Ctrl+C after a drag copied nothing in either pane (the busy left pane's streaming selection included) while the same keys copied in the single-pane layout. Likely cause: `ui::composer::transcript_keys` listens only for `state.session.active` and needs `transcript_focused()` (`focused_field == Transcript(active)`), while a press to select sets `focused_field` to the pressed pane's session without focusing that pane. Recorded as a follow-up | — |
| 9 | Two windows on one session | not run: the client opens one window | — |

Other observations, both in M4 code that M5 does not touch (M5 changed only the diff's path row):
on a diff's lines the selection anchors where the drag first moves, not where it was pressed (a
fast drag from `banana` started at `fig`), and a same-line drag that ends right of a short diff
line's end selects nothing. Prose, code results and raw events anchor at the press. Recorded as
follow-ups.
