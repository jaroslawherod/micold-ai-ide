# Visual pass — 1121 Overview cards, full text (quickstart §B, T048)

The whole of quickstart §B, recorded at close from the milestone passes (T023 M2, T035 M3, T046
M4), M1's §B.1 pass and the close pass. Each pass ran the client or the showcase built from its
milestone's tree on a private Xvfb, driven with XTest, against a fixture project of 8 worktrees plus
Default (M1 and M2: the showcase). Long titles, prompts and todos were fed through the hook endpoint.
Frames are named `<theme>-<scale>-<step>.png` under `visual-pass/<pass>/`.

| §B item | Result | Themes / scales | Evidence |
|---|---|---|---|
| 1 Showcase, collapsed cards: titles 1/2/3/5 lines at 280/380/480 px → 3 lines then an ellipsis; activity 2 lines; nothing clipped (FR-001, FR-007) | PASS (round 2, after the pose was fixed to show the three widths with equal row heights) | light, dark × 1x, 2x | `m1/` |
| 2 Showcase, the expanded card over its neighbours, raised, contrast as 1120 (FR-015) | PASS (round 2). Round 1 flagged a card peeking below the expanded one: it is the next row's card, covered as the spec's assumption intends. | light, dark × 1x, 2x; bottom-row and short-window poses dark-1x only | `m2/exp-*`, `m2/relook-*` |
| 3 Client, hover ~0.4 s → expands down over the card below; no other card moves; leave → collapses (Story 2) | PASS | light, dark × 1x, 2x | `m3/*-b3-*` |
| 4 Bottom-row card shifted up, whole; a very long activity → as tall as the view, header and title stay, the wheel scrolls inside, then the Overview (FR-008) | PASS. M3 inferred the wheel order; M4 showed it: one gesture to the body's end, the next scrolls the Overview. | shift: light, dark × 1x, 2x; tall view: 1x; wheel order: M4, all four | `m3/*-b4-*`, `m4/*-b4-w*`, `m4/*-b4-x*` |
| 5 Keyboard only: Tab/arrows expand each focused card; Escape collapses, the Overview stays; Escape again closes it; Page Down scrolls a long expanded card (Story 3, D1) | PASS. The Overview's opening focus expands no card (FR-013, M4 r2). Page keys with Ctrl/Cmd+Shift still reach the project chords. | light, dark × 1x, 2x; Page keys at 330 px also | `m4/*-b5-*`, `m4/*-b4-pd*`, `m4/*-b4-pu*`, `m4-r2/*-c1..c5-*` |
| 6 Expanded card: a click over the overlapped neighbour activates the expanded card with its ripple; right-click, Menu key and Shift+F10 open its menu over the whole card; no other card expands while it is open (Story 4) | PASS | light, dark × 1x, 2x | `m3/*-b6-*`, `m4/*-b6-*` |
| 6 (inner scrollbar) A press and drag on the expanded card's inner scrollbar keeps it expanded and scrolls only its body | PASS at 1x (M4 r2). At 2x in one column, round 2 did not check it: the grid's bar lay over the inner bar. After the `BAR_FOOTPRINT` inset, the close pass shows PASS at 2x one column and at 1x four columns. | M4 r2: light, dark × 1x. Close: dark-2x, light-2x, dark-1x | `m4-r2/*-c6-*`, `close/*-c1-*`, `close/*-c2-*` |
| 7 Full-screen: same behaviour; zoom unaffected | PASS. The first Escape collapses the card and full screen stays. The second Escape leaves the Overview and full screen. | M3: light, dark × 1x, 2x. M4: dark-1x. Close: light-1x, dark-2x | `m3/*-b7-*`, `m4/dark-1x-b5-fs*`, `close/*-c3-*` |
| 8 The directory line reads `~/…` for a worktree under the home folder (D2) | PASS. The close smoke shows full title, branch, `~/vpproj/.claude/worktrees/<name>` and the full activity with no ellipsis at 2x. | light, dark × 1x, 2x | `m3/`, `close/*-c4-*` |

## Observations, not failures

- **Hover beats keyboard focus.** A pointer resting on another card expands that card and collapses
  the focused one. The focused card expands again when the pointer leaves. This conforms to FR-002,
  FR-005 and FR-006, and the user guide says so (M4).
- **Fast wheel burst.** 12 synthetic notches 50 ms apart scrolled the body to its end and then the
  Overview, all in one burst (M4 r1). Slow notches latch as D4 says. This is open in *Follow-ups*.
- **No ring in click-opened full screen.** Full screen opened by a click shows no focus ring on the
  first card until Tab (close).
- **Ring over the grid's bar.** At 2x the expanded card's focus ring runs over the right edge of the
  Overview's bar. This is cosmetic: the thumbs do not overlap (close).
- **Not a UI clip.** The prompts that end at "compa" are the 200-character clip of the fed text at
  the source, not a UI truncation (close).

## Limits

- **Software rendering.** The passes ran on a software renderer in the cloud container, so frame timing such as the 0.4 s rest was
  judged from frames taken before and after the rest, not from motion.
- **No real wheel.** Real-wheel latching was not tried.
