# Contract: whole-window covered states (M3)

Registered in `crates/micold-client/tests/support/covered_states.rs`, one entry each and nothing
else (`tests/layout_coverage_registry.rs`). Composition: [research R5](../research.md#r5--whole-window-covered-states-names-and-composition).

## States

| Name | Board | Holds |
|---|---|---|
| `window-session-view` | *Proposal: session view* | title bar; sidebar with Overview row, chip bar, Default and five worktrees across more than one group; a session shown in `feat-working` with the main session tab, 2 shell tabs and 1 helper tab; status bar steady (connected, sandbox running, CLI item) |
| `window-overview` | *Proposal: Overview* | the same, with the Overview open in the session area's place (not full screen) and the sidebar's Overview row selected |

Both at 1280×800, both schemes, through the existing fixture, text-overflow and containment gates
(US3 scenario 4).

## Anchors

`shell.root` (`&[]`), `title_bar`, `shell.body`, `sidebar`, `session.area`, `status_bar`. Paths are
copied from the regenerated fixture (`tests/fixtures/layout_snapshot.txt`), not counted from the
source. `shell.body` and `status_bar` today sit at `&[0, 0, 1]` and `&[0, 0, 2]`
(`STATUS_BAR_ANCHORS`); #1127 may move them.

## Gate W — `tests/gates/window_regions_fit.rs`

Included from `tests/layout_snapshot.rs` with `#[path = "gates/window_regions_fit.rs"]`, reading the
cached records of both states in both schemes.

| ID | Assertion | Requirement |
|---|---|---|
| W1 | `title_bar.y == 0` and `title_bar.h == anatomy::app_bar::HEIGHT` (44.0 after #1127); `title_bar` anchors the bar, not the container that also holds its hairline | SC-004, US3 s2 |
| W2 | `status_bar.h == anatomy::status_bar::HEIGHT` (28.0) and `status_bar.y + status_bar.h == 800` | SC-004 |
| W3 | `title_bar.y + anatomy::app_bar::BOTTOM_EDGE == shell.body.y` (the bar plus its 1 px hairline, `HEIGHT + DIVIDER`; 0 px gap below the hairline) and `shell.body.y + shell.body.h == status_bar.y` | SC-004, FR-001 |
| W4 | `sidebar.h == shell.body.h`, `session.area.h == shell.body.h`, `sidebar.x + sidebar.w == session.area.x` | US3 s2–s3 |
| W5 | in `window-overview`, asserted on the `State` the state builds (the records hold geometry, not style): the Overview is open and the sidebar's selected row is the Overview row, read through the field or accessor #1125 M3 ships (open until stage 2) | US3 s3 |

The hairline is the title bar's bottom edge (research R6): SC-004's 0 px gap is measured from
`BOTTOM_EDGE`, its 44 px from `HEIGHT`. If #1127 drops the divider, W3 follows what ships
(`BOTTOM_EDGE == HEIGHT`, or `title_bar.y + title_bar.h` if the constant goes).

Each anchor name above is held by W1–W4, which is its "property only its own element satisfies"
(`docs/development/layout-snapshot.md#anchors`).

A failure message names the state, the anchor and both numbers.

## Mutation proof (quickstart §A3)

| Edit (reverted after) | Expected failure |
|---|---|
| +1 px padding on the title bar | fixture names `window-session-view` and `window-overview`; W1, W3 |
| +1 px on the status bar's height | fixture; W2, W3 |
| 1 px narrower sidebar | fixture; W4 still holds (edges meet), fixture names the state (SC-005) |
| a 1 px spacer between title bar and body | fixture; W3 |
| `window-overview` built with the Overview closed | fixture; W5 |
