# M2 visual pass (quickstart Part B, items 1-6)

2026-10-26. Sway headless + pixman + lavapipe (not a real display), client and daemon built from this
branch (M2 head d7e4a2ae4) and pinned. Window 640x480 (sidebar 300 px, default) and 1280x800, dark and
light. Fixture: a git project with a worktree whose long name and title cut, `claude` replaced by a
`sleep` shim so the Claude terminal session runs; killing the shim gives the stopped state.
Height: "about 100 px" (TWO_ROWS_MAX 100) per the user's decision, measured ~100 px from the chips row to the terminal.

| # | Result | Evidence |
|---|---|---|
| 1 | PASS. Header + strip only above the terminal; "Open in structured view" sits in the strip before the restart slot (status words, switch, [restart], +, splits). At 640 the label reads "Open in struct..." (strip gives way). | m2-{dark,light}-{640x480,1280x800}-single.png |
| 2 | PARTIAL. A held queue cannot be produced without a real structured composer queue, so the long caption was not shown. Squeeze order seen without a caption: switch label cut first ("Open i..." when stopped at 640), status word and restart whole. | m2-*-640x480-stopped.png |
| 3 | PASS. Review open: switch hidden, strip holds Review tab, status word, +, splits. Stopped session: restart at the same x with Review open (1136 px at 1280) as with it closed; closing returns the switch. | m2-*-review-open.png, m2-*-stopped*.png |
| 4 | PASS for title and branch/long name (title cut with ellipsis first, Idle and Changes chips whole at 1280). At 640 the title shrinks to a stray `..` fragment and Changes is cut to "Changes . 1 file . +1...". Pull request chip not tried (no gh in the fixture). | m2-*-long-title.png |
| 5 | PASS. Each tiled pane keeps its own "Open in structured view" row; the strip carries no switch. At 640 the tiled button wraps to two lines and the second pane's header is clipped at the right edge. | m2-*-tiled.png |
| 6 | PASS. Stopped session: "failed", switch, restart last of the cluster, then +, splits; whole at 640 and 1280. | m2-*-stopped.png |

Not covered: held-queue caption (item 2), PR chip (item 4), mid-state animation.
