# Research: Reporter, Labels and a Description Tooltip in the Issue List

**Feature**: [spec.md](./spec.md) · **Date**: 2026-10-02

Each entry: the decision, why, what was rejected, and the evidence in this repository. Paths are
relative to the repository root. "NEW" marks what does not exist yet.

## Where the code is today (read before the decisions)

| Thing | Where | Fact |
|---|---|---|
| `Issue` | `crates/micold-core/src/github.rs:325` | Fields `number, title, labels, updated_at, row_text`; derives `Debug, Clone, PartialEq, Eq`; no `Serialize`. `Issue::new(number, title, labels, updated_at)` builds `row_text = "#N title"` + `"  ·  " + labels.join(", ")`. |
| Queries | `github.rs` `LIST_QUERY` (:409), `SEARCH_QUERY` (:517), `SEARCH_WITH_NUMBER_QUERY` (:523), `issue_from_node` (:450) | Nodes carry `number title updatedAt [state] labels(first: 20){nodes{name}}`. `gh` runs in the client (`shell/capabilities.rs:164`); nothing crosses the daemon protocol. |
| Matching | `crates/micold-client/src/features/worktree_form.rs:439` | `rank(&held, \|issue\| issue.row_text(), &query)` from `micold_core::typeahead`; `Match.spans` are byte ranges of `row_text`. Searched issues are ranked by the same call. |
| Row | `crates/micold-client/src/ui/material/picker.rs` `Row { label, spans, enabled }`, `row_element`, `menu_element`, `EmphasisedLabel` | One line, fixed `density::MENU_ITEM_BASE` height, truncated around the emphasis (`fit_around`). Shared by `Typeahead` (branch picker, issue picker) and `Select`. |
| Issue picker view | `crates/micold-client/src/ui/worktree_form.rs:316` `issue_picker` | `TypeaheadRow::new(issue.row_text(), matched.spans)`; placeholder `"Search by number, title or label"`. |
| Floating list | `crates/micold-client/src/ui/cdk/picker.rs` `Menu` (an `overlay::Overlay`) | Does **not** implement `Overlay::overlay`, so a widget inside the list cannot float an overlay of its own. |
| Tooltip | `crates/micold-client/src/ui/cdk/tooltip.rs` (behaviour), `ui/material/mod.rs:234` (appearance) | Opens at once while the cursor is over the trigger; flips side instead of covering the trigger (`place`); `TOOLTIP_MAX_WIDTH = 320.0`; label is `Caption` with `Wrapping::WordOrGlyph`. Twenty call sites. |
| Frame requests | `crates/micold-client/tests/idle_requests_no_frames.rs`, `ui/cdk/motion.rs:276` | A gate allows exactly one `request_redraw` in the rendering layer, inside `Progress`, behind `animating()`. |
| Keyboard highlight | `features/worktree_form.rs` `issue_highlight_moved`; `main.rs:677` routes issue messages to `shell/issues.rs` | Nothing scrolls the highlighted row into view: `grep -n -i scroll` finds no scroll call in `material/picker.rs`, `material/typeahead.rs` or `cdk/picker.rs`. |
| Scroll-into-view precedent | `crates/micold-client/src/ui/focus.rs:93` `scroll_focused_into_view`, `delta_into_view` (:172); issued at `main.rs:611` | A two-pass `Operation` (find the rectangle, then scroll what hides it), chained as a `Task` after the message that moved focus. |
| Spec 036 | worktree `fix-issue-430`, `specs/036-tooltip-follow-cursor-delay/` | Phase `1-spec` with an open escalation; the spec directory is untracked; no PR exists; issue #430 is open (checked 2026-10-02). |

## R1 — Reporter and description come with the issue (FR-002, FR-024, FR-026)

**Decision**: Add two fields to the node selection of all three existing queries: `author { login }`
and `bodyText`. No new request, no new occasion, no new argument.

**Rationale**: FR-024 and FR-013 forbid a request per hover or per author. The fields ride the
requests 034 already makes. `author` is `null` for a deleted account; the row then shows `ghost`
(FR-002). A bot's `login` is shown as reported.

**Alternatives rejected**: a second request for bodies on hover (FR-024); `gh issue list --json`
(034 R1 chose `gh api graphql`, and the listing would need a second code path).

## R2 — Plain text is GitHub's own `bodyText`, folded (FR-022, D4)

**Decision**: Ask for `bodyText` (GitHub's rendering of the body to text), not `body`. The core
function `description_from(body_text: &str) -> String` (NEW, `github.rs`) folds every run of
whitespace into one space, trims, and applies the cap of R3.

**Rationale**: `bodyText` is the body with Markdown already rendered away by GitHub: heading marks,
emphasis marks, list and checkbox markers and code fences are gone, a link is its text, HTML comments
are absent, and characters that are not markers (`#12`, a `*` in a formula) stay. That is FR-022 as
GitHub itself reads the body, with no parser of ours and no new crate. Measured on this repository's
issue #518 (`gh api graphql`, 2026-10-02): `body` starts `"## Problem\n\nThe issue picker … one line:
`#<number> <title>  ·  <labels>`. …"`; `bodyText` starts `"Problem\nThe issue picker … one line:
#<number> <title>  ·  <labels>. …"`. Folding the newline gives story 3 scenario 12's shape.

**Alternatives rejected**:
- `pulldown-cmark` and walking its events: a new dependency to vet (Constitution, Dependencies) for
  what one GraphQL field already returns, and its rendering can differ from GitHub's.
- A hand-written marker stripper: decision logic with many edge cases (nested emphasis, reference
  links, setext headings, a `*` that is not emphasis) that would be wrong in ways GitHub's is not.
- `bodyHTML` and stripping tags: larger payload, and an HTML parser.

**What stays ours to test**: `description_from` (folding, trimming, cap, empty result). What
GitHub renders is held by a captured fixture of a real node and by quickstart §B.

## R3 — The description is cut in core to 600 characters (FR-025, SC-005, SC-008)

**Decision**: `DESCRIPTION_MAX_CHARS = 600` (NEW). A folded text longer than that is cut at a
character boundary, trailing space trimmed, and `…` appended. `Issue` holds only the result.

**Rationale**: A body can be 65,536 characters; 1,000 held issues would hold up to 65 MB of text
that is never shown. Three lines at 320 px of `Caption` text hold about 160 average characters and
about 330 of the narrowest glyphs, so 600 always overflows three lines before the cap matters; the
layout-time clamp (R11) then decides the visible cut. When the cap does cut, the text already ends
in `…`, so "cut text ends with an ellipsis" (FR-021) holds even if it then fits.

**Alternative rejected**: holding the whole body and cutting only at layout (memory, and a binary
search over 65 k characters per open).

## R4 — One match text, mapped onto two lines (FR-001–003, FR-009, FR-010, FR-014)

**Decision**: `Issue` keeps one match text (`row_text`, what `rank` reads) and gains two display
lines and a mapping between them (all NEW, `github.rs`):

- `title_line()` = `#<number> <title>`; `details_line()` = `<reporter>` + (`"  ·  "` +
  `labels.join(", ")` when there are labels).
- `emphasis(&self, spans: &[Range<usize>]) -> RowEmphasis { title: Vec<Range<usize>>, details:
  Vec<Range<usize>> }`: each span of the match text is clipped to the parts that are shown and
  rebased to the line that shows them; a span crossing a separator is split; separator bytes carry
  no emphasis.
- M1 and M2 (US1): match text is unchanged from today (`#N title` + `"  ·  "` + labels); the parts
  are title → title line, labels → details line after the reporter. From M3 (US2): the match text
  becomes `#N title` + `"  ·  "` + reporter + (`"  ·  "` + labels), and the reporter becomes a
  mapped part. The description is never part of the match text (FR-014).

**Rationale**: The matcher, its tiers, its case rule and its budget test stay untouched, so the
reporter matches "under the same rule" by construction (FR-009), and a searched issue is filtered
by the same call (FR-012). The mapping is pure and tested in core.

**Alternatives rejected**: ranking each line separately and merging (two matches per row, a new
ordering rule, and the 1,000-row budget doubles); matching on a second key for the reporter only
(a second rule that can drift from the first).

## R5 — The two-line wrapping row is a mode of the shared picker row (FR-004, FR-005, FR-028, FR-029)

**Decision**: `material::picker::Row` gains `.details(text, spans)` (NEW, chainable). A row with
details renders as a column — the label at `TypeRole::Body` in `on_surface`, the details at
`TypeRole::Caption` in `on_surface_variant` — both wrapping with `Wrapping::WordOrGlyph`, the row's
height `Shrink` with `density::MENU_ITEM_BASE` as its minimum and `spacing::XS` vertical padding,
the picked-row marker aligned to the first line. Emphasis keeps today's two channels (accent colour
and bold). A row without details is built by today's code path, unchanged.

The wrapping label is a wrapping mode of `EmphasisedLabel`, shaped as one paragraph with
`Paragraph::with_spans` (iced_core 0.14, `text/paragraph.rs:17`), because today's implementation
lays separate paragraphs side by side and cannot wrap.

**Rationale**: FR-028 puts the row in the component library; FR-029 and the `Select` need the
single-line row untouched, and `picker_parity.rs` / `menu_anatomy.rs` hold it. `WordOrGlyph` is
what breaks a word wider than the row (FR-004), as the tooltip label already does.

**Alternatives rejected**: a second row component for issues (Principle VIII); `iced::widget::rich_text`
at the call site (appearance outside `material/`, against `material_boundary.rs`); truncating the
details line (FR-004).

The list's height cap stays `MENU_ITEM_BASE × 8` in pixels: taller rows mean fewer visible, and the
list scrolls.

## R6 — Keeping the highlighted row wholly visible is new work (FR-007, SC-007)

**Finding**: The spec calls this existing behaviour. It is not built: no picker scrolls its list
when the highlight moves (table above). With one-line rows the defect is already there for a list
longer than eight rows. The requirement stands, so this feature builds it.

**Decision**: An operation `picker_highlight_into_view()` (NEW, `crates/micold-client/src/ui/`,
beside `focus.rs`), modelled on `scroll_focused_into_view`: pass one finds the rectangle of the row
the shared list marks as highlighted (a widget `Id` set by `menu_element` on that row only), pass
two scrolls the enclosing scrollable by `delta_into_view` (reused, already unit-tested; made `pub(super)`). The shell
chains it after `IssueHighlightMoved` where it routes the issue messages (`main.rs:677`,
`shell/issues.rs`). It reads laid-out rectangles, so rows of any height work.

**Alternatives rejected**: computing an offset in the reducer (it cannot know wrapped heights);
scrolling from inside the list widget on every frame (a frame request outside `Progress`).

**Scope**: only the issue picker chains it. The branch picker and `Select` have the same defect
from features 021 and 022; wiring them is outside this spec (FR-029) and is recorded as a follow-up
in the ledger.

## R7 — The rest delay is built here, as its own tooltip mode, not on 036 (D2, FR-015, FR-016, FR-028)

**Decision**: Build the rest delay in this feature, in the shared tooltip:
`material::Tooltip::after_rest(Duration)` (NEW, chainable) over a rest mode of `cdk::tooltip`. It
does not wait for or build on spec 036.

**Rationale**: 036 is at phase `1-spec` with an open escalation, its spec is uncommitted, and no PR
exists; waiting would block story 3 on another flow with no date. Its delay counts from pointer
entry, which cannot express "restarts on movement" (FR-016), so even merged it would not be the
mechanism. The two are separate builder methods on the same component and can coexist; whichever
flow merges second rebases over the other's change to `cdk/tooltip.rs`.

**Alternatives rejected**: wait for 036 (blocked, and the wrong semantics); a feature-local hover
timer in the form reducer fed by `on_move` messages (every cursor move would run `update` and
rebuild the view, and the behaviour would live outside the component library, against FR-028).

**The rule is pure and lives in core** (Principle I): `micold_core::tooltip::RestTimer` (NEW
module). Inputs are observations `(cursor position over the trigger or None, now: Instant)` and a
press; the output is `open: bool` and, while waiting, the `Instant` to wake at.

| State | Observation | Next |
|---|---|---|
| `Away` | cursor over the trigger at `p` | `Waiting { anchor: p, since: now }` |
| `Waiting` | cursor farther than the tolerance from `anchor` | `Waiting { anchor: p, since: now }` |
| `Waiting` | within tolerance, `now - since < delay` | unchanged; wake at `since + delay` |
| `Waiting` | within tolerance, `now - since >= delay` | `Open` |
| `Open` | cursor over the trigger, anywhere | `Open` (spec Assumptions) |
| any | cursor not over the trigger | `Away` |
| any | press on the trigger | `Spent`: closed until the cursor leaves (FR-017) |

Each row is its own trigger with its own widget state, so "another row always restarts" holds by
construction.

## R8 — Waking once at the deadline (FR-018)

**Decision**: A second, separately guarded door in `cdk/motion.rs`: `wake_at(shell, Instant)`
(NEW), which calls `Shell::request_redraw_at(RedrawRequest::At(instant))` (iced_core 0.14,
`shell.rs:71`). The tooltip calls it only while its `RestTimer` is `Waiting`, with the timer's
deadline. `idle_requests_no_frames.rs` is extended: still no frame request outside `motion.rs`;
inside it, exactly the `animating()`-guarded `request_redraw` and the one `request_redraw_at` in
`wake_at`; and a behavioural half proving a waiting tooltip asks for one timed wake at its deadline and
an open, away or spent one asks for none.

**Rationale**: A still cursor produces no events, so without a timed wake the tooltip would open
only when something else redraws. A redraw at one instant is the opposite of a held-awake loop:
the runtime sleeps until then.

**Alternatives rejected**: an `iced::time::every` subscription while any list is open (ticks
whether or not a cursor rests, and couples the component to the app's subscriptions);
`Task::perform(sleep)` from the reducer (R7's rejected reducer route).

**Time source**: `Event::Window(RedrawRequested(instant))` carries its instant; a mouse event does
not, so the glue reads `Instant::now()` for it. The rule itself takes `now` as an argument.

## R9 — Rest tolerance: 4 logical pixels (FR-016)

**Decision**: `REST_TOLERANCE = 4.0` logical pixels, as a distance from the anchor.

**Rationale**: The size Windows uses for its own hover rectangle (`SM_CXMOUSEHOVER`, 4 px). Large
enough for a hand resting on a mouse, small against a row at least 48 px tall.

**Alternatives rejected**: 0 (a resting hand's jitter would never open it); 8 or more (slow
deliberate movement across a row would count as rest).

## R10 — A tooltip inside the floating list needs nested overlays

**Decision**: `cdk::picker::Menu` implements `Overlay::overlay` (iced_core 0.14, `overlay.rs:85`),
forwarding to its content's `Widget::overlay` with the list's bounds as viewport.

**Rationale**: The rows live in the picker's own overlay. Without forwarding, a row's tooltip panel
is never produced. The wrappers between (`fade`, `scale`, `container`, `Scrollable`, `column`)
already forward `overlay` (`material/animation.rs:550, 946, 1130`).

**Check**: a geometry gate drives a real hover over a picker row and reads the panel against the
row (R15), which fails if any layer drops the overlay or its translation.

## R11 — At most three lines, with an ellipsis (FR-021, SC-005)

**Decision**: `micold_core::tooltip::clamp_to_lines(text, max_lines, lines_of: impl Fn(&str) ->
usize) -> Cow<str>` (NEW, pure): the text itself when it fits; otherwise the longest prefix, cut at
a character boundary and backed up to a word boundary when one is near, with trailing space trimmed
and `…` appended, that `lines_of` reports as at most `max_lines`. A trailing `…` from R3 is not
doubled. `material::Tooltip::max_lines(usize)` (NEW, chainable) measures with the renderer at
layout time (paragraph height ÷ line height at `TOOLTIP_MAX_WIDTH` less the panel padding) and
caches the result per text and width.

**Rationale**: The same split as `typeahead::fit_around` and `Ellipsized`: the decision is pure and
takes a measuring closure; the widget supplies the measurement.

**Alternative rejected**: cutting by a character count (three lines of `W` and three of `i` differ
by a factor of four).

## R12 — What a tooltip belongs to, and when it closes (FR-017)

**Decision**: `material::Tooltip::subject(u64)` (NEW, chainable): the identity of what the trigger
describes. Widget state is kept by position in the tree, so when the list narrows a different issue
arrives at the same position; a changed subject resets the timer to `Away`. The picker row carries
it as `Row::key(u64)`; the issue picker passes the issue number. A press closes it (`Spent`, R7).
Scrolling and list changes move rows under a still cursor; the next redraw observes the cursor
against the new bounds, so the old row goes `Away` and the new one starts `Waiting`.

A row gets a tooltip through `Row::tooltip(text)` (NEW); `menu_element` wraps such a row in
`Tooltip::new(row, text, roles).after_rest(ROW_TOOLTIP_REST).max_lines(ROW_TOOLTIP_LINES)
.subject(key)`, with `ROW_TOOLTIP_REST = 3 s` and `ROW_TOOLTIP_LINES = 3` named in
`material/picker.rs`. An issue with an empty description passes no tooltip text (FR-020).

## R13 — Reporters and descriptions stay out of files and logs (FR-025)

**Decision**: `Issue` stays without `Serialize` (held by an existing test) and gets a hand-written
`Debug` that prints the reporter and the description as `<redacted>`. No logging statement names
either; none exists today (`grep` finds no `tracing`/`log` macro over issues in `github.rs` or
`shell/issues.rs`).

**Rationale**: `{:?}` is the way a value reaches a log by accident; a redacting `Debug` makes that
harmless by type. `IssueLoadError::Other` carries only `serde_json`'s error text or a fixed
sentence (`parse_list_page`), never payload; a test holds that for a malformed page that contains a
body.

## R14 — Load time (SC-008)

**Decision**: Accept the larger answer; measure in quickstart §B against the same repository before
and after. No server-side truncation of `bodyText` exists.

**Evidence**: this repository, 25 open issues, three runs each on 2026-10-02: 4.5 KB without the
new fields (523, 1851, 600 ms) and 39 KB with them (1397, 757, 969 ms). The spread between runs is
larger than the difference, so the ratio must be measured on a 1,000-issue repository, as SC-008
says. `run_bounded` already drains pipes while it waits (034 R6), so a large page cannot stall.

**Risk kept**: a repository whose issues all have very long bodies makes each page of 100 large.
If §B measures more than 1.5×, no fallback is decided in advance: SC-008 then conflicts with
FR-024 ("the description arrives with the issue"), which is a question for the user (escalation,
category 1), asked with the measurement.

**Alternative rejected**: a smaller page size for the listing. It multiplies the round trips, which
cost more than the bytes, and FR-026 keeps the requests as they are.

## R15 — Which side the row tooltip opens on (FR-023)

**Decision**: `TooltipPosition::Bottom`, the component's default.

**Rationale**: The panel is at most three lines tall, so there is room below or above a row in any
window; `place` flips to the top when below has no room and never covers the trigger while either
side fits. Left or right would need 320 px beside a list as wide as its dialog, which a narrow
window does not have, and the fallback there slides over the row. The panel takes no input, so a
click on the row still picks it. A new geometry gate beside `tests/gates/tooltip_clears_its_row.rs`
holds a picker row against its panel for a first row, a last row and a row at the list's lower
edge.
