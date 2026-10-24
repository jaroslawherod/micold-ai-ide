# Research: Worktree context header and session tabs above the terminal (#1126)

Each decision names what was chosen, why, and what was rejected. Paths are workspace-relative;
`client/` is `crates/micold-client/`, `core/` is `crates/micold-core/`.

## R1. Where the header's text rules live

**Decision**: A new render-free module `micold_core::context_header` (`core/src/context_header.rs`)
holds every rule with a branch in it: `HeaderText::of(&CardModel, Option<PrChipInput>,
CountsReading)` gives the title and its tooltip, the state word, the branch chip (short branch, full
branch as tooltip), the pull request chip text and the Changes button label. `ChangeSummary::of(&ChangeList)`
sums the counts. The header widget only lays these values out (Principle I exception).

**Rationale**: FR-002 to FR-006 are text rules; Principle I and feature 1120's FR-018 put such rules
in `micold-core`, unit-tested by `mise run test-core`. The header reuses 1120's
`overview_text::card_title` and `short_branch` through the `CardModel`, so it cannot disagree with
the card (SC-001).

**Alternatives rejected**:
- Compute the words in `client/src/ui/session_area.rs`: reachable only through the GUI build, and a
  second copy of rules 1120 already moved to core.
- Add the fields to `overview_text::CardText`: the header needs the current pull request and the
  change counts, which a card does not show; mixing them would make every card carry unused fields.

## R2. Where the header's facts come from: the Overview card's model, built for one location

**Decision**: `client/src/ui/overview.rs::card(app, &Place)` stays the one builder of a `CardModel`.
A new `pub(crate) fn location_card(state, &SessionLocation) -> Option<CardModel>` resolves the
`Place` (Default, or the worktree node with that `dir_name` from the sidebar tree) and calls it.
The header reads `title`, `state`, `reason` and `branch` from that model.

**Rationale**: FR-002 to FR-004 and SC-001 ask for "the same title / state its Overview card shows".
Building the model with the card's own function is the only way the two cannot drift. The
Default entry's model has no branch (the card's rule), so the header shows the title *Default*, the
root's card state and no branch chip, which is US1 scenario 4's "follows the Overview card's rule …
nothing else is invented".

**Alternatives rejected**:
- Re-derive title and state in the header from the sidebar node: a second derivation of
  `card_state` signals (`location_state`, agent requests, pull request signal) that would drift.
- Give the Default entry the root's branch in the header only: contradicts "the same title its
  Overview card shows".

## R3. The issue title must be read without the Overview having been opened

**Decision**: `client/src/shell/issues.rs::on_overview_opened` is split: its body becomes
`request_titles(app, dirs: &[String])` (seed from the form's held issues, then `titles_to_fetch`,
then `send_remotes_read`), called by `on_overview_opened` with every worktree's dir as today, and by
the session-area follow step (R6) with the shown worktree's dir only, when the shown location
changes. `titles_to_fetch` already skips a number with any entry, so each title is asked once per
window.

**Rationale**: US1 scenario 1 expects the issue title in the header of a worktree the user opened
straight from the sidebar. Today `overview.titles` is filled only on Overview open
(`issues.rs:666`). Same lookup, same switch (`github_issues_enabled`), same route check, so the
header's title is the card's.

**Alternatives rejected**:
- Fetch every worktree's title at project open: reads issues nobody looks at; the Overview's rule
  (FR-019 of 813, read on a named event) stays the model.
- Show the branch until the Overview is opened: fails US1 scenario 1.

## R4. The pull request chip: the sidebar's current pull request, in words

**Decision**: The chip reads `pr_status.statuses.get(branch)` (the reading the sidebar's
`PullRequestIndicator` draws from, `features/sidebar.rs::row_pull_request`), only while
`pr_status.enabled`. Its text is `PR #<n> · <state word> · <phrase>`; the state word is
`PrMark::word` (Open, Draft, Merged, Closed) mapped from `PrState` as `ui/sidebar.rs::indicator_marks`
does, the phrase is `StatusPhrase` for an open or draft one (*checks failing*, *changes requested*,
*merge conflict*, *checks pending*, *approved*, *checks passing*), absent otherwise. The chip
leads with the sidebar's own `PullRequestIndicator` (state and check glyphs, so state is not told by
colour alone) and is drawn in its stale form under the sidebar's rule
(`pull_request::is_stale(read_at, now)`), with the reading's age in its tooltip. `StatusPhrase`
gains `of_status(&PullRequestStatus)`; both constructors share one private candidate list.

**Rationale**: FR-005 and US1 scenario 5: one source with the sidebar, so the two never disagree.
`StatusPhrase` is feature 1123's precedence; reusing it keeps the Overview rows, the card reason and
the header naming a pull request alike. The edge case "the last value is not shown as current" is
the stale form, as in the sidebar.

**Alternatives rejected**:
- The Overview's `PrDisplay` counts (*2 open · 1 merged*): FR-005 names *the current* pull request,
  its state and checks, which counts do not carry.
- `pr_status.lists` (feature 1123's listing): read only for the Overview; a second source would
  disagree with the sidebar's indicator between readings.
- A new phrase rule for `PullRequestStatus`: a second precedence order; `of_status` shares the list.

## R5. The change counts: the Changes view's own list read, for the shown location

**Decision**: `features::changes::State` gains `header: HeaderCounts` (entry, sequence number,
`Load<ChangeSummary>`). New messages `Msg::HeaderFollowed { project, entry: Option<SessionLocation> }`
and `Msg::HeaderCountsRead { seq, result: Result<ChangeSummary, String> }`, new effect
`Effect::ReadCounts { seq, entry }`. The shell runs it with the same `local` + `read_list` the view
uses (`shell/changes.rs:228`, `read_list` at :569), under `Toggles::default()` (both kinds on, as
the view opens), and reduces the list with `ChangeSummary::of` off the update thread. `Msg::Changed`
(the watch's wake) re-reads both the open view's list and the header's counts. A read that fails, or
an entry this window cannot read (the service runs on another computer: `local` refuses), leaves
`Load::Failed` and the button shows no counts (FR-006, edge case *Failure*). While a read is under
way after an earlier answer, the last answer is not shown (`Load::Loading { last }` is ignored by the
label): US3 scenario 4, "no stale figure".

**Rationale**: US3 scenario 1 asks for the counts "the way the Changes view's list counts them";
using its reader, its scope rule (Default → uncommitted only, worktree → against its review base)
and its default toggles makes that true by construction. The spec's assumption (background read for
the shown location only) is the `HeaderFollowed` trigger.

**Alternatives rejected**:
- Take the counts from the open view's list: it exists only while the view is open, and carries the
  user's toggles, so the header would change when a toggle is pressed.
- `git diff --shortstat` in a new reader: a second counting rule (binary files, renames, untracked
  files) that would differ from the view's list.
- Hold the last figure during a re-read: US3 scenario 4 forbids a stale figure.

## R6. When the header follows the shown location

**Decision**: The root's `State::update` ends with a follow step: when
`current_session_location()` (with the active project) differs from `changes.header`'s entry, it
applies `Msg::HeaderFollowed`, whose effect (`ReadCounts`, or nothing for `None`) goes through the
existing `changes.pending` slot that the shell drains with `take_changes_effect` (batched with an
effect already there). The shell, seeing the follow, also calls `request_titles` for a worktree
(R3). The file watch (`shell/subscriptions.rs:153`) is asked for the header's entry while a session
is shown and the Overview is closed, besides the open view's entry; iced keeps one subscription
for two equal keys, so a view and a header on the same worktree share one watcher.

**Rationale**: FR-014: the header changes on the same refresh as the sidebar (title, state, branch
and pull request are read from the same state each render) and the counts on the same refresh as the
view's (one `Msg::Changed` re-reads both). Deriving the follow from `current_session_location`
covers every way the shown session changes (sidebar, tab, chord, quick switcher, a session closing,
a worktree removed from another window) without a call at each site.

**Alternatives rejected**:
- Send `HeaderFollowed` from each selection site: a site added later would forget it.
- Watch every worktree: a recursive watch of locations nobody looks at wakes the window on every
  build (the reason the view watches only while open, `subscriptions.rs:150`).

**Cost**: the shown worktree is watched while a session is shown, which wakes the window once per
debounced burst of edits (`watch::Debouncer`), and each wake runs one `git diff` list read.

## R7. The tab strip's members, and who decides them

**Decision**: A new render-free module `client/src/features/session_strip.rs` builds
`StripModel::of(&State) -> Option<StripModel>`: the shown location's sessions in the order
`features::navigation::location_node(..).unfolded_sessions()` gives (main first, then helpers, the
sessions an agent started after the session that started them, folded or not), one `SessionTab` each;
directly after the shown session's tab, its `ShellTab`s in creation order and the `Plus`; then
nothing else (New session and the split buttons are fixed controls, R9). Each member carries its
key, label, whether it is marked (the shown session's marked tab is `marked_tab`: its session tab
when `StripTab::Ai`, else the shell tab), and its stopped mark (`process_stopped`, asked for that
member's own session and process). It also yields the shown session's status words
(`session_status`) and whether its restart is offered (`attached_process_restartable`), with the
restart message (`restart_message`). These render-free functions, `StripTab`, and the strip's
overflow and reveal arithmetic (`overflowing`, `strip_overflow`, `scroll_into_view`,
`marked_tab_index`) move from `client/src/ui/terminal.rs` into this module with their tests;
`ui/terminal.rs` and `ui/panes.rs` import them from there.

**Rationale**: Clarifications 1 and 2 fix the members and their order; `unfolded_sessions` is the
order the previous/next-session chords step through (`navigation.rs:136`), so the strip, the
chords and the sidebar agree. The rules (which session is shown, which tab marked, which stopped)
have branches, so they belong in a render-free module with unit tests (Principle I); the strip
widget is glue. Edge case *several sessions*: the stopped mark is asked per member, never from the
shown session's state.

**Alternatives rejected**:
- Build the members inside the view as `tab_strip_row` does today: decision logic in `ui/`.
- Leave the predicates in `ui/terminal.rs` and call them from the feature module: a render-free
  module would then depend on a rendering one.
- `listed_sessions` (respects folds): a session under a folded origin would have no tab while it is
  shown.

## R8. Session tab labels and widths

**Decision**: A session tab reads `<command> · <session name>` (`features::session::command_caption`
and `terminal::session_title`, e.g. *claude · main*), as the mockup draws it and as the pinned AI tab
named its CLI (feature 026-multi-provider FR-016a). Every session tab has one fixed derived width,
`material::tab::SESSION_WIDTH = SLOT_WIDTH + XS + SESSION_LABEL_WIDTH + XS + SLOT_WIDTH` with
`SESSION_LABEL_WIDTH = LABEL_MAX_WIDTH` (120 dp); a longer label ends in an ellipsis and the full
label, the session's role (main or helper) and its status are its tooltip. Shell tabs keep
`tab::WIDTH`. The "+" keeps its icon button's size.

**Rationale**: FR-010 keeps each tab at its own width while the strip scrolls; fixed widths let the
reveal and the edge-fade arithmetic stay derived from the member list rather than measured, which is
what fixed feature 027's stale-measurement fade (`terminal.rs::strip_overflow` doc). Uniform session
tabs keep feature 012 BUG-001's "a strip reads as a strip".

**Alternatives rejected**:
- Content-sized session tabs (`Tab::content_sized`): widths would have to be measured, reintroducing
  the stale content width that drew a fade over nothing (feature 027), and BUG-001's mismatched strip.
- One width for every member: ordinal shell tabs 2.5 times as wide as their label.

## R9. Strip layout: left to right, fixed controls at the right end

**Decision**: The strip is one row with a fixed child list: (1) the scrolling region
(`material::EdgeFade` around a horizontal `material::Scrollable` with id `TAB_STRIP_SCROLL_ID`) holding
the members **from the leading edge**, `Length::Fill`; (2) the status cluster: the shown session's
status words (`Ellipsized`, capped at `STATUS_MAX_WIDTH`, full words as tooltip) and, while the
shown process is stopped, the **restart** control as its last child; (3) **New session**
(`IconButton(Icon::AddSession)`, tooltip *Start another session…*); (4) the split buttons
(`panes::split_buttons` of the focused pane, as today). The status cluster, New session and the split
buttons are outside the scrolling viewport. Feature 027's trailing anchoring (`right_aligned_tabs`,
`leading_slack`, `natural_strip_width`, `pinned_ai_tab`) is removed; the AI conversation is the shown
session's own tab (Clarification 1), reachable in one press because it is the first member of the
shown session's group and the reveal scrolls it into view when it is marked.

**Rationale**: FR-008 places New session and the splits at the right end; the mockup starts the tabs
at the left. FR-010 keeps them at their place and size: being outside the viewport is what makes
that structural. US2 scenarios 5 and 6 put the status and restart above the terminal, beside the
selected tab group. The restart is the cluster's last child, so its coming and going moves no other
control (feature 023 FR-008a).

**Alternatives rejected**:
- Status text inside the selected tab: a fixed-width tab cannot hold *failed after 3 attempts*
  beside a name.
- Keep feature 027's right-aligned strip: the tabs would start under the split buttons' side and
  move left with each new tab, against the mockup's reading order.
- A labelled **New session** button: at 640 px the session area is 340 px wide (window minus the
  300 px default sidebar, `app.rs:26`); a label would leave the tab region a few pixels.

## R10. Presses while the member list changes

**Decision**: The scrolling region's row is a keyed row: the transcript's `KeyedColumn`
(`client/src/ui/transcript/keyed.rs`) is promoted to `client/src/ui/cdk/keyed.rs` as `Keyed` with
an axis (`Keyed::row` / `Keyed::column`, builder, `.into()`), and the transcript uses it from there.
Members are keyed `Session(SessionId)`, `Shell(SessionId, ShellInstanceId)`, `Plus`. The strip's own
row and the header's row keep a child list that never depends on state (R9, R11).

**Rationale**: FR-011 and US4 scenario 3: a press in flight must not move to a neighbour while the
tab count changes. Today's strip is a positional row, so a tab opened before the pressed one hands it
its neighbour's tree slot (feature 023 R1); in the new strip a session's shell tabs sit before later
session tabs, so opening a shell instance shifts them. Keyed slots follow the member. Promoting the
existing keyed widget follows Principle VIII rather than writing a second one.

**Alternatives rejected**:
- iced's `keyed_column`: column only, and it matches by index when the length is unchanged
  (`keyed.rs:5`).
- Accept the shift: FR-011 names the tab count explicitly.

## R11. The header's layout and components

**Decision**: `client/src/ui/session_area.rs::header` lays out, with a fixed child list: a column
(title line: `Ellipsized` at `TypeRole::Title`, full title as tooltip; chips row) and, at the right,
the **Changes** button. Chips are `material::Tag`, extended with `.leading(Element)` (a glyph or the
`PullRequestIndicator`) and `.max_width(f32)` (ellipsis under a ceiling, the chip sized by its
content below it), each in a `Tooltip` with its full text. The state chip uses the card's
`overview_card::palette(state)` fill and `state_icon(state)` glyph with the state word (colour,
glyph and word: FR-003's "more than colour"); branch and pull request chips are neutral
(`on_surface_variant` accent). The Changes button is `material::Button` (outlined variant) with
`Icon::Git` leading, label `Changes` plus the summary words (R1), `.selected(view open)`: the
pattern of the Overview button (feature 813 FR-012). Pressing it sends
`SidebarMsg::ReviewChangesRequested(location)` when the view is closed or open on another entry, and
`changes::Msg::Closed` when it is open on this one.

**Rationale**: Principle VIII: extend the shared chip and button rather than add a header-only
widget. The selected layer moves nothing when toggled. The chips row may hold one to three chips;
it is inside the column, so the Changes button's slot never moves (FR-011).

**Alternatives rejected**:
- A new `material::Chip`: `Tag` is the library's label chip; a second one is the duplication the
  component-reuse gate rejects.
- `ToggleChip` for Changes: it is a filter chip with a count, not a labelled action.

## R12. What the session area draws, and where

**Decision**: `client/src/ui/mod.rs::view_with`'s main-area branch changes only where a session is
active: `session_area::view` draws `column![header, body]`, where body is the Changes view when it is
open (no strip: its own heading and **Close** stay) or `column![strip, terminal::pane(..)]`
otherwise. `terminal::pane` loses its bottom bar and returns the pane area only. The header is drawn
when `current_session_location()` resolves; the strip whenever a session is active. Overview,
Compare, Settings and the empty state are unchanged. The Changes view opened with no session shown
draws no header (edge case *Empty*).

**Rationale**: FR-001 and US2 scenario 1 order the session area; US3 scenario 5 needs the header
above the open Changes view. Keeping the edit inside the main area's branch is what keeps the
rebase small against #1124 (status bar, the window column) and #1125 (sidebar).

**Alternatives rejected**:
- Header inside `terminal::pane`: it would vanish with the terminal when the Changes view opens.
- Draw the strip above the Changes view too: its tabs and splits act on panes that are not drawn.

## R13. Platform chord words in tooltips

**Decision**: `keymap.rs` gains `new_terminal_chord_label()` (as `overview_chord_label`), and the
"+" tooltip uses it instead of the literal *Ctrl+Shift+T*.

**Rationale**: Edge case *Platforms*: tooltips name Cmd on macOS. The bar's literal was wrong there.

## R14. Tests, gates and the layout snapshot

**Decision**:
- Core: `context_header` and `StatusPhrase::of_status` unit tests (`mise run test-core`).
- Client lib: `features/session_strip.rs` tests (members, order, marked, stopped per member,
  offsets/overflow/reveal arithmetic moved from `terminal.rs`); `features/changes.rs` tests for
  `HeaderFollowed`, `HeaderCountsRead` (stale seq dropped), `Changed` re-reading both; `cdk/keyed.rs`
  slot-follows-key test.
- Client integration: `tests/features_changes.rs` (follow step from `State::update`),
  `tests/session_area_stability.rs` replaces `tests/terminal_bar_stability.rs` (no focus- or
  shape-conditional child in the strip or header; no release-focus control), `tests/terminal_tabs.rs`
  and `tests/tab_reveal.rs` retargeted to the strip.
- Geometry gates: `gates/bar_controls_hold_their_size.rs` → `gates/strip_controls_hold_their_size.rs`;
  `gates/tabs_anchor_the_trailing_edge.rs` retired (feature 027 FR-003's trailing anchoring is
  superseded by FR-008) and replaced by `gates/strip_controls_anchor_the_trailing_edge.rs` (New session
  and splits at the strip's right end, in order); `gates/tab_children_fit.rs` re-anchored; new
  `gates/session_area_fits.rs`: at `MIN_WINDOW_SIZE` (640×480) and 1280×800, light and dark, with 1,
  6 and 12 members, no header or strip element overlaps another or leaves the window, Changes and
  split buttons whole (SC-004, FR-013); and nothing is laid out under the pane area (SC-005).
- Layout snapshot: `session-terminal-bottom-bar` becomes `session-area-one-tab`;
  `session-terminal-instance-tabs(-overflowing)` become `session-area-six-tabs` and
  `session-area-twelve-tabs`; new `session-area-changes-open` (header over the Changes view);
  anchors `session.header`, `session.header.changes`, `session.strip`, `session.strip.tabs`,
  `session.strip.new_session`, `session.strip.split`; fixture regenerated once the area is done.
- Visual pass: quickstart Part B.

**Rationale**: the spec's test layers per requirement (plan *Requirement map*); fonts are the
bundled Roboto on every platform and layout is in logical px, so the gates measure what each platform
draws (feature 1120 R14), and CI runs them on Linux, macOS and Windows (SC-004's per-platform clause).
