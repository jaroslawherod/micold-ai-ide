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
(newest 10 of any state), drops `oc<i>` and `mc<i>` (100 nodes each, only `isCrossRepository`),
and adds `m<i>`: `pullRequests(headRefName: $b<i>, states: MERGED, first: 10, orderBy: {field:
CREATED_AT, direction: DESC}) { nodes { ...pr } }`. The branch's list is the own nodes of
`o ∪ r ∪ m`, one entry per number. The open count is the own nodes of `o`, the merged count the own
nodes of `m`; a count is *capped* (reads *N+*) when its page came back full, 10 nodes, forks
included, the rule `count_own` applies to 100 today. `PrCounts` and `PrDisplay::footer` stay as
they are; `COUNT_PAGE`, `COUNT_PAGE_SIZE`, `COUNT_NODES`, `RawCounted` and `count_own`'s page size
go.

**Rationale**: D4 (clarify): newest 10, *10+*, no more GitHub cost than today. Every count then
comes from listed pull requests (the issue's "counts derive from the list", FR-006). GitHub prices a
query by the nodes it may return: today per branch `o` and `r` are 30 each (10 pull requests, 10
commits, 10 rollups) and `oc` and `mc` 100 each, 260; now `o`, `r` and `m` are 30 each, 90. Still
one request per chunk of up to 50 branches (feature 040 FR-023), so requests per hour are unchanged
(SC-004) and each request costs about a third of today's points.

**Alternatives**: keep `oc`/`mc` at 100 and list only `o ∪ r`: counts would not derive from the
list, and merged counts above the list's merged ones would disagree with the rows. Count pages of 11
`isCrossRepository` nodes beside `o ∪ r`: cheaper still but again counts not from the list.

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

## R6. Many links per card: a link slot, not a link

**Decision**: A card's links are named by a core enum `CardLink { Issue, PullRequest(u64) }`
(`overview_expansion.rs`). The expansion events that name a link carry it:
`LinkFocusGained(SessionLocation, CardLink)`, `LinkFocusLost(SessionLocation, CardLink)`;
`LinkRested(SessionLocation)` stays (a rest on any link of a card has the same effect, 1122 FR-007).
`FocusPart::Link` becomes `FocusPart::Link(CardLink)`, so `focused_link()` returns
`(&SessionLocation, CardLink)` and the menu's focus return (1122 FR-009) goes back to the link that
had it. A Tab between two links of one card is a loss and a gain on the same card, which the
existing "loss then gain on its own card" path (`just_lost`) already treats as no move: no collapse,
no re-expansion of an Escape-collapsed card (FR-010).

Client: `ui/overview.rs::card_link_id(location)` becomes `card_link_id(location, CardLink)`;
`OverviewCard` takes `.pull_requests(rows, more)` with `.on_pr_link(Fn(u64) -> M)`,
`.on_pr_link_keyboard_focus(Fn(u64, bool) -> M)` and `.pr_link_id(Fn(u64) -> Id)`; each number is a
`TextLink` (feature 1122) with the same menu-key and page wiring as the issue link
(`fn issue_link`, overview_card.rs ~l.522), factored into one helper both use. `CardGrid`'s
per-card `link_id: Option<Id>` becomes `link_ids: Vec<Id>`, `over_link` true on any of them; the
arrow step (`step_from`) maps every link id of a card to that card.

**Rationale**: FR-009 asks every 1122 link rule to hold for each number "in the same way": the
same `TextLink`, the same grid path, the same expansion events. Naming the slot keeps
`focused_link` exact with several links.

**Alternatives**: a second, parallel set of PR-link events: duplicates 1122's rules (and their
tests) per link kind. Indexing links by position: a reading that reorders rows would move focus to
another pull request; the number is stable.

## R7. Tab stops after Escape follow what is drawn

**Decision**: A number is a Tab stop exactly when its `TextLink` is in the widget tree; the
collapsed card builds only its first two rows, so after Escape collapses a focused card the hidden
numbers are no stops (FR-010) with no extra rule. Tab order is tree order: branch line (issue link),
then rows top to bottom.

**Rationale**: iced's focus traversal follows the tree; 1122 already relies on it for the issue
link.

## R8. Opening a pull request reuses the issue link's path

**Decision**: `features::overview::Msg::PullRequestLinkActivated(String)` carrying the address
from the reading; `main.rs` performs it exactly as `IssueLinkActivated` (feature 031 opener, its
FR-015 notification on failure). The address is only in memory and in this message (FR-014); the
message's `Debug` prints no address (a newtype with a redacting `Debug`, as `PullRequestStatus`
does).

**Rationale**: FR-009, FR-012, FR-014; one opener path.

## R9. Row layout and colours

**Decision**: A row is `[TextLink "#N"] [StatusChip pill] [Ellipsized status words]` on one line,
the number and pill at natural width, the words taking the rest and ellipsized first (FR-015,
1121 FR-007). The pill reuses `status_chip.rs` with roles mapped from the card-state colours of
feature 813: open and draft neutral (`secondary`), merged as done, closed muted; failing words take
the attention role, the rest the body role. Rows sit between the counts line and the card's
activity line; the "+N more" line is a muted body-role text below the rows. Contrast is covered by
`composition_contrast.rs` for each card state in both themes (feature 1120 FR-003).

**Rationale**: Principle VIII (shared components, roles); SC-006 (number and pill never clipped).

**Alternatives**: a new pill widget: `StatusChip` already draws a word on a tonal fill.

## R10. Docs, snapshot, showcase

**Decision**: `docs/user-guide/overview.md` "## Pull request counts" becomes "## Pull requests"
(rows, order, phrase precedence, "+N more", *10+*, opening by pointer and keyboard), and
"## Keyboard" lists the number stops. New covered state `overview-pr-rows` in
`crates/micold-client/tests/support/covered_states.rs` (a card with three pull requests, collapsed
and the expanded twin). Showcase `showcase/sections/surfaces.rs` gains a card with rows, collapsed
and expanded (FR-015).
