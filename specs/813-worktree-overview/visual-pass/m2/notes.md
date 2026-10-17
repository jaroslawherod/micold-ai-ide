# 813 M2 visual pass (2026-10-17)

Component showcase (`micold-showcase`, built from the working tree, copied out of target-shared), Xvfb :77 + `ICED_BACKEND=tiny-skia`, 1024x768, light + dark; input via python ctypes XTest. Cards are `OverviewCard` at HEIGHT 136, width 320.

| Item | Result | Evidence |
|---|---|---|
| B4 counts footer reads "1 open · 2 merged" on every card variant, readable, no overlap/clipping | PASS | light-cards-1/2, dark-cards-1/2 |
| B4 unknown counts: footer shows "—" | PASS | light-cards-2, dark-cards-2 (`unknown-counts`) |
| B5 "—" tooltip gives the reason | PASS | tooltip-unknown-counts.png ("GitHub is not reachable", Unreachable variant; the "pull request status is off" reason was not posed in the showcase) |
| Merge-conflict card is Needs attention: state colour, eye icon, word intact | PASS | `gone-worktree` (orphan, MergeConflict) in light-cards-2, dark-cards-2; it has no counts footer (Hidden) |
| State colour/icon/word intact on other cards at HEIGHT 136 | PASS | all five states, both schemes |

Notes: the showcase has no card with merge conflict AND counts, and no PR-off variant; the PR-off tooltip text and a conflict card with counts were not seen. Not judged: animation.
