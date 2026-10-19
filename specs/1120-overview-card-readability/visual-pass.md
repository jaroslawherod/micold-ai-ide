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
