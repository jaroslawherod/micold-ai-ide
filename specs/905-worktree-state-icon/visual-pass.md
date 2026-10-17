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

# Visual pass: quickstart §B rows B1-B11 (the client sidebar), 2026-10-17, close unit

- Real client and daemon, built together from this tree, on a private Xvfb (1600x1400), `ICED_BACKEND=tiny-skia` (no Vulkan ICD), light scheme only. A stand-in `claude` (a script that sleeps) on `PATH`, and the daemon's per-session hook URL and token POSTed by hand with `curl` (`UserPromptSubmit`, `PreToolUse`, `Notification`, `Stop`), so the sidebar saw real hook traffic.
- Setup: project P with Default and worktrees `wt-run`, `wt-block`, `wt-idle`, `wt-none` (no session) and `wt-two` (a main and a helper session). Window about 1024x770, so the glyphs are about 10 px tall.

| Row | Result | Seen |
|---|---|---|
| B1 | PASS | `wt-run` triangle, `wt-block` and `wt-two` red ring with "!", `wt-idle` and `wt-none` two bars, Default an empty slot (its session is Unknown). All in one column; names aligned. `sidebar-b1-light.png`. |
| B2 | PASS | `Stop` on `wt-run`: the triangle became two bars on the next frame; the name did not move. |
| B3 | PASS | `UserPromptSubmit`, `PreToolUse`, `Notification` on `wt-block`: red ring with "!". Then the next `PreToolUse`: triangle (`sidebar-b3-granted.png`). A lone `PostToolUse` does not resume the row: the turn clock only resumes on `PreToolUse` or an observed spinner (`activity.rs` `turn_change`, feature 613), and a real `claude` repaints its spinner after a grant, which the stand-in cannot. Tooltip for the Unknown state read `State unknown` (seen on `wt-run` before its first hook). |
| B4 | NOT RUN | needs the provider binary off `PATH`; covered by `sidebar_state_icon` (Failed row) and `location_state` tests. |
| B5 | NOT RUN | a daemon restart mid-turn; covered by `location_state` (`InterruptedResumable`). |
| B6 | PASS | The Default row and a fresh `wt-run` showed an empty slot with the name at the same x as the others, tooltip `State unknown`. |
| B7 | PASS | `wt-two` shows the ring with its sessions listed as sub-rows keeping their own dots. |
| B8 | NOT RUN | no drag of the sidebar divider; covered by the `layout_snapshot` and `tree_view` geometry tests. |
| B9 | PASS | See the showcase pass above; `sidebar-b1-light-gray.png` keeps the three shapes apart without colour. Dark was run on the showcase only. |
| B10 | NOT RUN | a second window. |
| B11 | PASS | Every worktree's state named from the sidebar alone, matching the setup. |

**Blocked octagon.** At 10 px (`sidebar-blocked-zoom.png`, 15x nearest-neighbour) the glyph is a ring with a "!", with flat top and bottom and cut corners. It is clearly told apart from the triangle and the two bars, in colour and in greyscale. Whether the ring is an octagon or a circle cannot be settled at this size, as the showcase pass found; it does not matter for FR-007, which asks only that the three shapes differ.

Not run: dark scheme on the client sidebar, B4, B5, B8, B10, and any real macOS or Windows display (§C is in the quickstart).
