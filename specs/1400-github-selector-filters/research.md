# Research: Filter the GitHub issue selector by search query, label and state

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Each decision names what was chosen, why, and what was rejected. Paths are as of `origin/main`
828bc8e24; "new" marks a file or symbol this feature adds. #1185 (Jira basic filters) and #1186
(Jira advanced JQL) are merged; this feature mirrors their split where it fits and reuses their
shared pieces instead of copying them.

## R1 — One new core module `micold_core::github_filter` beside `github.rs`

**Decision.** Every rule this feature adds — the basic filter values and their search terms, the
advanced query's cleaning and checks, the scoped search text, the classification of GitHub's
answer, the filtered load and the remembered state's format — lives in a new render-free module
`crates/micold-core/src/github_filter.rs`, declared beside `pub mod github;` in
`crates/micold-core/src/lib.rs`. `github.rs` gets only additive hunks: trait methods with default
bodies, their `GhCli`/`FakeIssueSource` implementations, the `closed` flag on `Issue`, and one
`keep_closed` parameter for the node reader.

**Why.** The same split 1185 (`jira_filter.rs`) and 1186 (`jira_query.rs`) made beside `jira.rs`:
decision logic in core, reachable by `mise run test-core`, under Principle I; `github.rs` is
already 2,276 lines. The default filter's request builders (`list_args`, `search_args`,
`LIST_QUERY`, `SEARCH_QUERY`) are not edited, which is what makes FR-009 ("the default filter
sends exactly as today") true by construction rather than by test alone.

**Rejected.** (a) Parameters on `list_args`/`search_args`: they carry 034's privacy contract
(`tests/github_privacy.rs`) and the default path; widening them risks FR-009. (b) Building the
query in the client reducer: logic outside core.

## R2 — Filtered listings use GitHub's issue search; the default filter keeps today's listing

**Decision.** With the default filter (basic, no label/assignee/author/milestone, state open) the
form loads exactly as today: `load_listing` over `LIST_QUERY` (`repository.issues(states: OPEN,
orderBy: UPDATED_AT DESC)`), then the description pass. Any other filter — basic or advanced — is
one scoped search text (R4) paged through GraphQL `search(type: ISSUE_ADVANCED, query: $q,
first: 100, after: $cursor)` with `issueCount` as the total, through the existing generic
`github::load_pages` (cap 1,000, first error aborts).

**Why.** The spec's acceptance criterion is "the same issues as the same search on github.com",
and the issue fixes `repo:` and `is:issue` as the scope, both of which are search syntax.
`repository.issues(filterBy:)` cannot exclude a label, ORs included labels, and has no `no:` or
`linked:`. GitHub's search returns at most 1,000 results, which is the selector's cap already
(spec Assumptions).

**Rejected.** (a) `repository.issues(filterBy: {labels, states, assignee, createdBy, milestone})`
for basic mode: no exclusion (the P1 use case), OR semantics for labels (spec says AND). (b) The
REST `search/issues` endpoint: a second response shape and paging scheme beside GraphQL, which
every other GitHub read in `github.rs` uses.

## R3 — `ISSUE_ADVANCED` search type

**Decision.** Filtered requests send `type: ISSUE_ADVANCED`, the GraphQL search type for
github.com's issue search syntax with `AND`/`OR` and parentheses. The value is one constant,
`github_filter::SEARCH_TYPE`. The default filter's search beyond the loaded issues keeps 034's
`type: ISSUE` (FR-009).

**Why.** FR-011 asks for boolean operators and parentheses "where GitHub's issue search
supports them", and SC-002's test list has `OR` and parentheses; the legacy `ISSUE` type reads
`OR` as a word. github.com's issue search uses the advanced syntax, so the same type is what makes
"same results as github.com" hold.

**Rejected.** `type: ISSUE`: `OR` and parentheses silently mean something else. **Risk, checked
at M1:** the cloud container has no signed-in `gh`, so the enum value is taken from GitHub's
changelog, not a live probe. M1's first task records a real `gh api graphql` probe of one search
with `ISSUE_ADVANCED` into `crates/micold-core/tests/fixtures/github_search/` when a signed-in
`gh` is available; if GitHub refuses the value, only `SEARCH_TYPE` changes (and FR-011's boolean
terms fall back to what `ISSUE` supports, recorded as a ledger deviation).

## R4 — The scoped search text

**Decision.** The only search text a filtered request sends is built by
`github_filter::ScopedSearch::new(repo, &filter)` (constructed nowhere else):

- **Basic:** `repo:<owner>/<name> is:issue` + the state term (`is:open`, `is:closed`, none for
  all) + each included label as `label:"<name>"` in chosen order + each excluded label as
  `-label:"<name>"` + (US6) `assignee:@me` / `no:assignee` / `assignee:<login>`, `author:@me` /
  `author:<login>`, `milestone:"<title>"` / `no:milestone` + `sort:updated-desc`.
- **Advanced:** `repo:<owner>/<name> is:issue` + `(<condition>)` when the condition is not empty
  + the user's sort qualifiers in typed order, or `sort:updated-desc` when there are none.
- **Search beyond the loaded issues** (FR-016): the same text without its sort qualifiers,
  followed by the typed search text as quoted words — cleaned, every `"` and `\` removed, split on
  whitespace, each word appended as ` "<word>"` — sent with `ISSUE_ADVANCED`, `first: 50` and no
  number lookup (Edge "Typed issue number with a filter"). No word left → no request. The raw
  typed text is never appended: under the advanced syntax a typed `OR` or `)` would escape the
  filter (`… -label:"blocked" crash OR ui` reads as `(… crash) OR ui`) and a typed `repo:x` would
  add a scope term, the same escape the parentheses around the advanced condition close. A quoted
  word is a literal term, so the typed text only narrows, and it keeps 034's meaning (`type:
  ISSUE` reads `OR` as a word). `\` is removed as well as `"`: whether GitHub reads `\"` inside a
  quoted term as an escaped quote is unverified (R7), and if it does, a typed `a\ OR b` would
  give `"a\" "OR" "b"` with `OR` left bare. search-rules §4 lists the cases.

`GithubRepo` is validated at construction (`GithubRepo::checked`: no space, quote or colon), so
the `repo:` term cannot be broken by the repository's name.

**Why.** Parentheses around the user's condition make a top-level `OR` bind inside the scope
(`repo:x is:issue a OR b` would otherwise read as `(repo:x is:issue a) OR b` under the advanced
syntax); the scope qualifiers sit outside, where the user text cannot reach them (R5). Sort
qualifiers are filters of nothing, so moving them out of the parentheses changes no result and
keeps `sort:` from appearing inside a group. `sort:updated-desc` is the selector's order today
(FR-011, spec Assumption "Same results as github.com").

**Rejected.** (a) Appending the user query unwrapped (`repo:x is:issue <query>`): a top-level `OR`
escapes the scope; FR-015's post-filter would hide it, but the list would hold wrong issues up to
the cap. (b) Rewriting the query into an AST: GitHub decides what is valid (Edge "Unknown
qualifier"); the app only guarantees the scope.
(c) For the search beyond: sending it with `type: ISSUE` and the raw typed text — no
`ISSUE` form exists for an advanced condition's `OR` and parentheses, and a typed `repo:x` would
still add a repository; wrapping the typed text in its own parentheses — a typed `a)` or `"`
would unbalance it and needs the same scanner as R5 with a refusal the search field has no place
to show; one `"…"` phrase for the whole text — a phrase match, narrower than 034's words.

## R5 — One left-to-right scanner for the advanced query

**Decision.** `github_filter::UserQuery::parse(raw)` cleans the text (`\n` `\r` `\t` → space, other
control characters removed, ends trimmed: the shared `micold_core::query_text::clean`, R12), refuses
it when longer than 1,000 `char`s, then walks it once, tracking whether it is inside a `"` quoted
string (GitHub's search has no escape inside quotes, so the next `"` closes) and the parenthesis
depth outside strings. At each position outside a string that follows no letter, digit, `_` or `-`
(so `(a)repo:x` and `"x"repo:x` count, `myrepo:x` does not; M1 review A F2), it checks for an
optional `-` then `repo:`, `org:`, `user:` or `owner:` (ASCII case-insensitive) — a scope
qualifier. At each token start (start of text, or after whitespace or `(`) it records each `sort:`
token's byte range. Refusals, in
priority order: `TooLong`, `UnclosedQuote`, `ScopeQualifier`, `UnbalancedParentheses` (depth below
zero at any point, or non-zero at the end). `condition()` is the text with the sort tokens cut out
and runs of spaces collapsed; `sorts()` the sort tokens in order.

**Why.** The scope is `repo:… is:issue (…)`. The only ways the user text could widen it are a
scope qualifier (GitHub ORs several `repo:` terms rather than narrowing, spec Edge), a `)` that
closes the group early, a `(` that swallows what follows, or a quote that swallows the closing
`)`. One pass catches all four, as 1186 R2 does for JQL. A qualifier inside a quoted string is
text and runs (spec Edge).

**Rejected.** (a) Reusing `jira_query::UserQuery`: JQL has `'` strings with backslash escapes and
`ORDER BY`, GitHub has neither; a shared scanner would need both grammars behind flags. (b) A
regex per rule: the quote state is what decides whether a `repo:` counts, which regexes cannot
track.

## R6 — Basic filter values are quoted and constructed only by validation

**Decision.** `LabelName::parse` and `MilestoneTitle::parse` (US6) refuse an empty name and any
name holding `"` or a control character (both also `\`, as typed words drop it: if GitHub
read `\"` as an escape the quote would stay open; M1 review A F1); every accepted value is sent as `label:"<name>"`. `Login`
(US6) accepts 1–39 ASCII letters, digits and single inner hyphens (no leading, trailing or double
hyphen), sent bare. `Assignee::Me`/`Author::Me` send `@me`. A label GitHub returns that fails
`LabelName::parse` is not offered, and the label field counts them ("2 labels can't be offered").

**Why.** FR-006: a value must match exactly that value and never change the query's structure.
Inside `"…"` GitHub's search reads spaces, colons, parentheses and non-Latin letters as text; it
cannot express a `"` inside one, so such a label is not offered (spec Edge). A login validated to
GitHub's own rule cannot hold a space or a colon.

**Rejected.** Escaping `"` as `\"`: GitHub's search has no escape; the value would match something
else.

## R7 — GitHub's refusal is `FilterFailure::Rejected`, classified from the GraphQL errors at `search`

**Decision.** A filtered request returns `Result<_, FilterFailure>` where
`FilterFailure::{Rejected(Vec<String>), Load(IssueLoadError)}` (new, core). The page parser reads
`errors[]`: `RATE_LIMITED`, `FORBIDDEN` and `INSUFFICIENT_SCOPES` are `Load(..)`, classified as the
default listing classifies them (034's SAML and scope notices; M1 review A F3); every other error whose `path` is `["search"]`,
or which has no `path` and no `data.search`, is a refusal of the query, and the messages of all
such errors, in GitHub's order, become `Rejected` — each cleaned (R12), cut at 300 characters with
`…`, at most 10, empty ones dropped. Any other error is classified as `parse_list_page` classifies
it. A `gh` exit that the parser cannot read goes through `answer_of`'s stderr classification
(not signed in, SAML, offline, timed out) as today; a `Rejected` from the parser stands whatever
the exit code (`gh` exits 1 on any GraphQL error).

**Why.** FR-018/FR-019 and US4: GitHub's own words for a bad query, 034's notices and Retry for
everything else. `NOT_FOUND` under `search` is GitHub's answer to `author:no-such-user` or a
milestone the search cannot resolve ("The listed users cannot be searched…"), which is a fault in
the query, not "no access" — the repository's access is already proven by the request scope or
reported by the default listing. 300 characters and 10 messages are 1186 R4's cut, kept for one
look.

**Rejected.** (a) A new `IssueLoadError::Rejected` variant: every exhaustive `match` on
`IssueLoadError` (notices in both providers, the Jira code) would have to learn a case only
filtered GitHub requests produce. (b) Reusing `jira_query::QueryFailure`: Jira-named, and its
`Load` branch is the same shape; a second two-variant enum costs less than a misleading name.
**Risk:** the exact `type` GitHub gives a refused search is not probed (no signed-in `gh` in the
container); the rule above does not depend on it. M1's fixture probe (R3) records real answers for
three refused queries when a signed-in `gh` is available, and one search whose quoted term holds a
backslash (`"a\" "OR" "b"`), to record how GitHub reads `\` inside quotes. Unverified until then;
the search beyond drops `\` from typed words (R4), so its safety does not depend on the answer.

## R8 — One `IssueList` for every filter; refusals and rejections are list states

**Decision.** The form keeps one issue list (`WorktreeForm::issues`) whatever the filter; search
text narrows it locally by 034 FR-005's rule in both modes. `IssueList` gains two variants:
`Refused(QueryRefusal)` (the app did not send) and `Rejected(Vec<String>)` (GitHub refused). A new
`github_filter::GithubSelector` sub-state on the form holds the mode, the basic filters, the draft
and the last run query; the list itself is not duplicated. Switching to basic after a query ran
reloads the basic filters' list (no query runs); switching when no query has run since basic mode
was left changes nothing (the list shown is already basic's). Switching to advanced never loads
(FR-002).

**Why.** Unlike 1186, the search text stays in advanced mode (spec Assumptions), so advanced
results need the same local narrowing, search-beyond, highlight, pick and details as the basic
list: one list reuses all of it. The reload on switch-back gives the basic filters' list as GitHub
holds it now, which is the list "as it was" up to edits made meanwhile (US3 AS6).

**Rejected.** (a) 1186's separate advanced list (`QueryRun::Loaded`): the search text, search
beyond and descriptions would need a second copy. (b) Stashing the basic `IssueList` while
advanced runs: a stashed `Loading { seq }` or `Searching { seq }` is stale on return, and the
stash would carry a stale `gh` path and search state.

## R9 — Filtered pages carry the body; no second description pass

**Decision.** Search nodes already select `bodyText` (034's `SEARCH_QUERY`); filtered pages select
it too, so a filtered `Loaded` list starts with `DescriptionPass::Done` and makes no description
requests. The default filter keeps 038's two passes.

**Why.** A second pass over a search would have to re-run the same search with cursors, which can
re-order between passes, and the default list (the large, common case) keeps its speed.

**Rejected.** A `describe_search` pass mirroring 038: more requests, more code, no gain for the
usual filtered list of tens of issues. **Cost accepted:** a filtered list of 1,000 issues arrives
about 1.9 times slower than without bodies (038 R14).

## R10 — Choices: on first focus, through the located `gh`, never persisted

**Decision.** Labels: `repository.labels(first: 100, after:, orderBy: {field: NAME, direction:
ASC})` paged up to 10 pages (1,000), then sorted case-insensitively and de-duplicated locally;
`partial` when a next cursor remained. Asked once per form, on the first focus of either label
field (include or exclude), shared by both. US6: milestones `repository.milestones(states: OPEN,
first: 100, after:)` paged up to 1,000, sorted by title; people `repository.assignableUsers(query:
$text, first: 50)` per typed text, debounced 300 ms (1185's people search). "me" is `@me` and
needs no request (no `viewer` query). The `gh` used is the one the listing located
(`IssueList::Loaded { gh }`), or else located the same way the listing does (the env-include path),
on a blocking thread. Each choice list has its own seq, so a late answer is dropped (1185
`ChoiceLoad`). Choices live in the open form only (FR-005, 034 FR-023).

**Why.** FR-005 asks for the repository's labels (not the labels of loaded issues, which 1185
used because Jira has no cheap label list); GitHub has one. Sorting locally makes the order
independent of GitHub's collation.

**Rejected.** (a) Labels from the loaded issues (1185 R5): a filtered or closed-state list would
offer only the labels it holds. (b) Loading choices when the form opens: requests the user never
asked for (FR-005, 034 FR-023).

## R11 — Remembered state: one owner-only JSON file per project in the app's data directory

**Decision.** `<data_dir>/github/<project id>.selector.json` (`store::github_selector_path`, new;
`paths::data_subdir("github")`), written with `micold_core::owner_only::write` through a temporary
file and rename, as 1186 writes `<data_dir>/jira/<id>.query.json`. Contents
`{"mode":"basic"|"advanced","basic":{…},"query":"…"}`, every field `#[serde(default)]`. Read at
each form open (and nowhere else, so another window's write never changes an open form); written
as a read-modify-write of one field: the mode at each switch, the basic filters at each change, the
query at each run (never the draft). Read leniently: unparseable JSON is the default; an invalid
label, login or milestone is dropped; a refused query is the empty query. Deleted by
`shell::daemon_sync::on_project_forget_confirmed` (FR-022). The read-modify-write and the lenient
read share one helper with 1186's file (`shell::remembered`, new, extracted from
`shell/jira.rs::{read_query, remember_query, remove_query}`).

**Why.** FR-021 wants local per-user data, not `settings.json`, a project file or the repository;
the data directory is that on every platform (Principle VI). One field per write keeps "last write
wins" per field with two windows (spec Edge). A file per project, as 1186, so forgetting a project
removes exactly its file.

**Rejected.** (a) The project's state file under `projects/<id>.json`: the daemon owns and rewrites
it, and the daemon may be remote; this is client state. (b) `settings.json`: shared across projects
and synced by users; FR-021 forbids it. (c) A copy of 1186's read/write functions: the duplication
budget (`mise run duplication`) and CLAUDE.md's "search for an existing helper".

## R12 — Shared query-text helpers

**Decision.** `jira_query`'s private `clean` and `cut_message` move to a new
`crates/micold-core/src/query_text.rs` (`pub fn clean`, `pub fn cut_message`) used by both
`jira_query` and `github_filter`. `QueryMode` (`jira_query::QueryMode`, Basic/Advanced, serde
lowercase) is reused for the GitHub mode; its doc stops naming Jira. The client's Basic/Advanced
chips (`ui/jira_query.rs::mode_switch`) are generalised to take the mode and the message, and both
selectors call them.

**Why.** Same rule, same text; copying them is the failure `mise run duplication` budgets.

**Rejected.** Copies in `github_filter`.

## R13 — Basic filters live in a panel built from shared widgets

**Decision.** The **Filters** button and its floating panel reuse 1185's `material::MenuSelect`
with `MenuSelectRow::controls`, its 160 px button width and the active-line layout. Inside the
panel: include labels and exclude labels are each a `material::Typeahead` over the label choices
(a pick adds the label and clears the field) above the chosen labels as `ToggleChip`s with a
trailing close icon (pressing removes); state is three `ToggleChip`s (Open, Closed, All); US6's
assignee, author and milestone are single-value `Typeahead`s like 1185's fields. A closed issue's
row carries a `TicketStatus { name: "Closed", category: Some(Done) }`, drawn by the existing
`TypeaheadRow::status` chip, ahead of a status the project's status board holds for that issue
(FR-014 requires the closed mark; selector-ui §3). The panel frame code shared with `jira_filters_control` is extracted
into one function both call. No new widget.

**Why.** Principle VIII: the panel, fields, chips and status chip exist; multi-value labels are
chips under a typeahead, a pattern `ToggleChip` already supports with `trailing_icon`. The panel
floats (1185 D15), so chips cost the tallest dialog no height. Its list caps at 12 menu items
(`FILTERS_PANEL_HEIGHT`, shared with Jira's Filters panel, M7); taller content scrolls inside it.

**Rejected.** (a) A new multi-select widget: one use, and `Typeahead` + `ToggleChip` already
compose it. (b) A "Closed" word in the details line: the row already has a status slot built for
exactly this.

## R14 — Advanced mode layout: query row in place of the active line

**Decision.** The Basic/Advanced switch heads the picker as in 1186 (with **Clear filters**
trailing it in basic mode, 1185 D12). In advanced mode the **Filters** button and the active line
are replaced by one row: a `material::TextField` labelled "Query" with `.on_submit(Run)` and a
**Run** button; the issue search typeahead stays below it. The dialog's tallest state is the same
in both modes (basic: switch row, search row with Filters, active line; advanced: switch row, query
row, search row).

**Why.** FR-001 (search stays), US3 AS1 (Filters replaced by a single query input), and the
panel-placement gate: the advanced query row takes the active line's height.

**Rejected.** Using the query field as the results typeahead (1186): there would be no search text.

## R15 — Requests and privacy

**Decision.** A filtered listing or search sends the scoped search text (repository, filter,
typed text) and nothing else; a label or milestone choice request sends the repository; a people
search the repository and the typed text (FR-023). The statement under the picker becomes "Reads
issues of <repo> from GitHub, sending the filter and the search text." while a non-default filter
is active, and stays 034's text otherwise (FR-009). No `tracing::`/`log` macro, notification or
session history takes a `UserQuery`, `ScopedSearch`, `BasicFilters` or draft (FR-024), checked by
the source scan `issues_are_requested_only_on_named_events.rs` (extended) and a `{:?}` redaction
test: `BasicFilters`, `UserQuery`, `ScopedSearch` and `GithubSelector` print `<redacted>` for
values in `Debug`.

**Why.** 034 FR-025 and spec FR-023/FR-024; the redacted `Debug` follows `Issue`'s and 1185's
`FilterField`.

**Rejected.** Logging the scoped text for diagnostics: FR-024.

## R16 — A dev-only fake `gh` for the visual pass

**Decision.** `crates/micold-client/examples/fake_gh.rs` (new) is a `gh` stand-in for quickstart §B:
it answers `gh api graphql` for the list, search, labels, milestones and assignable-user documents
from a built-in repository of ~30 issues (labels `in-progress`, `blocked`, `bug`, `ui`, a label with
a space and one with a `"`; open and closed), honours `label:`, `-label:`, `is:open/closed`,
`no:assignee`, `author:`, `milestone:` and `sort:` well enough for the pass, logs each `q` to
stderr, and has a `MICOLD_FAKE_GH=reject` mode answering a GraphQL error at `search`. §B points
the env-include `PATH` at it.

**Why.** The cloud container has no signed-in `gh`, and §B must exercise the render glue;
`fake_jira.rs` set the precedent. SC-002's github.com comparison stays a by-hand step with a real
`gh` (§B, marked as needing one).

**Rejected.** Recording only against real GitHub: not runnable in the cloud session where the pass
is driven.
