# Visual pass — 1122 Overview cards, a link to the worktree's issue (quickstart §B, T052)

The whole of quickstart §B, recorded at close from the milestone passes: T019 (M2, the showcase),
T030 (M3, GitHub links in the client), T038 (M4, the keyboard) and T050 (M5, the Jira base URL
setting and Jira links), plus M1's focus-ring rounds on the shared `TextLink`. Each pass ran the
showcase or the client and daemon built from its milestone's tree on a private Xvfb with lavapipe,
driven with XTest, with a throwaway HOME and data directory and a fake `xdg-open` that logs the
address it is given. The client fixture is a project whose `origin` is `github.com/o/r`, with
worktrees `feat-1122_overview-issue-link`, `feat-mic-87_add-login`, `feat_no-ticket` and one with a
120-character branch; M5 adds a second project with a `feat-mic-87_x` worktree. Frames are under
`visual-pass/<milestone>/`, named `<theme>-<scale>-<criterion>-<what>.png` (round-2 frames carry
`r2`).

| §B item | Result | Themes / scales | Evidence |
|---|---|---|---|
| 1 Showcase: "Issue #1122" on a card, collapsed and expanded, at 280/380/480 px: link in `primary`, whole, the short branch ellipsized first; underline only under the pointer; pointing hand (FR-003, FR-015, SC-004) | PASS (M2 r1). Line heights equal a card without a link; a press on the link draws no ripple, a press on the body does. M1's rounds on `TextLink`: the focus ring first painted over the glyphs (r3), then sat in a gutter clear of the label (r4). | light, dark × 1.0, 2.0 | `m2/*-1-*`, `m2/*-2-*`, `m2/*-3-*`, `m2/*-4-*`; ring: `m1/*-3-r4-*` |
| 2 Client Overview: "Issue #1122" on its card; none on Default and `feat_no-ticket`; none on `feat-mic-87…` until the open project's **Jira base URL** is set, then "Issue MIC-87" without a restart, also in a second window; the second project keeps its own value and link (Story 2, FR-010, FR-012, sc. 8) | PASS. GitHub link and its absences: M3 r1. Jira field, link after save in both windows within ~1 s, second project's own `b.atlassian.net` link while the first keeps `example.atlassian.net`: M5 r1. A read-only window switched from project B to A did not show "Issue #1122" in M5 r1 (its attach is refused, so the repository was not read); fixed and shown in M5 r2. | GitHub: light, dark × 1.0, 2.0. Jira cards: all four; two windows: light 1.0 | `m3/*-1-*`, `m5/*-1-*`, `m5/*-2-*`, `m5/*-3-*`, `m5/*-4-*`, `m5/r2-1-*` |
| 3 Click each link: the browser opens `https://github.com/o/r/issues/1122` and `https://example.atlassian.net/browse/MIC-87`; the Overview stays, no ripple, nothing selected (FR-004, FR-005) | PASS. The fake opener got exactly each address, once per click. | GitHub: all four (M3). Jira: all four (M5) | `m3/*-2-*`, `m5/*-3-after-click-*`, `m5/*-4-*after-click*` |
| 4 Rest on a collapsed card's link: no expansion; rest on the title: expands; the expanded card's link stays clickable (Story 4) | PASS (M3 r1) | light, dark × 1.0, 2.0 | `m3/*-3-*` |
| 5 Press on the card and release on the link, and the reverse: nothing activates; right-click on the link opens the card menu (Story 4 sc. 5) | PASS (M3 r1) | light, dark × 1.0, 2.0 | `m3/*-4-*` |
| 6 Keyboard only: Tab to a card, Tab to its link (ring, card stays expanded), Enter opens it; arrows move card to card from the link; Escape collapses, focus stays; Menu key opens the card menu, Escape returns to the link (Story 3) | PASS (M4 r2). M4 r1 failed Shift+Tab onto the previous card's link and Escape on a focused link (the link lost the keyboard when its card changed form); r2 shows both fixed and the Menu key working after an Escape-collapse. The ring sits clear of the label, also on a link that is the expanded body's first row. M5 r1 shows the same ring on a Jira link. | r1: light, dark × 1.0, 2.0; r2: light 1.0 all, dark 1.0 (1, 6, 7), light 2.0 (1, 6) | `m4/*-[1-9]-*`, `m4/*-r2-*`, `m5/*-8-*` |
| 7 Settings: `http://x`, `https://u@x`, `https://x?q` refused with the reason, nothing stored; clearing the field removes that project's Jira links, the other project's stay (FR-011) | PASS (M5 r1). Also refused: `https://x#f`, `https://`. A saved trailing `/` is stored and shown without it. | refusal shown in all four; the rest light 1.0 | `m5/*-5-*`, `m5/*-6-*` |
| 8 GitHub issues switch off: "Issue #1122" goes; on: back without a restart (FR-001, FR-012) | PASS (M3 r1). The Jira switch does the same for Jira links, with the base URLs kept (M5 r1). | light 1.0 | `m3/light-1.0-6-*`, `m5/light-1.0-7-*` |
| 9 A failing opener: the click shows feature 031's notification naming the address and the reason (FR-013) | PASS with an opener that exits 1 ("Couldn't open … xdg-open exited with status 1"). | light 1.0, dark 2.0 | `m3/*-7-*` |
| 10 Full screen: links behave the same | PASS. Pointer: M3 criteria 1–4 hold. Keyboard: M4 r1, forward Tab, Enter and the Menu key hold. | M3: light 1.0, dark 2.0; M4 r1: all four | `m3/*-8-*`, `m4/*-9-*` |

## Limits

- §B9 ran with an opener that exits 1, not with `PATH` lacking `xdg-open`. Both end in the same
  notification (`shell/link_opener.rs`); only the reason differs, a failed launch instead of an
  exit status.
- §B8 (GitHub switch) and §B9 ran in the combinations listed above, not all four.
- The reverse Shift+Tab fix (M4 r2) was not re-run in full screen. Full screen does not change the
  focus tree, and the r2 fix is checked windowed at 1.0 and 2.0.

## Observations, not failures

- **No hover layer on a card.** No card shows a hover layer, with or without a link. That is how
  `main` drew it before this feature (M2 r1).
- **At 2.0 the grid is one column and scrolls.** An expanded card overlaps the card above it, as
  in feature 1121.
- **Sidebar chips.** Sidebar rows keep their own "#1122" chip when the GitHub issues switch is off.
  That is a separate surface, outside this feature.
- **Narrow Settings.** At 800 px the Jira page's paragraph below the subtitle touches the scrollbar.
  That paragraph is not part of this change (M5 r2).
