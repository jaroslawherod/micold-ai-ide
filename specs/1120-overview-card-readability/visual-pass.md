# Visual pass — feature 1120

## M1 (quickstart §B6, M1 part), 2026-10-19, light scheme, showcase Overview card entry (11 poses)

| Check | Result |
|---|---|
| Title is a sentence | PASS: "Worktree overview: one card per location", ellipsized at card width |
| Branch line `feat/813` | PASS (Default shows `main`, the orphan shows none) |
| Sessions line `3 sessions · 2 running` | PASS, after the type word on the same line (`Feature · 3 sessions · 2 running`), as T010 specifies until the type word moves to the header in M4 (T031–T039) |
| No *No task progress* | PASS |
| No `0 open · 0 merged` | PASS (unknown counts show "—") |

Activity markup (M2), the header band (M4) and the other §B rows are checked in later milestones.
Screenshots: `visual-pass/b6-*.png`.

## M2 (quickstart §B6, M2 part), 2026-10-19, light scheme, showcase Overview card entry (12 poses)

| Check | Result |
|---|---|
| Activity line without markup | PASS: the "with activity" card reads `Continue M2 · 2/5`; its input is an orchestrator `<agent-message>` wrapping "Continue M2" (showcase `surfaces.rs`) |
| No `<`, tag or attribute on any card | PASS (12 cards) |

Screenshots: `visual-pass/b6-m2-*.png`.

## M3 (quickstart §B6, M3 part), 2026-10-20, light scheme, showcase (Xvfb, lavapipe)

| Check | Result |
|---|---|
| Card title 17 px, body 14 px | PASS: the same text is 101 px wide as CardTitle and 95 px as Section (ratio 1.063 ≈ 17/16); body lines at 14 px |
| Cards grow to fit, nothing clipped | PASS: the "with activity" card is the tallest; no line cut off |
| Minimum height | PASS: CardGrid cards are 172 px |
| No overlap at 280 / 380 / 480 px | PASS: a long title ellipsizes before the state icon at 280 and 380, fits at 480 |
| Every card in a row as tall as the tallest | Partly: the showcase grid's cards have equal content, so no card is stretched; covered by the unit test `every_card_in_a_row_is_as_tall_as_the_tallest`. M4's showcase work poses one taller grid card |

Snapshot `35292da2…:d544ff00…` (before the review A fixes, which change line heights by a few px and
row heights only mid-zoom). Screenshots: `visual-pass/b-m3-*.png`.


## M4 (quickstart §B, M4 part), 2026-10-20, light and dark, showcase and client Overview (Xvfb, lavapipe)

| Check | Result |
|---|---|
| Header row: tinted, full width, state icon and word at the start, type icon and word at the end | PASS in all five states of the "five states" grid and in the single-card poses |
| Idle header de-emphasised; body fill the same in every state | PASS: Idle header light rgb(231,224,236) on body rgb(248,242,247), dark rgb(73,69,78) on rgb(28,27,30); body fill identical across states |
| Legible in both schemes | PASS: pastel headers with dark text in light, deep headers with light text in dark |
| A taller card stretches its row | PASS: the needs-action card (activity and progress) sets its row's height |
| Client Overview has the same layout | PASS: seeded repository, three cards (Idle only: no sessions running) |
| User guide colour words match | PASS: `docs/user-guide/overview.md` lines 187–188 |

Not covered in this pass: tooltips (B4), outline, focus and ripple (B5), the resize sweep (B7).
Snapshot `1684990c…:89b4a4ac…` (before the review A fixes, which change how a targeted card's
outline is drawn and let the header words ellipsize in a card under 280 px). Screenshots:
`visual-pass/b-m4-*.png`.
