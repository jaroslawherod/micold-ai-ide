# Contract: issue fields — reporter, description, display lines, match text

**Module**: `crates/micold-core/src/github.rs` · **Research**: R1–R4, R13, R14 ·
**Data model**: [§1–4](../data-model.md)

## 1. Queries

`LIST_QUERY`, `SEARCH_QUERY` and `SEARCH_WITH_NUMBER_QUERY` each select, on every issue node,
two more fields beside `number title updatedAt [state] labels(first: 20){nodes{name}}`:

```graphql
author { login }
bodyText
```

- `author { login }` ships in milestone M1; `bodyText` ships in M4.
- Nothing else in the query text changes: not the arguments, the page size, the ordering, the
  `states` filter or the search string. `list_args` and `search_args` are untouched (FR-013,
  FR-026). The existing tests `list_args_send_only_the_repository` and
  `search_args_send_only_the_query` are extended to assert that the arguments carry no `author:`
  qualifier and no variable beyond today's.
- The three queries share one node selection (a `const` fragment string), so a node from any of
  them parses by `issue_from_node` (FR-006). A test asserts each query text contains the shared
  selection.

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
| M2 on | `#7 Fix it  ·  ana  ·  bug, ui` |

```rust
pub struct RowEmphasis { pub title: Vec<Range<usize>>, pub details: Vec<Range<usize>> }
impl Issue { pub fn emphasis(&self, spans: &[Range<usize>]) -> RowEmphasis; }
```

Cases the tests hold (M2 match text, issue #7):

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

## 6. Load time (SC-008)

Not a contract of this module beyond "no extra request". Measured in
[quickstart §B10](../quickstart.md); the fallback is in research R14.
