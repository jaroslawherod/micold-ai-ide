# Research: review source dropdown and two-column layout (#1272)

Each decision names the code it builds on and the alternatives it rejects. Paths are relative to
the repository root.

## R1: The control's model lives in a new core module `micold_core::review::source_menu`

- **Decision**: a pure, iced-free module `crates/micold-core/src/review/source_menu.rs` (NEW)
  decides the rows of the open list, their order, which are chosen, which can be picked, which rows
  the keyboard reaches, the closed label and the tooltip text (FR-001, FR-002, FR-005, FR-009,
  FR-010, SC-005). Its input is plain data the client already holds: the shown source (working tree,
  or a pull request's number and listed title), whether Files is on, the core
  `review::base::Toggles`, whether Committed is available, and the pull-request list state with
  `&[micold_core::pull_request::BranchPullRequest]` (already a core type). The client maps its
  `OpenView` onto that input in one function (`features::changes::source_menu_input`, NEW).
- **Rationale**: the issue's acceptance asks for the pure logic in micold-core with unit tests, and
  it can then ship before #1270 (R1) and #1269 (R0), which reshape `ui/changes.rs` and parts of
  `features/changes.rs` but not this module. Every input type is already core
  (`Toggles`, `BranchPullRequest`, `BranchPrState`, `CheckStatus`), so no client type crosses.
- **Rejected**: (a) deriving rows inside `ui/changes.rs` at draw time: decision logic in GUI glue
  (Principle I's exception does not cover it). (b) Extending the client's `PrSection` enum in
  `features/changes.rs`: render-free but not in core, against the issue's acceptance, and it
  conflicts with #1270's edits to the same file.

## R2: The pull-request list states keep one source of truth

- **Decision**: core gets `source_menu::PrRows<'a>` with the arms of today's client `PrSection`
  (`Absent`, `Loading`, `Unavailable`, `Empty`, `List { prs, stale }`). The client's
  `features::changes::section(view, enabled, now)` stays the only place that reads `PrList` and the
  stale threshold; it returns `PrRows` (its `PrSection` enum is replaced by the core type, so there
  is one enum, not two). The wording (`Reading pull requests…`, `Pull requests can't be shown for
  this worktree`, `No pull requests from this worktree yet`, `Possibly stale`, `Pull requests`,
  `Working tree`) moves from `ui/changes.rs` constants into `source_menu` constants, and the
  client's constants become `pub use` re-exports so existing tests keep their names (FR-005: same
  wording).
- **Rejected**: copying the strings into core and leaving the client's constants (two copies
  `mise run duplication` would flag, free to drift).

## R3: Files stays a mode over the shown source (clarify decision 1)

- **Decision**: `Files` is a row of the list, not a third arm of `Source`. Picking it sends the
  existing `Msg::FilesPaneToggled` (tree read via `request_tree`), and picking a source sends the
  existing `Msg::SourceChosen(..)`, so `reread_after_source_change` keeps re-reading the tree and the
  browsed file and clearing Back exactly as in 1150 (FR-006, FR-007, SC-004). The label prefixes
  `Files · ` while `view.files.shown` and drops the kinds suffix.
- **Rejected**: a `Source::Files` arm (the spec's clarification rejects Files as a source; it would
  also break `read_target`/`SourceKey`, which key expansions and reads per source).

## R4: Kind chips while a pull request is shown

- **Decision**: `Msg::CommittedToggled` and `Msg::UncommittedToggled` keep their names and, when a
  pull request is shown, first call `show_working_tree(state)` (NEW private helper holding today's
  `Msg::SourceChosen(None)` body, `features/changes.rs:1840-1852`: source back to
  `Source::Changes`, the entry's remembered pull request forgotten, `reset_diff_pane`), then flip the
  kind and `request_read`. `SourceChosen(None)` calls the same helper and then closes the list; the
  chip arms do not close it (FR-002, FR-004 s3). The flip stays a no-op for
  Committed on the Default entry (`committed_available`). Because the source changes,
  `reread_after_source_change` runs as for any other source change (Story 3 s4).
- **Rejected**: a new `Msg::KindChosen` (two messages for one toggle); leaving the chips inert while
  a pull request is shown (the spec makes it a deliberate change).

## R5: The closed label and long titles

- **Decision**: `source_menu::label(&input) -> Label { fixed: String, title: Option<&str> }`.
  `fixed` is the part that must stay visible (`Files · `, `#<n> · `, `Working tree`, the kinds
  suffix); `title` is the pull request's title, the only part that may give way. The trigger draws
  `fixed` as text and `title` with the existing `material::Ellipsized`, which measures at layout and
  ends in `…` (Edge "Long PR title"). Which part truncates is decided in core and tested; how many
  pixels fit is the widget's measurement (GUI glue). `Label::full()` is the untruncated text for
  the tooltip and tests.
- **Kinds suffix**: computed over the kinds that can be listed: `committed && committed_available`
  and `uncommitted`. Both listable kinds on: `Working tree`. One on: `Working tree · committed` /
  `Working tree · uncommitted`. None on: `Working tree · nothing shown`. On the Default entry
  Committed cannot be listed, so `Working tree` with Uncommitted on and `Working tree · nothing
  shown` with it off: the same rule as `list_body`'s `BOTH_HIDDEN` check, so the label and the
  empty state never disagree.
- **Unlisted pull request**: while the shown number is not in the list (a refresh in flight before
  the fallback of 1003), the label reads `#<n>` with no title.
- **Rejected**: character-count truncation in core (`reply::shortened` style): the title would cut
  at a width unrelated to the header's space; pixel fitting in core (core cannot shape text).

## R6: The shared component: `material::MenuSelect` (NEW), built on `cdk::picker::Picker`

- **Decision**: a new builder component `MenuSelect` in `crates/micold-client/src/ui/material/
  menu_select.rs`: a text trigger (label parts per R5, chevron, tooltip) and a floating list of
  caller-built rows, composed with the existing `cdk::picker::Picker` (`Picker::new(field, menu,
  open, gap).keyboard(..).on_move(..).on_pick(..).on_dismiss(..)`) and `cdk::tooltip::Tooltip`.
  Rows are a small builder enum `MenuSelectRow` (`option(content, chosen, on_pick)`,
  `option_with_chips(content, chosen, on_pick, chips, note)`, `heading(text)`, `note(text)`), reusing
  `PullRequestRow` for pull requests, `ToggleChip` for the chips and the list surface and row style
  of `material::picker`. Openness and highlight are the caller's (the reducer's), as with the search
  picker, because the list must stay open on a chip press and across a pull-request refresh, and
  close on a pick. A showcase entry poses it closed, open, open with a long title, and with every
  list state.
- **Rationale**: Principle VIII and the spec's assumption: `Select` (`material/select.rs`) owns its
  own openness and takes `T: ToString` options, so it cannot hold chips inside an option or stay
  open on a chip press; `MenuOverlay`/`MenuItem` (`material/menu.rs`) are icon + label actions. The
  library gains the variant; the view builds no one-off. `Picker` is the one floating-list
  mechanism, so `tests/one_overlay_implementation.rs` needs no new exception.
- **Rejected**: extending `Select` with a rows mode (its widget assembles its subtree in
  `Widget::layout` around self-owned state; caller-owned openness is the opposite design);
  `MenuOverlay` with custom items (anchored to a point, built for context menus, no keyboard
  highlight over disabled rows).

## R7: Keyboard

- **Decision**: the keyboard reaches, in order: Working tree, Committed chip, Uncommitted chip, each
  pull request, Files (`source_menu::stops(&rows) -> Vec<Stop>`). Arrows move with
  `typeahead::move_highlight` (saturating); `Enter` takes the highlighted stop through
  `typeahead::intent_for`, which already refuses an unavailable one (Committed on the Default entry);
  `Esc` and `Tab` dismiss; a refreshed list re-seats the highlight with `typeahead::clamp_highlight`.
  `Enter`/`Space` on the focused trigger opens it (the trigger is a button). Taking a chip stop
  keeps the list open; taking any other stop closes it (FR-002, FR-011).
- **Rejected**: chips reachable only by mouse (FR-011 asks the control be operable by keyboard, and
  the chips are part of it); a second keyboard rule beside `typeahead` (it has drifted before, see
  its docs).

## R8: Layout: header placement and two columns

- **Decision**: the header row becomes `[title + subtitle (Fill, subtitle ellipsized)] [MenuSelect]
  … [Close]`; the left column loses `pull_request_section`, the toggle row and the Default note
  (the note moves into the Working tree option). Once #1270 has removed the review panel's column,
  the body is `row![file column Fixed(FILE_COLUMN_WIDTH = 260.0), diff Fill]`. At 1280×800 with
  `spacing::LG = 24` padding and gap: body 1232 px, diff 1232 − 260 − 24 = 948 px = 77% ≥ 70%
  (FR-008, SC-002); the file column keeps 260 px on resize (Story 4 s2). The PR section's
  `PRS_MAX_HEIGHT` cap is replaced by the list's own cap: never taller than the space below the
  header (Edge "Many pull requests"), scrolling inside.
- **Sequencing**: #1270 rewrites the same header row and body row in `ui/changes.rs`, and #1269 may
  move the header into a tab. All view wiring is therefore one milestone gated on #1270 having
  merged (and #1269, if it is open then, merged or confirmed not to touch `ui/changes.rs`'s header).
  The model, the reducer and the component ship before it.
- **Rejected**: wiring the header now and rebasing over #1270 (two threads editing the same rows at
  once; the conflict lands on whoever merges second, and FR-008 cannot be checked until #1270
  anyway); a resizable column (R3, #1273).

## R9: No storage, protocol or daemon change

- The control's openness and highlight are in-memory view state on `OpenView`, reset when the view
  opens; the chosen source keeps its existing per-entry memory (`State::remembered`). Nothing is
  persisted or sent (Principle IV). Pull requests are read exactly as today (spec Out of scope).
