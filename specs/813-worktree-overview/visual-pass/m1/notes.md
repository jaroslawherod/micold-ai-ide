# 813 M1 visual pass (2026-10-17)

Xvfb :77 + `ICED_BACKEND=tiny-skia` (no Vulkan ICD, no xdotool; input via python ctypes XTest), window 1024x768, light + dark. Showcase and client built from the working tree.

| Item | Result | Evidence |
|---|---|---|
| B15 five states: own fill, state icon, state word (light, dark) | PASS | light-cards-1/2, dark-cards-1/2 |
| B15 Idle greyed but legible (both schemes) | PASS | light-cards-2, dark-cards-2 |
| B15 Default and orphan variants | PASS | light-cards-2, dark-cards-2 |
| B15 type icon on every card | PASS | all |
| B15 type WORD on every card | FAIL | The word ("Feature") appears only in a hover tooltip on the type icon (tooltip-type-word.png); no card shows it as visible text. Code does this deliberately (`overview_card.rs`: type_label is "the type icon's tooltip"); the check asked for visible icon + word. |
| B15 no clipped/overlapping text; long names ellipsize | PASS | `needs-attent…`, `gone-worktr…`; the name is cut before the state word |
| B15 CardGrid 1 / 3 / 30 cards | PASS | light-grid-1-3, light-grid-3cards-30scroll, dark-grid-* (30 cards scroll inside the 320px frame, verified by wheel scroll) |
| B15 narrow = one column | PASS | light-grid-narrow, dark-grid-30-narrow |
| B15 greyscale: five states told apart | PASS | light-grey-five-states.png (icon + word differ; fills of Needs action / Needs attention / Working are near-equal greys, so icon and word carry it) |
| B1 (reduced) Overview button | PASS | b1-overview-open/closed: click shows the grid in the main pane (sidebar + top bar stay), second click closes it. Project had only the Default card (1 card, "Idle"); the sidebar listed no worktree, so no multi-card grid in the client. |

Not judged: crossfade, animation.
