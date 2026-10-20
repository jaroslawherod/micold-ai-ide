# Research: Overview cards, readable layout and content (#1120)

Each decision names what was chosen, why, and what was rejected. Paths are workspace-relative.

## R1. Where the card's text rules live

**Decision**: A new render-free module `micold_core::overview_text` (`crates/micold-core/src/overview_text.rs`)
holds the title rule, the short branch, the branch line, the sessions line, the progress row and the
activity text, gathered in one value `CardText::of(&CardModel)`. The pull request row stays on
`PrDisplay::footer` in `crates/micold-core/src/overview.rs`, whose rule changes (R6). The markup
sanitiser is its own module `micold_core::markup` (R7), because `hook_activity` uses it too.

**Rationale**: FR-018 and Principle I: every rule with a branch in it must be unit-tested without the
GUI. Today the title rule (`features::overview::summary`, client), the progress words
(`overview_card::progress_words`, client) and the footer (`overview_card::words`, client) sit in the
client, where only the GUI test binary reaches them. One `CardText` value is the spec's *Card text*
entity and leaves the card widget pure glue (constitution, Principle I exception).

**Alternatives rejected**:
- Keep the rules in `overview_card.rs` and test them there: they would stay reachable only through
  the client crate and its GUI build (`mise run test-core` would not see them), against FR-018.
- Put everything in `overview.rs`: it is already 1,132 lines of state rules; the text rules change
  for different reasons (copy, not state).

## R2. The title rule and what happens to the summary line

**Decision**: `CardModel.summary: String` becomes `CardModel.title: String`, computed in
`ui/overview.rs::card` by `overview_text::card_title(issue_title, branch, display_name)`: the known
issue title, else the branch, else the display name. The client passes
`Some(title)` only for `TitleLookup::Known`. `features::overview::summary` is removed. The separate
summary line goes: the title now says what the summary said (US2). The title's tooltip is the
full title then the display name, or the display name alone when it is the title (FR-006, FR-015). The project root's title is its display name *Default*, because the card
model carries no branch for the root today (`ui/overview.rs` passes `None`) and the spec puts new
data sources out of scope; the header's type word says *Project root* instead of the old summary.

**Rationale**: The issue asks that the most prominent line say what the worktree is for. The old
layout said it twice (folder name as title, then a summary line), which is the duplication the issue
complains about.

**Alternatives rejected**:
- Keep `summary` and add `title`: two fields saying the same thing, and a renderer free to show both.
- Read the root's branch to title it: a new data source (spec Out of scope); the edge case's "unless
  the root's branch is known" stays true without it, since the card model never knows it.

## R3. Short branch parsing

**Decision**: `short_branch(branch) -> Option<String>` returns `type/number` when the branch is
`<type><sep><digits>` optionally followed by `_` or `-` and a non-empty slug, with `<sep>` one of `/`
or `-`, `<type>` a `naming::ConventionalType::from_token` token, and `<digits>` ASCII decimal digits;
`None` otherwise, and the card shows the branch unchanged. Hand-written byte scan, no regex.
`branch_line(title, branch)` is `None` when there is no branch or the title equals the branch (US2.8),
else the short (or unchanged) branch with the full branch as tooltip.

**Rationale**: FR-007's grammar, with the same type list feature 813 uses for the location type
(`naming::ConventionalType`), so the card's type and its short branch agree. `fix/1115-foo` and
`fix/1115_foo` both read `fix/1115`; `feat/make-branch-selector` (no number) and
`scratch/023-docs-gate` (no conventional type) stay as written (spec Edge Cases).

**Alternatives rejected**:
- `naming::parse_tags`: it reads a *directory* name into tags and drops the separator shape; reusing
  it would accept `feat_813` forms the spec does not name and lose the "both parts present" rule.
- The `regex` crate: a dependency for one 20-line scan (Constitution, Dependencies).

## R4. Sessions line

**Decision**: `sessions_line((total, running))`: `No sessions` for 0; `1 session`; `n sessions`; then
` · k running` when `k > 0`. `running` is clamped to `total` for display (spec Edge Cases: never more
running than sessions). No cap on counts.

**Rationale**: FR-008 and US2.5 verbatim. The type word leaves this line for the header (FR-001).

**Alternatives rejected**: hiding the line at 0 sessions (spec Assumptions: *No sessions* tells the
user activation will offer to start one).

## R5. Progress row

**Decision**: `overview_card::progress_words` moves to core as `overview_text::progress_row(&Progress)
-> Option<ProgressRow>`, `None` when neither phase nor ratio is known (FR-009); the text and the bar
rule are otherwise those of feature 813 FR-016 (*Implement · 6/10*, *6/10*, the phase alone; a bar
only with a ratio).

**Rationale**: Hiding a row is a decision; it belongs in tested core logic. *No task progress* is
removed on purpose (spec US2.6).

**Alternatives rejected**: keeping the empty-state text at a muted colour: the issue names it as a
row with nothing to say.

## R6. Pull request row

**Decision**: `PrDisplay::footer` keeps its signature (`Option<(String, Option<String>)>`) and changes
its rule: `Counts` with both 0 is `None`; with one 0, only the other (`1 open`, `2 merged`, `100+
open`); both non-zero as today. `Hidden` stays `None`; `Unknown` stays the dash with its reason.

**Rationale**: FR-010. Keeping the signature keeps the one caller (`overview_card.rs`) unchanged in
shape; `CardText` carries the result.

**Alternatives rejected**: a new enum for the row: the existing pair already says text plus optional
tooltip.

## R7. Markup sanitiser and slash commands

**Decision**: `micold_core::markup::readable(text: &str) -> Option<String>`:
1. Remove every *tag*: `<`, an optional `/`, an ASCII letter, then every character up to and
   including the next `>`, provided no `<` comes first. `a < b`, `x<3` and an unterminated `<b`
   are not tags and stay as written.
2. Collapse every run of whitespace (newlines included) to one space and trim.
3. Repeat step 1 until no tag is left: removal can join `<` and a following `a>` into a new tag,
   as in `<<b>a>` (which becomes empty) or `Vec<Vec<u8>>` (which reads `Vec`). Return `None` when
   the result is empty (FR-013). (Amended in M2 review: returning `None` whenever a tag was left
   hid the whole line of a Rust prompt such as `Vec<Vec<u8>>`.)

`overview_text::activity_text(text) -> Option<String>` applies `readable`, then, when the result
starts with `/` followed by a command name (ASCII letters, digits, `-`, `_`, `:`, `.`) and then
whitespace or the end, forms *Running name* or *Running name on args*: `speckit-` stripped from the
front of the name, `-` read as a space (FR-012). `/home/u/x` is not a command (the name ends at a
second `/`). The same sentence for every command, built-ins included (Clarifications, Decision 1).
The card's activity line is `activity_text(line)` plus ` · done/total` when the todo list has a ratio;
`None` hides the line, ratio included.

**Rationale**: The spec's examples and edge cases (nested and unbalanced tags, markup inside a slash
command's arguments, `a < b` survives). Sanitising before forming the sentence is what removes markup
from a command's arguments.

**Alternatives rejected**:
- An HTML/XML parser crate (`quick-xml`, `scraper`): prompts are not well-formed documents; an
  unbalanced tag would fail the parse and leave raw text, the defect being fixed; and a dependency.
- Removing only `<agent-message …>`: the next wrapper tag (`<system-reminder>`, `<command-name>`)
  would show raw again.
- Hiding the content between tags: the spec shows the words (`Continue M2`).

## R8. Where the prompt is sanitised: at the hook, and again at the card

**Decision**: `HookActivity::apply` (`crates/micold-core/src/hook_activity.rs`) stops keeping only
the prompt's first non-empty line. It applies `markup::readable` to the whole prompt and then clips
to `MAX_TEXT_CHARS` (200); a prompt with nothing readable sets `prompt` to `None`, so an earlier
prompt does not linger as the line (spec Edge Cases, several sessions). The card applies
`activity_text` again at render time, which is idempotent on sanitised text and also covers any
value from an older service. Todo content and `activeForm` are sanitised and clipped the same way
when the `TodoWrite` list is parsed; a todo with nothing readable is skipped like an empty one, so
it never hides the prompt (amended in M2 review).

**Rationale**: Today `<agent-message from="x">` on its own first line is the whole stored prompt, so
the card could only show markup or nothing, never *Continue M2* (US3.1 with a multi-line wrapper),
and US3.6 asks for a multi-line prompt on one line. Clipping before sanitising could cut a closing
tag in half (`</agent-mes`), which no rule could then tell from text; sanitising first avoids it.

**Alternatives rejected**:
- Sanitise only at the card: the stored first line or a clipped tail cannot be repaired there.
- Store the raw prompt unclipped: unbounded text in the service's memory and on the wire for every
  session summary.

## R9. Card palette: tinted header, neutral body

**Decision**: `overview_card::Palette` becomes `{ header, on_header, body, on_body, on_body_variant }`:
- header: the state's `*_container` / `on_*_container` pair as today; Idle: `surface_variant` /
  `on_surface_variant` (FR-001, FR-003);
- body (every state): `surface_container_low`; the title in `on_surface`; the other body lines in
  `on_surface_variant`.
The card's ripple and focus layer, and the targeted *selected* layer, use `on_surface`, the colour of
the content that now fills most of the card; the targeted layer goes over both the header and the
body fill. The `primary` outline and the `secondary` focus ring are unchanged.

**Rationale**: FR-002's one neutral body; FR-005 keeps Idle de-emphasised by its grey header against
the coloured ones. `surface_container_low` is today's Idle fill, so an Idle card's body looks as it
does now, and it differs from `surface_variant` (Idle's header) in both schemes, so Idle's header
still reads as a band. Contrast is asserted, not assumed (R11).

**Alternatives rejected**:
- Body `surface_container_highest`: equal to `surface_variant` in the Material baseline, so the Idle
  header would vanish into the body.
- Keep the ripple in the state's `on_*_container`: on the neutral body it is a coloured flash on a
  grey surface and is not the content colour feature 971 tied it to.

## R10. Type scale: a card-scoped title role

**Decision**: Add `typography::OVERVIEW_CARD_TITLE = role("overview_card_title", 17.0, 24.0, 500, 0.0)`
in `crates/micold-core/src/tokens/typography.rs`, listed in a new `typography::SCOPED` array (outside
`ALL`, which stays the fifteen Material roles), and `TypeRole::CardTitle` in
`crates/micold-client/src/ui/material/text.rs` resolving to it. The header's state and type words use
`TypeRole::Label` (12/16, weight 500); every body line uses `TypeRole::Body` (14/20); header glyphs
are sized by `TypeRole::Label`. `type_role_mapping.rs`'s `no_role_invents_a_size_outside_the_scale`
admits roles in `typography::SCOPED` by name, so the exception is one declared list.

**Rationale**: FR-004 and spec Assumptions: 17 px is not on the Material scale (16, 22), and the
sidebar's precedent is a named, scoped mapping in one table. Weight 500 makes the title read as the
card's heading at a size only 3 px over the body.

**Alternatives rejected**:
- `TypeRole::Section` (16): misses FR-004's 17 px.
- `TypeRole::Title` (22, today): the size the issue asks to reduce; two lines of it crowd 280 px.
- Captions at 12 px for body lines: FR-004 sets body lines at 14.

## R11. Contrast check

**Decision**: Replace `composition_contrast.rs::an_overview_cards_text_and_icon_are_legible_on_its_state_fill`
with a check over the new `Palette`: for both schemes and the five states, `on_header` on `header`
and `on_body`, `on_body_variant` on `body` at 4.5:1 (header icons at 3:1 at least, and they share
`on_header`, so 4.5:1 covers them); Idle's header is `surface_variant` and differs from `body`. The
target outline check (`the_card_target_outline_is_visible_on_every_state_fill`) runs against both
fills.

**Rationale**: SC-001 (10 state-theme pairs, header and body). The check reads `palette`, the function
the card draws with, as today.

**Alternatives rejected**:
- Sample rendered pixels from a screenshot: needs a display and a renderer in the test, is flaky
  under anti-aliasing, and still has to know which colour is text and which is fill.
- Check the header text only: the body lines are what the user reads most and sit on a different
  fill, so SC-001's body half would go unchecked.
- Keep one combined fill per state (today's check): FR-001 splits header and body fills, so a single
  pair no longer describes the card.

## R12. Height: a minimum, and every card in a row as tall as the tallest

**Amended in M3**: the decision below is carried out inside `CardGrid`'s `Grid::layout`, not by a
`cdk::EvenRow`: feature 970 made the grid one widget placing cards on a ribbon that cuts a card
across rows during a zoom, so a per-row widget cannot sit in it. The measure-then-stretch rule is
the same (contracts K2): measure each card at its width with the minimum as its least height, take
each row's tallest, lay each card out at exactly the tallest of its rows. T023 and T028 are
superseded.

**Decision (as planned)**: A new layout-only widget `cdk::EvenRow` (`crates/micold-client/src/ui/cdk/even_row.rs`):
`EvenRow::new(children, columns).spacing(gap).min_height(h)`. It gives each of `columns` slots the
width `(W − gap·(columns − 1)) / columns`, lays every child out at that width with an unbounded
height to measure it, takes `H = max(h, tallest)`, then lays every child out again with limits
`min = max = (slot, H)`. `CardGrid::rows` builds one `EvenRow` per row with
`min_height(overview_card::MIN_HEIGHT)` (172, today's `HEIGHT`, renamed). The card's face drops
`Length::Fixed(HEIGHT)` for a shrink height that accepts the minimum its limits give it; a missing
slot in the last row stays empty, so its cards keep the full-row width (feature 1024).

**Rationale**: FR-014. iced 0.14's `row` cannot do it: in `core/src/layout/flex.rs` (0.14.0) a child
that fills both axes in a height-shrinking row is laid out in the third pass with `cross` as its
maximum height, and `cross` is 0 when every child fills, so `height(Fill)` cards collapse rather than
match the tallest. Measuring first is what `cdk::Reflow` already does for its own case; the widget
draws nothing, so it belongs in the behaviour layer (`tests/cdk_no_appearance.rs`).

**Alternatives rejected**:
- Compute each card's height from which rows it has (header + lines × line heights): a second copy of
  the layout that drifts from the real one with every padding or role change.
- Keep a fixed, taller height: sparse cards would be mostly empty, and a future row would clip again.
- `height(Fill)` cards in a plain `row` (above): collapses to 0.

## R13. Header row and the outline over it

**Decision**: The face is a column of the header (its own `container`, `header` fill, top corners at
the card radius 12, padding `spacing::SM` × `spacing::MD`) and the body (padding `spacing::MD`), on an
outer container with the `body` fill and radius 12. The target outline is drawn by a top layer of an
iced `stack` (transparent fill, `primary` border, radius 12) so the header's fill cannot paint over
it. The header holds `Reflow`-free `row![state cluster, Space::Fill, type cluster]`; the state
cluster keeps the reason tooltip, the type cluster gets its word as tooltip (US1.4). The `Ripple`,
`TakesTheKeyboard` and `ContextArea` wrappers stay outside the whole face, so the ripple, focus ring
and secondary press cover the header too (FR-016).

**Rationale**: An iced `container` draws its border before its children, so today's single-container
outline would be hidden under the header band.

**Alternatives rejected**: inset the header by the outline width: the header would shift by 2 px
whenever the card is targeted.

## R14. Overflow at 280 / 380 / 480 px and the layout snapshot

**Decision**: Every body line is an `Ellipsized` single line; the title, branch and activity lines
carry their full text as a tooltip, and the pull request unknown mark keeps its reason (FR-015).
Geometry tests in `overview_card.rs` lay out a card with every optional row at 280, 380 and 480 px
and assert no two leaves overlap and all sit inside the card; `card_grid.rs` asserts the equal row
height and the minimum. The layout snapshot (`crates/micold-client/tests/layout_snapshot.rs`) has
no Overview state today; one is added in `tests/support/covered_states.rs` (`overview-open`: the
fixture project with the Overview open), with anchors on the first card's header and title, and the
fixture is regenerated. That state also runs the text-overflow and containment gates
(`docs/development/layout-snapshot.md`, *The three checks*) on the real Overview.

**Rationale**: FR-015, FR-019 and SC-002. The app draws every role in its bundled Roboto
(`text.rs`, `ROBOTO`) on all three platforms and lays out in logical pixels, so these Linux-measured
geometries hold at scale 2.0 and on macOS and Windows (Principle VI); CI runs them on all three.

**Alternatives rejected**: relying on the showcase visual pass alone: it cannot fail a build.

## R15. Showcase and user guide

**Decision**: `showcase/sections/surfaces.rs::card_model` gains a title (an issue-like sentence), the
branch `feat/813_worktree-overview` (shown `feat/813`) and, on one card, the activity
`<agent-message from="orchestrator">Continue M2</agent-message>` (shown *Continue M2*); the five state
cards are shown in a `CardGrid` so the even row height is visible. `docs/user-guide/overview.md`
rewrites *One card per location*, folds *The summary line* into a *Title* paragraph, drops *No task
progress* from *Feature progress*, rewrites *What the agent is doing now* (markup, slash commands,
one line) and *Pull request counts* (hidden at 0 · 0, single count), and adds row height to
*Layout* (FR-019).

**Alternatives rejected**:
- A separate showcase section per state: the even row height (FR-014) only shows when cards of
  different heights share one row, so separate sections could not show it.
- Keep *The summary line* section in the guide: the card no longer has a summary line; the title
  replaces it, so the section would describe something that is not on screen.
