# Visual pass — the whole window against the mockup (M4, T030–T034)

Record per [contracts/visual-pass-record.md](contracts/visual-pass-record.md); quickstart Part B.

## Header

- **Commit**: `ad1fc7b30e7ba0c28f8b3a8540af135d0449a826` (main after #1124–#1127 and M2 #1429, plus
  M3's commits: the two whole-window covered states). M3 reaches main by rebase as #1431, which
  rewrites this SHA; `crates/*/src` here is byte-identical to main at
  `58ef1146da2e1cd31d7def7217c08b7132538dd3` (`git diff 58ef1146 ad1fc7b3 -- 'crates/*/src'` is
  empty: M3 changed tests and specs only), so the frames are what that main draws, from #1431's
  covered states.
- **How it was run**: research R8's route. In the cloud container, with no daemon or GitHub, the
  application's own `ui::view` (`ui::view_with`, as the layout snapshot's `view_of` calls it) was
  drawn through its tiny-skia renderer from a harness in the session scratchpad, not committed. The
  harness builds the covered states `window-session-view` and `window-overview`
  (`tests/support/covered_states.rs`), sets the theme preference to Light or Dark, mounts the view
  at 1280×800 with the shipped Roboto, Material Symbols and Fira Mono faces, hands it 60 redraw
  frames so entrances finish, then one more redraw on the final tree, and saves the RGBA frame.
  The states: every sidebar group (Needs action, Working, Idle, Done), the session shown at
  `feat-working` with its main session, two shell tabs and a helper, the sandbox up, connected;
  and the same with the Overview open in the session area's place, not full screen.
- **Size**: 1280×800, scale 1. **Renderer**: tiny-skia (what `ICED_BACKEND=tiny-skia` selects).
- **Not checked**: macOS and Windows by eye (Linux only); sizes other than 1280×800; failure states
  (held by `specs/1124-bottom-status-bar/visual`); a live terminal grid (a fixture has no process,
  so the pane reads *Starting…*); split panes; hover, focus and tooltips.
- **Mockup**: *Micold Layout Redesign*, boards *Proposal: session view* (`Main.dc.html`) and
  *Proposal: Overview* (`Overview.dc.html`). The boards are drawn in light only; a dark frame is
  judged against the same board for structure, and on its own for legibility.
- **Contrast** (FR-016, research R9): each region cites the composition-contrast tests in
  `crates/micold-client/src/ui/material/composition_contrast.rs` that hold its text on its fill,
  and every region was also measured on the frames: per text, the glyph-core pixel with the most
  contrast against the most common colour of its box, the ratio from `micold_core::tokens::contrast`
  in the scratchpad harness. The column gives the lowest ratio among the texts measured in the
  region (titles, second lines, chips, annotations, band words, labels, hints); icons are not text.
  Fixture names (`project`, `feat/working`, *no daemon*, *Claude Code*) stand where the boards
  have `micold-ai-ide`, `fix/1117`, *local* and *Claude*; that is data, not a difference.

Frames: [visual-pass/](visual-pass/).

## session-view, light — [session-view-light-1280.png](visual-pass/session-view-light-1280.png)

| Region | Verdict | Reason | Contrast |
|---|---|---|---|
| Title bar | differs on purpose | App mark, breadcrumb `project ▾ / feat/working`, jump field with Ctrl+Shift+P, Settings, ⋯, as the board. No *Plan usage 62%*: plan usage stays in the status bar (`specs/1127-title-bar-breadcrumb/spec.md` Clarifications, after `specs/1124-bottom-status-bar/spec.md` Clarifications) | `the_breadcrumbs_words_are_legible_on_the_title_bar`, `the_jump_fields_words_and_edges_are_legible_on_the_title_bar`; measured 8.92:1 (location crumb), 7.25:1 (jump field) |
| Sidebar | differs by defect | Region issue #1125. Two defects, follow-ups in the ledger's *Follow-ups not done*: the row chip *no daemon* reads 4.40:1 (`#625B71` on `#DAD3DC`), under 4.5:1; the Overview row's badge reads *1 need action*, the plural verb with a count of one. On purpose besides, per `specs/1125-sidebar-state-groups/visual-pass.md` B10: Done is a collapsed group, not a footer (research R8); ⋯ after add and hide (FR-015); the chips wrap, *untyped* on a second line (FR-016); second line is branch and sessions only (FR-011); *Default* first in Idle (FR-003); the row chips are today's (FR-013); the session rows under the expanded worktree are unchanged by #1125 | `a_sidebar_group_header_is_legible_on_the_sidebar`, `a_two_line_rows_text_is_legible_on_the_sidebar_and_when_selected`, `the_overview_row_and_its_badge_are_legible`; measured 8.48:1 (headers, second lines), 7.24:1 (selected row's annotation), **4.40:1 (*no daemon* chip)** |
| Session area | differs on purpose | Header: title, state and branch chips, Changes at the right; the title is the branch because the fixture has no issue title, and there is no pull request chip or count because the fixture has none (title rule and chips: `specs/1126-session-context-header/spec.md` US1; counts: its M3). The strip holds one tab per session with the shown session's shells and "+" after its AI tab, then *running*, New session as "+", and the split buttons, where the board draws *New session* as a word tab (`specs/1126-session-context-header/spec.md` Clarifications). One pane, not the board's two; it reads *Starting…* (fixture) | No test holds the header and strip; measured 16.35:1 (title), 13.25:1 (state chip), 6.41:1 (branch chip), 8.92:1 (tab labels, *running*), 6.14:1 (Changes) |
| Status bar | matches | Dot and connection, sandbox, CLI, then the Overview and Jump hints at the right, as the board. *Daemon connected · no daemon* is the connection, then where the shown worktree runs: the fixture's worktree is bound to none (`ui/status_bar.rs` `location`, feature 1124 research R6) | `the_status_bars_text_and_dots_are_legible_on_its_fill`; measured 8.04:1 |
| Whole window | matches | One title bar (43 px and its 1 px divider), the sidebar column to x 266 and the session area beside it, the status bar 28 px across the full width; no stacked chrome row; nothing overlaps | — |

## session-view, dark — [session-view-dark-1280.png](visual-pass/session-view-dark-1280.png)

| Region | Verdict | Reason | Contrast |
|---|---|---|---|
| Title bar | differs on purpose | As in light: no plan usage in the bar (`specs/1127-title-bar-breadcrumb/spec.md` Clarifications) | the two title-bar tests above; measured 10.85:1 (location crumb), 7.22:1 (jump field) |
| Sidebar | differs by defect | Region issue #1125. The badge reads *1 need action* (follow-up in the ledger); the *no daemon* chip is legible here. On purpose besides, as in light, per `specs/1125-sidebar-state-groups/visual-pass.md` B10 (Done group, ⋯, wrapping chips, two-line rows, *Default* first, today's row chips) | the three sidebar tests above; measured 10.05:1 (headers, second lines), 7.24:1 (selected row, badge), 6.25:1 (*no daemon* chip), 5.48:1 (selected row's annotation *claude*) |
| Session area | differs on purpose | As in light (`specs/1126-session-context-header/spec.md` US1 and Clarifications; one pane, *Starting…*, fixture) | measured 14.35:1 (title), 7.24:1 (state chip), 6.92:1 (branch chip), 10.85:1 (tab labels), 10.83:1 (Changes) |
| Status bar | matches | As in light | `the_status_bars_text_and_dots_are_legible_on_its_fill`; measured 9.61:1 |
| Whole window | matches | As in light; every region drawn in dark, no light surface left | — |

## overview, light — [overview-light-1280.png](visual-pass/overview-light-1280.png)

| Region | Verdict | Reason | Contrast |
|---|---|---|---|
| Title bar | differs on purpose | Breadcrumb `project ▾ / Overview`, as the board; no plan usage (`specs/1127-title-bar-breadcrumb/spec.md` Clarifications) | the two title-bar tests above; measured 8.92:1 (crumb), 7.25:1 (jump field) |
| Sidebar | differs by defect | Region issue #1125. The Overview row is the selected one, in the primary container, with its badge, as the board; but the badge reads *1 need action*, and the *no daemon* chip reads 4.40:1, as on the session view; follow-ups in the ledger. On purpose besides: as on the session view (`specs/1125-sidebar-state-groups/visual-pass.md` B10) | the three sidebar tests above; measured 13.25:1 (Overview row, badge 13.26:1), **4.40:1 (*no daemon* chip)** |
| Overview | differs on purpose | Three cards per row at 1280, as the board. The cards carry their state on a coloured header band with the type at its right, not the board's 4 px stripe on a neutral card: card styling is feature 1120's (this spec's *Out of scope*). No title line with a count and no state filter chips; the header holds the Full screen toggle (feature 970, `ui/overview.rs` plan D5) | `an_overview_cards_header_and_body_text_is_legible_on_their_fills`; measured 7.25:1 (Idle header), 5.04:1 (the type word on the Idle band, *Project root*, *Other*), 8.92:1 (body), 6.14:1 (Full screen) |
| Status bar | differs on purpose | No CLI item while the Overview is the body (`specs/1124-bottom-status-bar/spec.md` US4 scenario 3); the hint reads *Ctrl+Shift+O Overview* and the Jump hint stays, where the board writes *Ctrl+Shift+O closes Overview* (same scenario's wording) | `the_status_bars_text_and_dots_are_legible_on_its_fill`; measured 8.04:1 |
| Whole window | matches | As on the session view: one title bar, sidebar beside the Overview, the status bar across the bottom; the Overview fills the session area's place; nothing overlaps | — |

## overview, dark — [overview-dark-1280.png](visual-pass/overview-dark-1280.png)

| Region | Verdict | Reason | Contrast |
|---|---|---|---|
| Title bar | differs on purpose | As in light (`specs/1127-title-bar-breadcrumb/spec.md` Clarifications) | the two title-bar tests above; measured 10.85:1 (crumb), 7.22:1 (jump field) |
| Sidebar | differs by defect | Region issue #1125. The badge reads *1 need action* (follow-up in the ledger); the *no daemon* chip is legible here. On purpose besides: as in light (`specs/1125-sidebar-state-groups/visual-pass.md` B10) | the three sidebar tests above; measured 7.24:1 (Overview row, badge), 6.25:1 (*no daemon* chip) |
| Overview | differs on purpose | As in light: header bands, not stripes (feature 1120, this spec's *Out of scope*); Full screen toggle in the header (feature 970) | `an_overview_cards_header_and_body_text_is_legible_on_their_fills`; measured 5.48:1 (Idle header, the type word on the Needs action band), 4.77:1 (the type word on the Idle band, *Project root*, *Other*), 5.68–5.77:1 (the other bands' type words), 10.85:1 (body), 10.83:1 (Full screen) |
| Status bar | differs on purpose | As in light (`specs/1124-bottom-status-bar/spec.md` US4 scenario 3) | `the_status_bars_text_and_dots_are_legible_on_its_fill`; measured 9.61:1 |
| Whole window | matches | As in light; every region drawn in dark | — |

## Count (quickstart Part B)

4 screenshots, 20 verdicts: 6 *matches*, 10 *differs on purpose*, 4 *differs by defect* (the
sidebar on every frame: the badge's *1 need action*, and in light the *no daemon* chip at 4.40:1).
Every non-*matches* verdict names its reason. No code changed (FR-020): the defects are follow-ups
in the ledger.
