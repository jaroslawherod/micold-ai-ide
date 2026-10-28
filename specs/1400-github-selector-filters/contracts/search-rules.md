# Contract: search rules (`micold_core::github_filter`, new)

Pure functions, no I/O. Tests: `crates/micold-core/tests/github_filter.rs` (new), one test per
table row. Research R4–R7, R12.

## 1. Cleaning and limits (FR-012, Edge "Control characters", "Very long query")

| Input | Result |
|---|---|
| `"  is:open\n-label:x\t "` | `Ok`, `as_str() == "is:open -label:x"` |
| text with `\u{7}` | the character removed |
| 1,000 `char`s after cleaning (non-ASCII counted as one each) | `Ok` |
| 1,001 `char`s after cleaning | `Err(TooLong)` |
| `""`, `"   "` | `Ok`, empty; the client does not send it: an empty run shows the prompt (Edge "Empty advanced query") |

## 2. Refusals (FR-012, US4 AS5, SC-004)

Priority: `TooLong` > `UnclosedQuote` > `ScopeQualifier` > `UnbalancedParentheses`.

| Query | Result |
|---|---|
| `label:"in progress` | `UnclosedQuote` |
| `repo:other/x is:open` | `ScopeQualifier` |
| `-repo:other/x`, `org:acme`, `USER:bob`, `owner:me`, `(a OR repo:x)` | `ScopeQualifier` |
| `"repo:x" label:bug` | `Ok` (inside a quoted string, spec Edge) |
| `(a)repo:other/x`, `"x"repo:other/x`, `label:a,repo:other/x` | `ScopeQualifier` (after any character but a letter, digit, `_` or `-`) |
| `myrepo:x`, `my_repo:x`, `my-repo:x` | `Ok` (inside a word; GitHub judges it) |
| `(label:a OR label:b` | `UnbalancedParentheses` |
| `label:a) OR (label:b` | `UnbalancedParentheses` (depth below zero) |
| `"(" label:a` | `Ok` |
| `state:open no:assignee -linked:pr -label:blocked sort:created-asc` | `Ok` |
| `is:pr` | `Ok` (Edge: runs, empty result) |

## 3. Sort split

| Query | `condition()` | `sorts()` |
|---|---|---|
| `-label:x sort:created-asc` | `-label:x` | `["sort:created-asc"]` |
| `sort:created-asc` | `` | `["sort:created-asc"]` |
| `(sort:updated label:a)` | `( label:a)` (the token cut, whitespace runs collapsed) | `["sort:updated"]` |
| `"sort:x" bug` | `"sort:x" bug` | `[]` |
| `SORT:comments-desc a` | `a` | `["SORT:comments-desc"]` |

## 4. Scoped search text (FR-004, FR-011, research R4)

Repository `acme/app`.

| Filter | `ScopedSearch::listing(..).as_str()` |
|---|---|
| Basic: exclude `in-progress`, `blocked` | `repo:acme/app is:issue is:open -label:"in-progress" -label:"blocked" sort:updated-desc` |
| Basic: include `bug`, `ui`; state all | `repo:acme/app is:issue label:"bug" label:"ui" sort:updated-desc` |
| Basic: state closed | `repo:acme/app is:issue is:closed sort:updated-desc` |
| Basic: include `good first issue`, `日本語` | `… is:open label:"good first issue" label:"日本語" sort:updated-desc` |
| US6 Basic: assignee Me, author `octo-cat`, milestone `v2.4` | `… is:open assignee:@me author:octo-cat milestone:"v2.4" sort:updated-desc` |
| US6 Basic: assignee Nobody, milestone Nobody | `… is:open no:assignee no:milestone sort:updated-desc` |
| Advanced `-label:in-progress state:open` | `repo:acme/app is:issue (-label:in-progress state:open) sort:updated-desc` |
| Advanced `a OR b sort:created-asc` | `repo:acme/app is:issue (a OR b) sort:created-asc` |
| Advanced `` (empty) | `repo:acme/app is:issue sort:updated-desc` (constructible, never sent: an empty run sends nothing) |
| Advanced `sort:created-asc` | `repo:acme/app is:issue sort:created-asc` |
| Default basic | not constructible: `GithubFilter::is_default` → today's path (FR-009) |

`ScopedSearch::beyond(repo, filter, typed)` is the listing text without its sort qualifiers, then
the typed search text as **quoted words** (research R4): the typed text is cleaned like a query
(§1), every `"` and `\` is removed, it is split on whitespace, and each word is appended as ` "<word>"`.
A quoted word is a literal search term under `ISSUE_ADVANCED`: it cannot be read as `OR`/`AND`/`NOT`,
a parenthesis, a `-` negation or a qualifier, so the typed text can only narrow the filter, never
widen it or change its scope (FR-016). `\` is dropped too: GitHub's handling of a backslash inside a
quoted term is not verified live (research R7), and a word ending in `\` could otherwise close its
own quote early. This is 034's meaning of the typed text, where `type: ISSUE`
reads `OR` as a plain word. No word left (empty, whitespace or only `"` and `\`) → `None`: no search
beyond is sent, the local narrowing alone applies. 034's default-filter `search_args` is untouched
(FR-009).

| Filter | Typed text | `ScopedSearch::beyond(..).map(as_str)` |
|---|---|---|
| Basic: exclude `blocked` | `crash` | `Some("repo:acme/app is:issue is:open -label:\"blocked\" \"crash\"")` |
| Basic: exclude `blocked` | `crash OR ui` | `Some("… -label:\"blocked\" \"crash\" \"OR\" \"ui\"")` (no top-level `OR`) |
| Basic: exclude `blocked` | `a)` | `Some("… -label:\"blocked\" \"a)\"")` (parentheses stay balanced) |
| Basic: exclude `blocked` | `repo:x` | `Some("… -label:\"blocked\" \"repo:x\"")` (a word, not a scope qualifier) |
| Basic: exclude `blocked` | `-label:ui (x` | `Some("… -label:\"blocked\" \"-label:ui\" \"(x\"")` |
| Basic: exclude `blocked` | `say "hi"` | `Some("… -label:\"blocked\" \"say\" \"hi\"")` (quotes dropped) |
| Basic: exclude `blocked` | `a\ OR b` | `Some("… -label:\"blocked\" \"a\" \"OR\" \"b\"")` (backslash dropped) |
| Advanced `a OR b sort:created-asc` | `crash OR ui` | `Some("repo:acme/app is:issue (a OR b) \"crash\" \"OR\" \"ui\"")` |
| any | `""`, `"   "`, `"\"\""`, `"\\"`, `"\\\" \\"` | `None` |

The string cells are Rust literals; `…` stands for `repo:acme/app is:issue is:open`. One core test
per row in `github_filter.rs`.

## 5. Values (FR-006, research R6)

| Input | `LabelName::parse` |
|---|---|
| `bug`, `good first issue`, `area: ui`, `(wip)`, `日本語`, ` lead space` (kept as is) | `Some` |
| `say "hi"`, `tab\there`, ``, `foo\`, `a\b` | `None` (`\` refused as typed words drop it: if GitHub read `\"` as an escape, `label:"foo\"` would leave its quote open) |

| Input | US6 `Login::parse` |
|---|---|
| `octocat`, `octo-cat`, `a`, 39 chars | `Some` |
| `-octo`, `octo-`, `oc--to`, `octo cat`, `octo:cat`, 40 chars, `` | `None` |

## 6. Answers (FR-014, FR-015, FR-018, research R7, R9)

`SEARCH_PAGE_QUERY` (new, one line):
`query($q: String!, $cursor: String) { search(type: ISSUE_ADVANCED, query: $q, first: 100, after:
$cursor) { issueCount pageInfo { hasNextPage endCursor } nodes { ... on Issue { <ISSUE_NODE_SELECTION>
bodyText state repository { nameWithOwner } } } } }`. `SEARCH_FILTERED_QUERY` is the same with
`first: 50`, no cursor.

`parse_search_page(stdout, repo) -> Result<IssuePage, FilterFailure>`:

| Answer | Result |
|---|---|
| nodes of `acme/app` issues, open and closed | issues in GitHub's order, closed ones `is_closed()`; `total_open = issueCount` |
| a node of `ACME/App` | kept (repository compared ASCII case-insensitively) |
| a node of `other/x`, an empty node (pull request) | dropped (FR-015) before the cap counts |
| `errors: [{type: "INVALID", path: ["search"], message: "m1"}, {path: ["search"], message: "m2"}]` | `Rejected(["m1", "m2"])` |
| `errors: [{type: "NOT_FOUND", path: ["search"], message: "The listed users cannot be searched…"}]` | `Rejected([..])` |
| `errors: [{message: "x"}]`, no `data.search` | `Rejected(["x"])` |
| `errors: [{type: "RATE_LIMITED", …}]` | `Load(RateLimited)` |
| `errors: [{type: "FORBIDDEN", path: ["search"], …}]` (beside any refusal) | `Load(graphql_error_of(..))`; `gh`'s stderr then gives 034's SAML notice (`NoAccess`) |
| `errors: [{type: "INSUFFICIENT_SCOPES", path: ["search"], …}]` (beside any refusal) | `Load(graphql_error_of(..))`; `gh`'s stderr then gives 034's scope notice |
| a message of 400 chars / with `\n` / empty | cut at 300 + `…` / cleaned / dropped; at most 10 kept |
| JSON that is not this shape | `Load(Other(..))` |

`load_filtered(source, repo, &ScopedSearch) -> Result<IssueListing, FilterFailure>` pages through
`load_pages` (cap 1,000, first error aborts); `complete = exhausted`; `total_open = issueCount`.
