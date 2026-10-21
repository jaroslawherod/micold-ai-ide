# Research: Overview cards, a pull request list with state, linked

Decisions for [plan.md](plan.md). Each: Decision, Rationale, Alternatives considered. Code
positions are at `origin/main` f1d7d1b5.

## R1. The reading already carries every field; keep the nodes it drops

**Decision**: Change nothing in the `pr` fragment (`crates/micold-core/src/pull_request.rs`
`FRAGMENT`): it already asks `number title url state isDraft createdAt isCrossRepository headRefOid
reviewDecision mergeable` and the last commit's check counts by state. `parse_status` today keeps
only the node `select_pull_request` picks and throws the rest away; it now also turns every own
(non-fork) node of the branch's aliases into a `ListedPr` (data-model §1) through the same state,
checks (`reduce_checks`) and review mapping `select_pull_request` uses, moved into one shared
`fn listed(node: &PrNode) -> Result<ListedPr, Unreadable>` so the indicator and the rows never
disagree (FR-013).

**Rationale**: FR-001 asks for these fields "within the readings feature 040 already sends"; they
are already in the answer. An unknown `state` already fails the whole reading as `Passing`
(`Unreadable`); listing more nodes keeps that rule for every listed node.

**Alternatives**: a second query per branch (feature 1003's `list_query`, 100 nodes): a new request
kind, against FR-001 and SC-004. Reusing 1003's `BranchPullRequest`: it has no review or conflict
and carries base and head branch names this feature does not need.

## R2. Counts from the list: a merged page of 10 replaces the two 100-node count pages

**Decision**: Per branch the query keeps `o<i>` (newest 10 open, drafts included) and `r<i>`
(newest 10 of any state), both `{ nodes { ...pr } }` as today, drops `oc<i>` and `mc<i>` (100
nodes each, only `isCrossRepository`), and adds `m<i>`: `pullRequests(headRefName: $b<i>, states:
MERGED, first: 10, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...merged } }` with a
second fragment `fragment merged on PullRequest { number url state isDraft createdAt
isCrossRepository }`: no `commits` connection, no review or conflict field. A merged row shows no
status words (FR-004, `StatusPhrase::of` is `None` for merged), so nothing it would show is lost.
`m` nodes parse through their own `RawMerged` (the `RawNode` fields `m` does not ask stay required
for `o` and `r`) into a `ListedPr` with `checks` none, `review` none and `conflict` false. The
branch's list is the own nodes of `o ∪ r ∪ m`, one entry per number, the `o`/`r` node kept when a
number is also in `m`. The open count is the own nodes of `o`, the merged count the own nodes of
`m`; a count is *capped* (reads *N+*) when its page came back full, 10 nodes, forks included, the
rule `count_own` applies to 100 today. `PrCounts` and `PrDisplay::footer` stay as they are;
`COUNT_PAGE`, `COUNT_PAGE_SIZE`, `COUNT_NODES`, `RawCounted` and `count_own`'s page size go.

**Cost** (FR-008: no more rate-limit points per reading than today). GitHub's documented GraphQL
rate-limit score ("Rate limits and node limits for the GraphQL API", *Calculating a rate limit
score*): add up the requests needed to fulfil each connection of the call, assuming every
connection reaches its `first`/`last` argument, so a connection nested under a page of 10 counts
10 requests; divide the sum by 100 and round; the least a call costs is 1 point. Objects that are
not connections (`repository`, `commit`, `statusCheckRollup`, `rateLimit`) add nothing. Per
branch:

| Alias | Today | Now |
|---|---|---|
| `o<i>` | 1 (`pullRequests`) + 10 (`commits(last: 1)` × 10) + 10 (`contexts(first: 1)` × 10 × 1) = 21 | 21 |
| `r<i>` | 21 | 21 |
| `oc<i>` / `mc<i>` | 1 + 1 | gone |
| `m<i>` | — | 1 (no nested connection) |
| **Branch** | **44** | **43** |

A full request of 50 branches (feature 040 FR-023) sums 2,200 today and 2,150 now, 22 points
either way after rounding; for any number of branches `n` the sum is 43n ≤ 44n, and rounding keeps
the order, so no request costs more than today's. Had `m<i>` used `...pr` it would add 20 per
branch (63, about 32 points per 50 branches), which is why it asks the `merged` fragment. GitHub's
separate node limit (500,000 per call, multiply `first` down the tree) falls from 260 to 70 per
branch, far below the limit either way; it is not what points measure. Still one request per chunk
of up to 50 branches, so requests per hour are unchanged (SC-004).

**Rationale**: D4 (clarify): newest 10, *10+*, no more GitHub cost than today. Every count then
comes from listed pull requests (the issue's "counts derive from the list", FR-006).

**Alternatives**: `m<i>` with `...pr`: one fragment, but 63 requests per branch against today's 44,
breaking FR-008. Keep `oc`/`mc` at 100 and list only `o ∪ r`: counts would not derive from the
list, and merged counts above the list's merged ones would disagree with the rows. Count pages of 11
`isCrossRepository` nodes beside `o ∪ r`: same cost as `m<i>` but again counts not from the list.

**Spec effect**: FR-008 names the three pages (the merged one was implicit in "read enough to tell
a count of 9 or fewer from 10 or more"); FR-006 says a full page with a fork in it reads its own
count with *+*. Both amended in spec.md with this plan.

## R3. Order and the status phrase are core functions

**Decision**: `micold_core::pr_list` (new module, render-free) holds `ListedPr`, `BranchPrList`,
`StatusPhrase` and `PrRow`. `BranchPrList::new` sorts by FR-003: open and draft first, then merged
and closed; within a group `created_at` descending, then the higher number (the comparison
`sort_newest_first` uses for feature 1003). `StatusPhrase::of(&ListedPr) -> Option<StatusPhrase>`
returns, for open and draft only, the first of `ChecksFailing`, `ChangesRequested`,
`MergeConflict`, `ChecksPending`, `Approved`, `ChecksPassing` that applies (D3), with `label()`
giving FR-004's words. `BranchPrList::rows(expanded: bool) -> (Vec<PrRow>, usize)` returns the
rows to draw (2 collapsed, all expanded) and the "+N more" count (0 expanded).

**Rationale**: Principle I and FR-016: every rule (order, phrase, more-count, counts) is unit-tested
without the GUI. The first-three order is `card_state`'s attention order
(`crates/micold-core/src/overview.rs` ~l.328), so a card that says *checks failing* has a first row
that says it too.

**Alternatives**: computing the phrase in the client view: untestable without iced, against FR-016.

## R4. Where the list lives: it replaces the counts map end to end

**Decision**: `StatusReading.counts: BTreeMap<String, PrCounts>` becomes `lists: BTreeMap<String,
BranchPrList>`, one entry for every asked branch (an empty list for "no pull request", as the 0/0
counts are today). The same rename runs through the client: `shell/pr_status.rs`
(`Msg::StatusesRead`), `features/pr_status.rs` (`Outcome::Ok`, `State.counts` → `State.lists`,
every `clear()` site), `shell/daemon_sync.rs` (`MergedBranchCheck`). `PrCounts` is derived by
`BranchPrList::counts()` where `ui/overview.rs::pr_display` reads it today.

**Rationale**: one source for rows and counts (FR-006, SC-005); the clear sites already encode
FR-007's keep/drop rules (`Passing` and `RateLimited` keep, `Unavailable` and `Released` clear), so
the rows follow them with no new code (spec Story 1 sc 4 and 8).

**Alternatives**: a second map beside `counts`: two maps cleared in lockstep, the invariant the
existing code already had to document ("cleared exactly when `statuses` is").

## R5. The card model carries the rows

**Decision**: `CardModel` (core `overview.rs`) gains `pull_requests: Option<BranchPrList>`, set by
`ui/overview.rs` from `State.lists` when `pr_display` returns `Counts`, `None` otherwise (dash,
hidden). `PrDisplay::Counts` keeps carrying `PrCounts` (from `counts()`), so `footer()` is
unchanged.

**Rationale**: the card is derived per render (feature 813); `BranchPrList` holds at most 30
entries, cloned per render as `CardModel` already clones strings.

**Alternatives**: the rows inside `PrDisplay::Counts`: one value for counts and rows, but the
variant feeds `footer()` and the sidebar-facing tests of feature 813, which would all change for
data they do not read. The view reading `State.lists` itself: the card would no longer be a pure
function of `CardModel`, which the layout snapshot and the showcase build without a `State`.

## R6. Many links per card: a link slot, and focus by the last gain

**Decision, names**: A card's links are named by a core enum `CardLink { Issue, PullRequest(u64) }`
(`overview_expansion.rs`). The expansion events that name a link carry it:
`LinkFocusGained(SessionLocation, CardLink)`, `LinkFocusLost(SessionLocation, CardLink)`;
`LinkRested(SessionLocation)` stays (a rest on any link of a card has the same effect, 1122 FR-007).
`focused_link()` returns `Option<(&SessionLocation, CardLink)>`.

**Decision, the focus model** (`Expansion::gained`/`lost`, positions at f1d7d1b5). Today a card has
two keyboard parts, and the two reports of a move between them pair through `FocusPart::other()`
(Card ↔ Link) with `FocusPart::Both` between a gain and the matching loss (l.322, l.335, l.356).
That pairing cannot hold three or more parts: a loss of `Link(#1)` then a gain of `Link(#2)` does
not match (`other()` of a link is `Card`), so the card collapses and re-expands, also after Escape;
and a gain of `#2` before the loss of `#1` ends in `Card` (`Link(#1).other()`), so `focused_link()`
is `None` while `#2` holds the keyboard. The new model:

- `enum FocusPart { Card, Link(CardLink) }` (default `Card`); no `Both`, no `other()`.
  `focus_part` is **the part that reported the last gain** on the focused card.
- `gained(location, part)`: (1) `link_held` first, unchanged except that it holds
  `(SessionLocation, CardLink)`: a gain on the held location is the return from the menu, to the
  link or the card, and sets `focus_part = part`; any other gain applies the deferred loss first
  (`lost_plain`). (2) `just_lost` pairing: a loss on `location` of **any part other than `part`**,
  as the event just before, is one move inside the card: restore `before`, `focus_part = part`
  (was: `lost.part == part.other()`). (3) The card is already focused: `part == focus_part` is a
  repeated report (U6's hand-off), nothing changes; any other part is a move inside the card,
  gain first: `focus_part = part`, no form change (was: `Both`). (4) Otherwise a new focus, as
  today (open-focus, menu hold, `Cause::Focus` expansion).
- `lost(location, part)`: the card is focused and `part != focus_part`: a later part already
  reported its gain, so this is the old part's half of a move inside the card: nothing changes
  (was: `focus_part == Both || focus_part == part.other()`). With `part == focus_part`: a link part
  while the menu is open is held, `link_held = Some((location, link))` (FR-009); otherwise
  `just_lost` is recorded and `lost_plain` runs, as today.
- `focused_link()`: `Some((focused, link))` when `focus_part` is `Link(link)` and nothing is held.
- `Listed`, `WindowBlurred`, `MenuClosed`, `Escape` keep their 1122 M4 rules; `link_held` is
  compared by its location for "gone".

Every 1122 rule survives with Card and `Link(Issue)` as the only two parts: the loss-then-gain pair
is the same pair; gain-then-loss ends in the part that gained, which is what `Both` then
`other()` produced; a repeated report changes nothing; a held link returns to the link or card;
`Listed` with no change keeps the pairing (1122 review A r1 F1); `WindowBlurred` applies the held
loss (1122 r1 F3; the card's later gain now sets `Card` instead of `Both`, which is the state F3
wanted). One order changes meaning: under `Both`, a loss of the part that gained last, before the
older part's loss, was a move to the older part; now it is a loss. iced reports a move as one gain
and one loss of two different widgets, so that order is a real departure from the card followed by
a stale report, and a loss is the right reading. K5 (contracts/card-rows.md) keeps 1122's K5
cases and adds the link-to-link cases in both report orders, with and without Escape.

**Decision, the client** (`main.rs`, `features/overview.rs`, `ui/overview.rs`): every hand-off
that knows only "the issue link of a location" takes the link:

- `ui/overview.rs`: `card_link_id(location, CardLink)`; a new `card_link_shown(state, location,
  CardLink) -> bool`: the issue link when `issue_link_for` is `Some`, a number when the card is
  listed and its row is drawn at the card's current form (`BranchPrList::rows(expanded)` contains
  it, with `expanded` read from `Expansion::expanded()` exactly as the card view reads it).
- `main.rs`: `shown_focused_link(app) -> Option<(SessionLocation, CardLink)>`, filtered by
  `card_link_shown`; `LinkFocus.link` holds that pair. `link_focus_handoff` takes the pair and hands
  off to the card when `card_link_shown` is false after the update (was: `issue_link_for` is
  `None`): Escape on a focused row-3 number collapses the card, the number is no longer drawn, and
  the card takes the keyboard with `FocusGained(location)` then `LinkFocusLost(location, link)`, a
  move inside the card (no form change). The same covers a reading that drops the focused pull
  request, or moves it below the second row of a collapsed card. `link_focus_after` re-focuses
  `card_link_id(location, link)` when the card changed form and the link is still shown.
- Menu: `Msg::LinkMenuRequested(SessionLocation, CardLink)` and
  `MenuOrigin::KeyboardFromLink(CardLink)`, so `FocusReturn.to_link` becomes `Option<CardLink>`
  (`Some(link)` when a menu opened from that link closed without a choice). `card_focus_return_id`
  returns `card_link_id(&target, link)` when `card_link_shown`, else the card (was:
  `card_link_id(&target)` when `issue_link_for` is `Some`).

Client view: `OverviewCard` takes `.pull_requests(rows, more)` with `.on_pr_link(Fn(u64) -> M)`,
`.on_pr_link_keyboard_focus(Fn(u64, bool) -> M)`, `.on_pr_link_menu(Fn(u64) -> M)` and
`.pr_link_id(Fn(u64) -> Id)`; each number is a `TextLink` (feature 1122) with the same menu-key and
page wiring as the issue link (`fn issue_link`, overview_card.rs ~l.522), factored into one helper
both use. `CardGrid`'s per-card `link_id: Option<Id>` becomes `link_ids: Vec<Id>`, `over_link`
true on any of them; the arrow step (`step_from`) maps every link id of a card to that card.

**Rationale**: FR-009 asks every 1122 link rule to hold for each number "in the same way": the
same `TextLink`, the same grid path, the same expansion events. Naming the slot keeps
`focused_link`, the hand-off and the menu return exact with several links; "last gain wins"
is the one pairing rule that does not depend on how many parts a card has.

**Alternatives**: keep `Both` and `other()` and give links a common "some link" part: a move
between two numbers would pair, but `focused_link()` could not say which number holds the keyboard,
so the menu return and the hand-off would be wrong. A second, parallel set of PR-link events:
duplicates 1122's rules (and their tests) per link kind. Indexing links by position: a reading that
reorders rows would move focus to another pull request; the number is stable. A set of parts that
currently hold focus (gain inserts, loss removes): exact, but needs a rule for a set left non-empty
by a lost report; the last gain needs none.

## R7. Tab stops after Escape follow what is drawn

**Decision**: A number is a Tab stop exactly when its `TextLink` is in the widget tree; the
collapsed card builds only its first two rows, so after Escape collapses a focused card the hidden
numbers are no stops (FR-010) with no extra rule. Tab order is tree order: branch line (issue link),
then rows top to bottom.

When a drawn number stops being drawn while it holds the keyboard (Escape on a row-3 number, a
reading that drops it), the client hands the keyboard to the card (R6, `card_link_shown`).

**Rationale**: iced's focus traversal follows the tree; 1122 already relies on it for the issue
link.

**Alternatives**: draw every row and mark the hidden ones not focusable (`TextLink::focusable`):
an explicit flag per number, a second source of "is it a stop" beside "is it drawn", and hidden
rows would still take layout. Draw all numbers and skip hidden ones in the grid's focus
operation: a custom traversal rule against iced's tree order, which 1122 avoided.

## R8. Opening a pull request: the address is looked up, not carried

**Decision**: `features::overview::Msg::PullRequestLinkActivated(SessionLocation, u64)`, the card
and the number. `main.rs` looks the address up in what the window holds now, through a new
`ui/overview.rs::pull_request_link_for(state, location, number) -> Option<String>` (the branch of
the location, its `State.lists` entry, the listed pull request with that number), and passes it to
the link opener as `IssueLinkActivated` does with `issue_link_for` (feature 031 opener, its FR-015
notification on failure). A pull request no longer listed, or a card no longer there, opens
nothing. The message carries no address, so its derived `Debug` prints none (FR-014).

**Rationale**: FR-009, FR-012, FR-014; one opener path, and the same "recomputed from what the
window holds now" rule as the issue link (main.rs ~l.885).

**Alternatives**: carry the address, `PullRequestLinkActivated(String)` behind a redacting newtype:
a link drawn from an older reading would still open after the reading dropped it, unlike the issue
link, and the address would sit in a message queue. Carry a row index: a reading that reorders rows
between draw and press would open another pull request.

## R9. Row layout and colours

**Decision**: A row is `[TextLink "#N"] [pill] [Ellipsized status words]` on one line, the number
and pill at natural width, the words taking the rest and ellipsized first (FR-015, 1121 FR-007).
The pill is the shared `Tag` (`ui/material/tag.rs`, `Tag::new(label, accent)`, the tonal pill
`StatusChip` is built on) with the state word as its label and `PrMark::role(roles)` as its accent:
`PrMark` (`pull_request_indicator.rs`) already maps open to `primary`, draft and closed to
`on_surface_variant`, merged to `tertiary` for the sidebar's state glyph, and the contrast gate
measures that function. `BranchPrState` maps to `PrMark` in `ui/overview.rs`, as the sidebar maps
`PrState` (`ui/sidebar.rs` `indicator_marks`; `ui::material` does not depend on core pull request
types). A new `PrMark::word()`
(*Open*, *Draft*, *Merged*, *Closed*) gives the label, so the word and the tint come from one
enum. The pill has no tooltip: its accessible text is its word, which it shows, and the colour only
repeats it (as `StatusChip`'s rule "the group is never carried by colour alone"). Failing words
take the `error` role, the rest the body role. Rows sit between the counts line and the card's
activity line; the "+N more" line is a muted body-role text below the rows. Contrast is covered by
`composition_contrast.rs`: it measures the `PrMark::role` text colour over the `Tag` chip fill
(`style::chip(accent)`, the accent's own faint tint), for each of the four marks in both themes
(feature 1120 FR-003).

**Rationale**: Principle VIII (shared components, roles); SC-006 (number and pill never clipped);
the card's pill and the sidebar's glyph colour a state the same way.

**Alternatives** (Principle VIII):
- `StatusChip`: its only constructor takes a Jira `&TicketStatus`, and its tint and tooltip come
  from Jira status categories; a pull request state would need a fake ticket status. A new
  `StatusChip` constructor taking a word and a role would only wrap `Tag` and a tooltip repeating
  the word.
- `PullRequestRow` (feature 775): a fixed 36 px row with the state and check glyphs, number and
  title, for the Changes pane's list; it shows the title (out of scope here), has no link and no
  status words, and its height does not fit the card's rows.
- `PullRequestIndicator` / `PrMark` glyphs in place of the word: reused for the colour, but the
  spec asks for a state pill with a word (FR-002), and a glyph alone needs a tooltip to be read.
- A new pill widget: `Tag` already draws a word on a tonal fill.

## R10. Docs, snapshot, showcase

**Decision**: `docs/user-guide/overview.md` "## Pull request counts" becomes "## Pull requests"
(rows, order, phrase precedence, "+N more", *10+*, opening by pointer and keyboard), and
"## Keyboard" lists the number stops. New covered state `overview-pr-rows` in
`crates/micold-client/tests/support/covered_states.rs` (a card with three pull requests, collapsed
and the expanded twin). Showcase `showcase/sections/surfaces.rs` gains a card with rows, collapsed
and expanded (FR-015).

**Alternatives**: a new user-guide page for pull request rows: the counts section they replace is
in `overview.md`, and a reader looks there. Extending covered state `overview-open` instead of a
new one: its snapshot would change for every card, hiding what rows changed.
