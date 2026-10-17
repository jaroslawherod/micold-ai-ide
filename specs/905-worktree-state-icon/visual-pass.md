# Visual pass: quickstart §B row B9 (showcase `StateIcon` entry)

- Date: 2026-10-17. Xvfb 1600x1400 with the software renderer (`ICED_BACKEND=tiny-skia`; no Vulkan ICD here, so not lavapipe), not a real display. No window manager. Binary: `micold-showcase`, built from this worktree and copied out of `target-shared/`.
- Window about 1024x770, so the glyphs are only about 10 px tall. Crops are 1:1 and the zoom strip is 10x nearest-neighbour.
- Not run: the greyscale screenshot of B1 (the client sidebar, outside this showcase-only scope), macOS and Windows (§C), mid-animation behaviour.

| Check | Result | Seen |
|---|---|---|
| Running is a triangle (play_arrow) | PASS | Right-pointing triangle, primary-coloured, in both schemes. |
| Blocked is an octagon (report) | INCONCLUSIVE | A red ring with a "!". At about 10 px the silhouette reads as round, so an octagon cannot be told from a circle. Distinct from the other two. |
| Idle is two bars (pause) | PASS (weak) | A grey block split into two bars by a thin vertical gap. Both bars are visible but soft at 10 px. |
| Unknown is an empty slot of the same width | PASS | Nothing drawn. The name starts at the same x as in the other rows, so the slot holds the width. |
| Three shapes distinct in greyscale | PASS | Triangle, ring with "!" and two-bar block are told apart in `state-icon-light-gray.png` and `state-icon-dark-gray.png`. Colour is not needed. |
| Legible against the row, light | PASS | Contrast is adequate. Idle (grey) is the faintest. |
| Legible against the row, dark | PASS | Same. Idle (grey) is the faintest. |
| TreeItem: icon before the name | PASS | The icon sits left of "feat/material-showcase". |
| TreeItem: icon vertically centred on the name line | PASS | Icon centre is within about 1 px of the name's x-height centre. |
| TreeItem: no overlap with name or tags | PASS | Gap between icon and name; the tag chips are on the next line. |
| TreeItem: tag line starts under the name | PASS | The `feat` chip's left edge aligns with the name's left edge, not with the icon. |

Follow-up: the octagon check needs a larger render (a real display or a 2x scale factor). At the sidebar's body_small size on this renderer, report and error_outline look alike.

Evidence (`visual/`): `state-icon-light.png`, `state-icon-light-gray.png`, `state-icon-dark.png`, `state-icon-dark-gray.png`, `state-icon-glyphs-zoom.png` (rows: light, light grey, dark, dark grey; columns: Running, Blocked, Idle).
