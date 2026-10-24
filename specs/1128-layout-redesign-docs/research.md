# Research: Layout redesign — user guide, layout snapshot and visual pass

Feature #1128. Each decision gives what was chosen, why, and what was rejected. Paths are
repository-relative; line numbers are as of the plan (2026-10-24, main after #1124 and #1126).

## R1 — Staging and milestones

- **Decision**: four milestones. M1 = stage 1 (status bar and session area in the user guide),
  done now. M2 = stage 2 user-guide sweep (sidebar, title bar, FR-009 opening section). M3 = the two
  whole-window covered states, their geometry gate, the regenerated fixture and
  `docs/development/layout-snapshot.md`. M4 = the visual pass. M2–M4 start only after #1125 and
  #1127 have merged on main (spec Assumptions); M3 before M4, because the visual pass draws M3's
  states.
- **Rationale**: the spec's staging; M1 is docs only (every path `micold-docs` in
  `.gitattributes`), so it ships without the test suite and without waiting. M3 is the only
  milestone with test code.
- **Rejected**: one PR at the end (stage 1's pages stay wrong for as long as #1125/#1127 take);
  doing M3 now against today's app bar (its anchors, paths and the 44 px height all change with
  #1127, so the fixture and gate would be rewritten).

## R2 — Owning sections

- **Decision**:

  | Element | Owning section | Source |
  |---|---|---|
  | Status bar (connection, sandbox, CLI, plan usage item, key hints) | `appearance-theming.md#status-bar` | written by #1124 |
  | Context header | `terminal-panes.md#the-header-above-the-terminal` | written by #1126 |
  | Tab strip (session and shell tabs, "+", status and restart, New session, splits) | `terminal-panes.md#the-tab-strip-above-the-terminal` | written by #1126 |
  | Where plan-usage values come from (not the item) | `worktrees-and-sessions.md#claude-plan-usage` | existing; it links to `#status-bar` for the item |
  | Sidebar's Overview row, chip bar, groups, two-line rows | where #1125 writes them (today *Browsing worktrees* and its subsections in `worktrees-and-sessions.md`) | stage 2 reads what shipped |
  | Title bar (breadcrumb, jump-to field, Settings, overflow menu) | where #1127 writes it; if it writes none, a new `### Title bar` in `appearance-theming.md#appearance--layout`, beside `### Status bar` | stage 2 |

- **Rationale**: FR-004/FR-010 ask for one owning section each; the region features already wrote
  them, so moving them would only churn links. `worktrees-and-sessions.md#the-tab-strip` (line
  1796) describes the strip a second time; it keeps its instance behaviour (exactly one marked tab,
  the AI tab's three rules, right-click), which is about Regular Terminal instances, and its opening
  paragraph that lists the strip's members becomes one sentence linking to the owning section.
- **Rejected**: moving the status bar's section into `worktrees-and-sessions.md` (breaks #1124's
  links from `settings.md` and `sandboxed-daemon.md` for no reader gain); deleting the
  `#the-tab-strip` subsection (its anchor is linked from the same page's lines 1748–1788 and its
  instance rules are owned nowhere else).

## R3 — How the stale mentions are found

- **Decision**: each docs milestone re-runs one grep over `docs/user-guide/` and `site/media.toml`
  with fixed patterns (contract [doc-edits.md](contracts/doc-edits.md) §Patterns), then reads each
  hit; the plan-time inventory in that contract is the expected result, not the limit. The done
  check is SC-001: the same grep returns only hits that describe the element as absent or that are
  an owning section.
- **Rationale**: SC-001 is a search; running the same search to find and to verify keeps them one
  check. The removed-element list is final only once #1125 and #1127 ship (spec Terms), so stage 2
  adds the patterns for what they removed.
- **Rejected**: reading the 2,342-line `worktrees-and-sessions.md` whole (costly, and misses the
  other pages FR-011 covers); a script committed to CI (out of scope, FR-020; a phrase list in CI
  would reject correct historical sentences such as "it is no longer in the app bar").

## R4 — `site/media.toml` alt text and captions are in the sweep

- **Decision**: alt text and captions of the media the four pages embed (`<!-- media: id -->`:
  `worktree-sidebar-*`, `create-worktree-light`, `switch-session-light`, `session-terminal-*`,
  `open-project-light`) are swept with the pages, under the same staging. Known stage-2 hit:
  `worktree-sidebar-light` alt *"each tagged with the kind of branch it holds -- docs, feat and
  fix"* (type chips on rows, removed by #1125 M2).
- **Rationale**: `site/stage.sh` expands each directive into the picture with that alt text and
  caption, on the page the reader reads, and the picture is re-captured from the real app at each
  release (`docs/development/docs-site.md`), so text written for the old layout would describe a
  picture that no longer shows it.
- **Rejected**: leaving them (a reader on the site sees a caption naming a removed element, against
  FR-001/FR-005); adding new media (spec Assumptions: no screenshots added to the guide).

## R5 — Whole-window covered states: names and composition

- **Decision**: `window-session-view` and `window-overview`, registered in
  `crates/micold-client/tests/support/covered_states.rs` after the status-bar states.
  - `window-session-view`: `with_every_card_state()` (Default plus one worktree per card state, so
    more than one sidebar group) with a session shown in `feat-working`, its strip holding the main
    session, two shell instances and one helper (the shape `strip_with(2, 1)` builds), sandbox
    running (`running_sandbox()`), connected. 1280×800 (the canonical size).
  - `window-overview`: the same base with `overview.open = true`, not full screen, so the sidebar
    stays and its Overview row is the selected row (as #1125 M3 ships the selection).
- **Rationale**: one state per mockup board (FR-012); reusing the existing builders keeps it "one
  entry in `covered_states.rs`" (`layout-snapshot.md#adding-a-covered-state`) and avoids a
  duplicate fixture builder (CLAUDE.md, search before writing). `strip_with` today builds its
  session on `feat-short` over its own base; it is extended to take the base `State` and the
  location (its current callers pass what they pass now), not copied. At `feat-working` the
  extended builder **replaces** the session `with_every_card_state()` put there with its own main
  session, keeping `ActivitySignal::Working`, so `feat-working` stays in its sidebar group. The `window-` prefix sets them apart
  from region states (`status-bar-*`, `session-area-*`, `main-shell-*`) in a failure message.
- **Rejected**: `proposal-session-view` (names the mockup, not the window); reusing
  `status-bar-steady` (one worktree, one group, one tab: it does not compose SC-004's regions, D3);
  a full-screen Overview (the board keeps the sidebar).
- **Open until stage 2**: the exact builder fields for the Overview row's selection and for the
  title bar come from #1125 M3 and #1127 as merged.

## R6 — Anchors and the region gate

- **Title bar and its divider**: `title_bar` anchors the bar itself, not the container that also
  holds the 1 px hairline beneath it (`anatomy::app_bar::DIVIDER`; in `status-bar-steady` today the
  container `0/0/0/0` is 65 px, the bar `0/0/0/0/0` 64 px, the hairline `0/0/0/0/1` 1 px at y 64).
  The hairline is the title bar's own bottom edge (§7.1, `app_bar::BOTTOM_EDGE = HEIGHT +
  DIVIDER`), so SC-004's "title bar's bottom edge" is `BOTTOM_EDGE` and its "0 px gap" is between
  that edge and `shell.body`'s top; its "44 px tall" is `HEIGHT`. The gate reads both constants. If
  #1127 drops the divider (or the constant), the rule follows what #1127 ships: the bottom edge is
  then the bar's own, `BOTTOM_EDGE == HEIGHT`.
- **Decision**: each new state anchors `shell.root`, `title_bar`, `shell.body`, `sidebar`,
  `session.area` (the session area, or the Overview in its place) and `status_bar`. Paths are read
  from the resolved fixture after regeneration, never from the source
  (`layout-snapshot.md#anchors`). A new gate `crates/micold-client/tests/gates/window_regions_fit.rs`,
  included in `tests/layout_snapshot.rs` with `#[path]` like `gates/status_bar_fits.rs`, asserts on
  both states in both schemes:
  1. `title_bar` height equals the title bar's height constant in
     `micold_core::tokens::anatomy` (today `app_bar::HEIGHT` = 64.0; #1127 sets it to 44) and its
     top is 0;
  2. `status_bar` height equals `anatomy::status_bar::HEIGHT` (28.0) and its bottom is the window's;
  3. `title_bar.y + app_bar::BOTTOM_EDGE` (the bar plus its hairline) equals `shell.body` top (gap
     0, SC-004), `shell.body` bottom equals `status_bar` top;
  4. `sidebar` and `session.area` each span `shell.body`'s height, `sidebar` right edge equals
     `session.area` left edge;
  5. in `window-overview` only, a state-level assertion on the `State` the state builds (the
     records hold geometry, not style): the Overview is open, and the sidebar's selected row is
     the Overview row. The selection's field or accessor comes from #1125 M3 as merged (open until
     stage 2, as in R5).
- **Rationale**: SC-004 is a claim about numbers, and the fixture can only catch a change, never a
  defect older than it (`layout-snapshot.md#3-the-containment-invariant`). The gate is also each new
  anchor's "property only its own element satisfies" that the page requires ("Add an anchor, add
  its property"). Reading the constants, not literals, keeps one definition (the FR-029a argument in
  `micold_core::tokens`).
- **Rejected**: extending `gates/status_bar_fits.rs` (it holds texts at the smallest window, a
  different question); asserting 44 as a literal; no gate, fixture only (SC-004 would pass on a
  regenerated defect).
- **Test-first**: the gate is test code over an unchanged layout, so it passes when written. Its
  red is shown by mutation (quickstart §A3): moving a region edge by 1 px fails the fixture naming
  the state (SC-005), breaking each of assertions 1–4 by a 1 px edit fails the gate, and building
  `window-overview` with the Overview closed fails assertion 5; all are reverted.

## R7 — Fixture regeneration

- **Decision**: M3 regenerates with `UPDATE_LAYOUT_SNAPSHOT=1 mise exec -- cargo test -p
  micold-client --test layout_snapshot` on main after the last of #1124–#1127 merged, then runs the
  same test without the variable to confirm byte equality (FR-013), and records the warm cost of the
  two states in `layout-snapshot.md` (as the page asks).
- **Rationale**: the page's own procedure; a regeneration before #1127 merges would be redone.
- **Rejected**: regenerating in stage 1 (no new state yet, and the fixture already equals main).

## R8 — How the visual pass draws the window

- **Decision**: the application's own `ui::view` drawn through its tiny-skia renderer from the two
  M3 states, as #1126 did in the cloud container (`specs/1126-session-context-header/visual-pass.md`
  *How it was run*), at 1280×800 in light and dark, 4 PNGs under
  `specs/1128-layout-redesign-docs/visual-pass/`. The render harness lives in the session scratchpad
  and is not committed; the record names the commit and the state names. Where a display is
  available, the `visual-pass` skill (Xvfb, `ICED_BACKEND=tiny-skia`, live client) is equally
  acceptable (spec Assumptions).
- **Rationale**: the covered states are the exact composition the boards draw and are reproducible
  from fixed data; no daemon or GitHub is needed. FR-020 forbids app changes, so the harness stays
  out of the tree.
- **Rejected**: a live window with a real project (unreproducible; the cloud container has no
  daemon); committing a render binary (out of scope; a second registration site for states would
  also fail `layout_coverage_registry.rs` if it named `CoveredState`).

## R9 — How contrast is measured (FR-016)

- **Decision**: per region, the visual pass cites the composition-contrast test that holds that
  region's text on its fill in both schemes, from
  `crates/micold-client/src/ui/material/composition_contrast.rs` (status bar:
  `the_status_bars_text_and_dots_are_legible_on_its_fill`; sidebar:
  `a_sidebar_group_header_is_legible_on_the_sidebar`,
  `a_two_line_rows_text_is_legible_on_the_sidebar_and_when_selected`; Overview cards:
  `an_overview_cards_header_and_body_text_is_legible_on_their_fills`; and the tests #1126/#1127
  added for the header, strip and title bar, if any). Text a test does not hold is measured with
  `micold_core::tokens::contrast` on the two colours the screenshot shows (a solid glyph-core pixel
  and its fill), in a scratchpad probe, and recorded with its ratio. Under 4.5:1 is *differs by
  defect* (D1).
- **Rationale**: those gates read the very style functions the view calls, so they measure what is
  drawn; one formula (`tokens::contrast`) for both routes.
- **Rejected**: eyeballing (FR-016 asks for a number); a new committed contrast gate (a fix-like
  change; a missing gate is a follow-up under D1).

## R10 — Link and anchor check (FR-019)

- **Decision**: `site/checks/links.sh --sources` (lychee with `--include-fragments`, offline), run
  in CI by `.github/workflows/ci.yml`; locally it needs `lychee` (`cargo install lychee`). If it
  cannot be installed in the container, each changed link's anchor is checked against the target's
  headings with GitHub's slug rule by hand, and CI's run is the gate.
- **Rationale**: the check already exists and covers fragments; no new tool.
- **Rejected**: a new link script.

## R11 — Shortcuts in both forms (FR-018)

- **Decision**: every chord the four pages name is checked for its macOS form in the same sweep;
  the wording follows `appearance-theming.md#status-bar`'s ("Ctrl+Shift+O"; "Cmd+Shift+O" on
  macOS). Chords are taken from `crates/micold-client/src/keymap.rs`, not from the mockup.
- **Rationale**: Principle VI; the keymap is the source the app draws its own hints from.
- **Rejected**: a table of chords per page (would duplicate `terminal-panes.md#keyboard-shortcuts`).
