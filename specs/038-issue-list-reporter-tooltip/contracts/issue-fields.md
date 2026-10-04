# Contract: issue fields — reporter, description, display lines, match text

**Module**: `crates/micold-core/src/github.rs` · **Research**: R1–R4, R13, R14 ·
**Data model**: [§1–4](../data-model.md)

## 1. Queries

`LIST_QUERY`, `SEARCH_QUERY` and `SEARCH_WITH_NUMBER_QUERY` each select, on every issue node,
`author { login }` beside `number title updatedAt [state] labels(first: 20){nodes{name}}`. The two
search queries also select `bodyText`; `LIST_QUERY` does not (SC-008, research R14):

```graphql
author { login }
bodyText        # the search queries and DESCRIPTIONS_QUERY only
```

`DESCRIPTIONS_QUERY` (M5) reads the connection `LIST_QUERY` reads — `issues(states: OPEN, first:
100, after: $cursor, orderBy: {field: UPDATED_AT, direction: DESC})` — and selects `pageInfo {
hasNextPage endCursor }` and, per node, `number bodyText` only. `descriptions_args(repo, cursor)`
is `list_args(repo, cursor)` with that query: the same variables, nothing more (FR-026).

- `author { login }` ships in milestone M1; `bodyText` and the description pass ship in M5.
- Nothing else in the query text changes: not the arguments, the page size, the ordering, the
  `states` filter or the search string. `list_args` and `search_args` are untouched (FR-013,
  FR-026). The existing tests `list_args_send_only_the_repository` and
  `search_args_send_only_the_query` are extended to assert that the arguments carry no `author:`
  qualifier and no variable beyond today's.
- The three queries share one node selection (a `const` fragment string), so a node from any of
  them parses by `issue_from_node` (FR-006). A test asserts each query text contains the shared
  selection. A search node carries `bodyText` after it; a list node has none and parses to an issue
  without a description, which the description pass fills in (§6).

## 2. Parsing

`issue_from_node(node)`:

| Node | `Issue` |
|---|---|
| `"author": {"login": "octocat"}` | `reporter == "octocat"` |
| `"author": null`, or no `author` key | `reporter == "ghost"` (`GHOST_LOGIN`) |
| `"author": {"login": "dependabot"}` (a bot; GitHub omits the `[bot]` suffix in `login`) | shown as reported; nothing marks it |
| `"bodyText": "Problem\nThe list cuts long titles off."` | `description == "Problem The list cuts long titles off."` |
| `"bodyText": ""`, `"  \n\n "`, `null`, or no `bodyText` key | `description == ""` |
| `bodyText` of 65,536 characters | `description` has 601 characters and ends in `…` |

A node that lacks `author` or `bodyText` still yields an issue (spec, Edge Cases: loading failure).
A node that fails for 034's reasons fails as it does today.

`description_from(body_text: &str) -> String` is public and pure; its rules are
[data-model §2](../data-model.md). `DESCRIPTION_MAX_CHARS = 600`.

**FR-022 and `bodyText`.** GitHub renders the body to text: heading, emphasis, list, checkbox and
fence markers are gone, a link is its text, an HTML comment is absent, and non-marker characters
stay. A fixture under `crates/micold-core/tests/fixtures/gh/` holds a captured node whose body is
story 3 scenario 12's (a hidden comment, `## Problem`, a blank line, `The **list** cuts [long
titles](https://example.com) off.`); the test asserts `description == "Problem The list cuts long
titles off."`. A second fixture node, whose body is only a comment, asserts `description == ""`
(scenario 13).

## 3. Display lines

```rust
impl Issue {
    pub fn title_line(&self) -> String;     // "#<number> <title>"
    pub fn details_line(&self) -> String;   // "<reporter>" [+ "  ·  " + labels.join(", ")]
}
```

| Issue | `title_line()` | `details_line()` |
|---|---|---|
| #7 "Fix it", by `ana`, labels `bug`, `ui` | `#7 Fix it` | `ana  ·  bug, ui` |
| #8 "Docs", by `ana`, no labels | `#8 Docs` | `ana` |
| #9 "Old", no author, label `bug` | `#9 Old` | `ghost  ·  bug` |

The separator is the one 034 uses between title and labels (`"  ·  "`), so the reporter is
"visibly separated" from the labels (FR-003) and a label spelled like a login cannot be mistaken
for the reporter: the reporter is always first.

## 4. Match text and emphasis

`Issue::row_text()` is what `typeahead::rank` reads.

| Milestone | `row_text()` for #7 above |
|---|---|
| M1 | `#7 Fix it  ·  bug, ui` (as today) |
| M3 on | `#7 Fix it  ·  ana  ·  bug, ui` |

```rust
pub struct RowEmphasis { pub title: Vec<Range<usize>>, pub details: Vec<Range<usize>> }
impl Issue { pub fn emphasis(&self, spans: &[Range<usize>]) -> RowEmphasis; }
```

Cases the tests hold (M3 match text, issue #7):

| Typed | Span of `row_text` | `title` | `details` |
|---|---|---|---|
| `fix` | `3..6` | `3..6` | — |
| `ana` | `15..18` | — | `0..3` |
| `ui` | `29..31` | — | `14..16` |
| (a span crossing title, separator and reporter) | `7..16` | `7..9` | `0..1` |
| (a span over a separator only) | `10..13` | — | — |

Offsets are bytes; the separator `"  ·  "` is 6 bytes.

- The description is never in `row_text`: an issue whose description contains `zebra` and whose
  number, title, labels and reporter do not is not matched by `zebra` (FR-014).
- Letter case: `rank` already folds case; `ANA` matches `ana` (FR-009, story 2 scenario 4). No
  change to `micold_core::typeahead`.
- `typeahead_budget.rs` ranks 1,000 issue rows that carry reporters, under the existing 50 ms
  release budget.

## 5. Privacy (FR-025)

- `Issue` has no `Serialize` (the existing test stays).
- `impl Debug for Issue` prints `reporter: "<redacted>"` and `description: "<redacted>"`; number,
  title and labels print as today. A test formats an issue with `{:?}` and asserts neither value
  appears.
- `parse_list_page` on a malformed page that contains a body returns an `IssueLoadError` whose
  `Display` and `Debug` contain no part of that body.
- No `tracing` or `log` macro in `github.rs`, `shell/issues.rs` or `features/worktree_form.rs`
  takes an `Issue`, a reporter or a description. A source gate (`github_privacy.rs`, reading the
  three files) holds it.

## 6. The description pass and load time (FR-024, FR-026, SC-008)

The list appears from `LIST_QUERY` alone, so its load time is what it was before `bodyText`
(measured in [quickstart §B10](../quickstart.md); research R14). Descriptions follow:

| Item | Rule |
|---|---|
| `DescriptionPage { descriptions: Vec<(u64, String)>, next_cursor }` | One page of the pass: issue number and `description_from(bodyText)`. Its `Debug` prints the count, never a description (FR-025). |
| `parse_descriptions_page(stdout)` | A node without `bodyText`, or with `null`, gives `""`; a node without a `number` is skipped. GraphQL errors and a malformed answer are classified as `parse_list_page` classifies them, and the error carries no part of a body. |
| `IssueSource::describe_open(repo, cursor)` | One request, bound by the same 10 s as every other (034 FR-007). `FakeIssueSource::with_descriptions` scripts it and `description_calls()` records it. |
| `describe_listed(issues, page)` | Puts each description on the held issue with that number. An issue the page does not name keeps what it has; a number that is not held is ignored. |
| `next_description_cursor(asked, page, pages_read)` | The cursor of the next request, or `None`: on the last page, on a page with no node, on a cursor equal to the one asked with, and once `DESCRIPTION_PAGE_CAP` (10) pages are read. |

The form (`features/worktree_form.rs`): `IssueList::Loaded` holds `descriptions: DescriptionPass`,
`Loading { seq, cursor, pages }` or `Done`. An accepted load sets `Loading` with the load's `seq`
and no cursor, unless the listing holds no issue. `Msg::IssueDescriptionsLoaded { seq, cursor,
result }` applies only while the pass awaits exactly that `seq` and `cursor`; `Ok` describes the
held issues and moves to the next cursor or `Done`; `Err` sets `Done` and changes nothing else: no
error is shown and nothing is retried. Leaving the source, a new load or closing the form drops the
pass with the list. The highlight, the matches and the open list do not change when a page lands.

The shell (`shell/issues.rs`): `start_issue_descriptions` is the only path to `describe_open`. It
runs when the reducer accepted a load, and again when it accepted a page and awaits another: never
from a view, a timer or a hover (SC-006). `issues_are_requested_only_on_named_events.rs` counts
its callers.
