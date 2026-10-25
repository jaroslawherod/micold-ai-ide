# Visual pass evidence — #1271 (SC-005)

The visual pass ran once per milestone, by the orchestrator, against quickstart Part B in both
themes (cloud Xvfb session). The close unit ran no new GUI pass: this folder collects the final
rounds' crops. Round-by-round record: `../autopilot.md` (Review rounds).

| Milestone | PR | Rounds | Verdict | What the rounds fixed |
|---|---|---|---|---|
| M1, M2 | #1374, #1390 | — | no view change (tests only) | — |
| M3 row states, mark control, progress | #1411 | 3 | R1 FAIL (3), R2 FAIL (3), R3 PASS (Part B 1, 3–4, 8, light and dark) | R1: path cut with an ellipsis, wrapping chip and toggle rows, progress slot bounded. R2: path cut from its directory side, fixed-width bar, **Mark reviewed** a one-line chip. |
| M4 changed-since dot, other windows' marks | #1417 | 4 | R1 FAIL (2), R2 FAIL (1), R3 FAIL (900 px only), R4 PASS | dot drawn over the row's padding in a `stack!` (tooltip, path width); tags one line; row content clipped at the content edge (gate `a_changed_rows_text_is_clipped_clear_of_the_dot`). |
| M5 pull request marks and progress | #1419 | 1 | PASS on all 5 checks (Part B 7 and the pull request states, light and dark) | none needed; follow-ups recorded in `../close.md`. |

Two-window check (quickstart Part B 2, 9; M4 tooltip text for another window's mark): **not
exercisable in cloud**. A second client is read-only ("project busy"), so a second window cannot
send a mark. The behaviour is covered by the daemon tests (`review_edit`, `review_send`) and the
client's `daemon_sync` and `features_changes` tests.

Not covered by a pass: the 640x480 layout (no layout below about 900 px session width, a
pre-existing gap, see follow-ups).

## Crops

| File | Shows |
|---|---|
| `m3-r3-full-1280-1of4-{light,dark}.png` | 1280x800, one of four reviewed (Part B 3) |
| `m3-r3-full-1280-4of4-{light,dark}.png` | full bar, four of four |
| `m3-r3-b1-rows-1600-panel-fits-{light,dark}.png` | rows with ○ / ◉ / ✓ and a draft count, panel open |
| `m3-r3-b34-header-{1of4-dark,4of4-light}.png` | the header's progress slot |
| `m3-r3-b8-640x540-chip-unobstructed-light.png` | chip at a narrow window |
| `m3-b4-unavailable-tooltip-{light,dark}.png` | **Mark reviewed** unavailable while a toggle hides part of the file (D4); round 1 crops, behaviour unchanged since |
| `m4-r4-c3-1280-panel-rows-{light,dark}.png` | changed-since dot at the end of a row |
| `m4-r4-c2-…-t3-zoom-{light,dark}.png` | the dot's tooltip, no longer fading black |
| `m4-r4-c1-900-panel-rows-{light,dark}.png` | 900 wide, panel open: tags clipped clear of the dot |
| `m5-b1-pr-{light,dark}.png` | a pull request's rows and progress |
| `m5-b2-marked-{light,dark}.png` | a file marked in a pull request |
| `m5-b3-worktree-{light,dark}.png` | the working tree unaffected by the pull request's marks |
| `m5-b4-tooltip-{light,dark}.png` | changed-since dot tooltip in a pull request |
| `m5-b5-panel-open-file-{light,dark}.png` | Comments panel open on a file (covers **Mark reviewed**, follow-up 4) |
