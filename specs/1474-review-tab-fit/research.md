# Research: Review tab fits the smallest window

No `NEEDS CLARIFICATION` remained after clarify. Paths relative to `crates/micold-client/src/`.

## R1 — Derive "compact", do not store it

- **Decision**: `compact` is computed from `window.window_size` on each read (`width != 0 && width < 720`).
- **Rationale**: a stored flag would need updating on every resize path and could disagree with the
  size (Principle V). One accessor, `app::State::sidebar_hidden()`, applies it.
- **Alternatives**: store `compact` in `sidebar::State` and update it in `resized` (second source of
  truth; rejected). Mutate `sidebar.hidden` on crossing and remember the old value (breaks SC-003
  "byte-identical" and loses the user's value if the app ends while compact; rejected).

## R2 — One transient override

- **Decision**: `sidebar.compact_open: bool`, set by a toggle at compact width, cleared by `resized`
  when the window is no longer compact.
- **Rationale**: US3 needs "expanded until the window next leaves compact width" and "fresh
  crossing" (a second shrink collapses again); a bool cleared on leaving gives exactly that. A
  toggle at compact width never writes `hidden` (FR-004), so the strip's **show** cannot make the
  collapse permanent.
- **Alternatives**: clear on every resize (breaks US3 s1, resizes within compact would collapse the
  sidebar the user opened); store the width at which it was opened (more state for no behaviour).

## R3 — The harness already carries the window

- **Finding**: `tests/gates/review_tab_fits.rs` `laid_out` sets `state.window.window_size` from the
  window under test, so the two runs differ without touching `tests/support/layout.rs` or
  `covered_states.rs`. The earlier bug-flow note ("the harness never sets window_size") was about
  `records_for`/`view_of`, which feed only the `layout_snapshot` fixture at 1280x800.
- **Decision**: leave `records_for` unreported (not compact). That keeps FR-013 trivially and
  avoids changing the clamp behaviour of the overflow menu, which also reads `window_size`
  (`ui/changes.rs` `overflow_menu`). Add premises to the gate instead (D3).
- **Alternative**: plumb a window into `records_for` and regenerate the fixture (changes every
  snapshot that reads the size; rejected).

## R4 — Re-measure before fixing the header and placeholder

- **Finding** (from the earlier ledger): at 640x480 the diff pane was 66px wide, so each chip took a
  line and the wrap row's height was clamped to 124px. With the strip the diff pane is about
  320px (608 minus the 260px file column, the 2*LG gaps and paddings).
- **Stop rule**: if entries still fire after the strip and the D4/D5 steps, escalate to the spec
  rather than removing controls.
- **Decision**: delete the entry, run the gate, fix only what still fires, in the order D4 (shrink
  height, then order), D5 (centre in a `Fill` container).
- **Alternatives**: give the diff pane a minimum width in the view (tried earlier; a plain iced row
  cannot give a `Fill` child a minimum, and shrinking the tree broke its filter and rows, per the
  earlier ledger's escalation); collapse controls into a menu below ~150px (a second control form
  for a state the sidebar fix removes).

## R5 — Drawer width floor lives in `drawer::width`

- **Finding**: `SideSheet` caps its width at half the area (`review::drawer::capped`, feature 1276
  FR-010). At a 320px diff area that is 160px, which the header and the row buttons do not fit.
- **Decision**: `SideSheet::min_width` (default 0) honoured in `layout` through a new pure
  `drawer::capped_with_floor` (`capped(..).max(min).min(area)`), fed `comments_drawer::MIN_WIDTH`
  (anatomy token) by `comments_drawer`; plus `Row::wrap()` on a row's action buttons. `capped` is
  unchanged for the showcase pose. Passing a wider `wanted` alone achieves nothing, since `capped`
  halves the area.
- **Alternatives**: raise the drawer to the full area when narrow (hides the diff completely, the
  drawer is an overlay of the diff's right side); ellipsise the buttons (a clipped control is not
  reachable, FR-009).
- **Risk**: the floor shrinks the visible diff while the drawer is open at 640. Accepted: the drawer
  is closeable and the diff keeps its layout under it (overlay).

## R6 — The tree needs no change

- **Finding**: `FILE_COLUMN_WIDTH = 260` is fixed; the tree's failures in the earlier ledger arose
  from narrowing that column. The strip leaves it alone.
- **Decision**: pin with a gate assertion; do not edit the tree view.
