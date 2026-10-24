# Research: Advanced JQL in the Jira Issue Selector

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Each decision names what was chosen, why, and what was rejected. Paths are as of `origin/main`
0e5a1e9b; "new" marks a file or symbol this feature adds.

## R1 — A separate `jira_query` module beside `search_jql`

**Decision.** The advanced-mode rules (cleaning, the sort-clause split, the balance check, the scoped
query, Jira's error messages, the remembered state's format, the advanced load) live in a new core
module `crates/micold-core/src/jira_query.rs`, declared next to `pub mod jira_status;` in
`crates/micold-core/src/lib.rs`. `list_jql` and `search_jql` (`crates/micold-core/src/jira.rs`)
are not changed (ledger D3).

**Why.** #1185 (`specs/1185-jira-selector-filters`, branch `origin/claude/project-thread-xwkotj`)
adds filter clauses to `search_jql`/`list_jql`. Keeping the advanced builder in its own file means
neither feature edits the other's lines; feature 1101 set the precedent with `jira_status.rs`
beside `jira.rs`. The rules are decision logic, so they belong in the render-free core under
Principle I, not in the client.

**Rejected.** (a) A mode parameter on `search_jql`: the same function #1185 rewrites, and basic
mode adds `statusCategory != Done` and escapes the text as a string, which advanced mode must not.
(b) Building the JQL in the client's reducer: decision logic outside core, unreachable by
`mise run test-core`.

## R2 — One left-to-right scanner for quotes, parentheses and the sort clause

**Decision.** `jira_query::UserQuery::parse` walks the cleaned text once, keeping whether it is
inside a `"` or `'` string (a backslash inside a string takes the next character as text), the
parenthesis depth outside strings, and the byte offset of the first sort clause outside strings.
A sort clause is the word `ORDER` (any case), preceded by the start of the text or a character
that is not a letter, digit or `_`, followed by one or more whitespace characters and the word
`BY` (any case), followed by the end of the text or a character that is not a letter, digit or
`_`. The query is refused when it ends inside a string, when the depth ever goes below zero, or
when the depth is not zero at the end. The check runs over the whole query, condition and sort
clause both.

**Why.** The scope is `project = "<KEY>" AND (<condition>)`. The only ways a user text can leave
the parentheses are a `)` that closes them early (depth below zero), a `(` left open that swallows
the closing one, or a quote that hides one of ours. Those three checks are exactly FR-007's, and a
quote-aware scanner is the smallest thing that tells `summary ~ "a)"` from `a)`. Checking the
sort clause too closes `x ORDER BY created) OR (project = X`, which would otherwise append a
balanced-looking tail after our `)`. The word-boundary rule keeps `reorder by` and `order byline`
inside the condition.

**Rejected.** (a) A JQL parser: Jira is the judge of validity (spec Assumptions); a parser would
be a second, drifting grammar. (b) A regex for the sort clause: cannot skip quoted strings.
(c) Escaping the user's text as a string: it would stop being JQL.

## R3 — Cleaning and the 2,000-character limit

**Decision.** Before the scan: every `\n`, `\r` and `\t` becomes a space, every other control
character (`char::is_control`) is removed, and the result is trimmed. The limit is 2,000 `char`s
of the cleaned, trimmed text (clarify Q4). The cleaned text is what runs, what the input shows
after a run, and what is remembered.

**Why.** Matches `search_jql`'s cleaning order (clean, then cap) and FR-007's clarified count.
Counting `char`s, not bytes, makes the limit the one the user sees for non-Latin text.

**Rejected.** Cutting at 2,000 silently, as `search_jql` cuts at 100: a cut JQL query changes
meaning, so it is refused with the limit in the message (Edge "Very long query").

## R4 — Jira's error messages on HTTP 400

**Decision.** A `400` on an advanced request is read as Jira's refusal of the query. The body
(read up to 64 KiB) is parsed as `{"errorMessages": [string], "errors": {field: string}}`: the
`errorMessages` in Jira's order, then the `errors` values ordered by field name (the workspace's
`serde_json` has no `preserve_order`, so a map's order is its keys'). Each message is cleaned of
control characters (R3's rule), trimmed, dropped when empty, and cut at **300 characters** with
`…` appended when cut; at most **10** messages are kept. A body that is not that JSON, or carries
no message, gives the plain "not valid JQL" text (FR-010).

**Why.** Jira's usual JQL messages ("Field 'asignee' does not exist or you do not have permission
to view it.", "The value 'XYZ' does not exist for the field 'project'.", "Error in the JQL Query:
Expecting ')' before the end of the query.") run 50 to 180 characters; 300 keeps them whole while
keeping a pathological body from filling the dialog. `REASON_MAX_CHARS` (80,
`crates/micold-core/src/jira.rs`) is the precedent for a fixed cap but too short here (clarify Q5).
In basic mode a `400` stays `NoAccess` (`classify`), unchanged: there the app wrote the JQL.

**Rejected.** (a) Showing the raw body: may be HTML or huge. (b) One joined string: the spec asks
for the messages "in Jira's order", and a list renders one per line.

## R5 — A new `JiraSource` method, and the 400 body in the adapter

**Decision.** `JiraSource` (`crates/micold-core/src/jira.rs`) gains one method with a default body:

```rust
fn query_page(&self, conn: &JiraConnection, token: &ApiToken, jql: &ScopedQuery, cursor: Option<&str>)
    -> Result<IssuePage, QueryFailure>;
```

The default answers `QueryFailure::Load(IssueLoadError::Other(..))`, so a source that does not
implement it (the `Recording` source in `main_tests.rs`) still compiles. `JiraCloud`
(`crates/micold-client/src/shell/jira_http.rs`) implements it with the same `POST
/rest/api/3/search/jql`, the same `FIELDS`, `PAGE_SIZE` (100) and headers, and maps a `400` to
`QueryFailure::Rejected(jira_query::rejection_messages(&body))`. To get the body, the private
`PostError::BadRequest` becomes `PostError::BadRequest(Vec<u8>)`, read with a 64 KiB bound; the
status path (`read_chunk`) ignores it as it does today. `FakeJiraSource` gains `with_query`,
`query_calls` and the method's scripted answer, added at the end of its fields and impl.

**Why.** The paging and the cap are core's (`load_pages`), so the source answers one page, as
`list_open` does. Taking a `ScopedQuery` (data-model §3, constructible only from a parsed
`UserQuery`) and not a `&str` makes "a refused query is never sent" a type property (Principle V).
A default body keeps the trait change additive for #1185, which also adds methods to this trait.

**Rejected.** (a) Reusing `search_open` with the JQL as `text`: it builds `search_jql` from its
argument and returns at most 50. (b) Mapping the `400` to an `IssueLoadError` variant: that enum
is GitHub's and Jira's shared notice vocabulary (`crates/micold-core/src/github.rs`), and a query
error is advanced mode's only.

## R6 — The advanced load: `load_pages` generic over its error, plus the FR-008 filter

**Decision.** `jira_query::load(source, conn, token, &ScopedQuery) -> Result<IssueListing,
QueryFailure>` calls `micold_core::github::load_pages` with a closure over `query_page`, and that
closure keeps only the page's issues whose key is `<KEY>-<digits>` of the connected project
(`Issue::key()`), before the page is counted. `load_pages` becomes generic over the error type
(`impl FnMut(Option<&str>) -> Result<IssuePage, E>` → `Result<PagedIssues, E>`); its body and its
existing callers are unchanged (they infer `E = IssueLoadError`). `complete` is `exhausted`, so
FR-006's "more issues match" caption has the same source as basic mode's.

**Why.** One paging and cap rule for every provider (533 R1). Filtering inside the fetch keeps the
1,000 cap counting only listable issues. A page made only of other projects' keys (impossible
under the scope, short of a Jira bug) then ends the load, which fails closed.

**Rejected.** A second paging loop in `jira_query.rs`: duplication `mise run duplication` budgets.

## R7 — Where the remembered query and mode live

**Decision.** A small owner-only file per project beside the token:
`<data_dir>/jira/<project id>.query.json` (`micold_core::store::jira_token_dir()`,
`project_id_of`), holding `{"mode":"basic"|"advanced","query":"<last run query>"}`. Core owns the
format (`jira_query::RememberedQuery`, `to_json`, lenient `from_json`); the client's shell reads it
when the form opens and writes it (read-modify-write of one field, `owner_only::write`, atomic
rename) at each mode switch and each run. `shell::jira::apply`'s `ConnectionAction::Remove`
deletes it with the token (FR-014).

**Why.** FR-013 asks for per-user local data, not a project file or `settings.json`. The token
directory is already the per-user, per-project, owner-only home of this connection, and removing
the connection already deletes a file there. Reading at each open (not caching at start-up) makes
two windows' last write win (Edge "Two windows") and leaves an open form's input alone.

**Rejected.** (a) A field in the project's state file `projects/<id>.json`, as `issue_source`
(feature 533 FR-016): the daemon writes that file too, so the field would need its own merge line
in `JsonFileStore::save`, a `Workspace` map and the fake store; and clearing it on connection
removal reaches into the store. (b) `settings.json`: forbidden by FR-013, and synced to every
daemon. (c) Memory only: fails "in a later app run".

## R8 — The selector: a mode switch of `ToggleChip`s and the shared `Typeahead`, given an `on_submit`

**Decision.** With the Jira source showing, a **Basic / Advanced** pair of `ToggleChip`s (as
`provider_switch` in `crates/micold-client/src/ui/worktree_form.rs`) sits above the picker. In
advanced mode the picker is the same `material::Typeahead`, its text the draft, its rows the
results in Jira's order (no local ranking), with a new builder method
`.on_submit(message)` that the Typeahead hands to its `TextField` (which already has
`on_submit`). Enter with a highlighted row still picks the row (the list claims Enter,
`crates/micold-client/src/ui/cdk/picker.rs`); Enter with none runs the query. A **Run**
`Button::text` beside the field is FR-004's run control. Query errors and refusals render as
error-tinted `Text` lines in place of the list.

**Why.** Principle VIII: the row form (key, status chip, summary, labels, reporter, tooltip) is the
picker's, and the Typeahead is the only control that shows it; extending it with one optional
builder method is the "extend the shared primitive" route. `on_submit` is useful beyond this
feature (any search that runs on Enter).

**Rejected.** (a) A `TextArea` for multi-line JQL: its Enter inserts a line (Ctrl+Enter submits),
against FR-004, and a pasted newline is already cleaned to a space. (b) A bespoke list widget under
a plain `TextField`: forks the picker row. (c) A second Typeahead builder in the feature module:
the Component-reuse gate's one-off.

## R9 — State, messages and requests in the client

**Decision.** A new render-free module `crates/micold-client/src/features/worktree_form/jira_query.rs` holds
`JiraQuery` (mode, draft, last run query, run state) and its reducers; `WorktreeForm` gets one
field `jira_query: JiraQuery`, and `worktree_form::Msg` one variant `JiraQuery(jira_query::Msg)`.
A run is numbered from the form's existing `issue_request_seq`, so every reset that already bumps it
(source to GitHub, settings change, close) also makes an in-flight run stale (Edge "Running a new
query while one is in flight"). The shell's new `crates/micold-client/src/shell/jira_query.rs`
starts a run through the existing gate `jira_request(app)?.run(..)` (`shell/issues.rs`), and
reads and writes the remembered file. Persisting is an `Outcome` the shell acts on, as
`Outcome::IssueSourceChosen` is.

**Why.** Keeps #1185's and this feature's edits to `features/worktree_form.rs` and
`ui/worktree_form.rs` to a handful of hook lines each (plan §Overlap with #1185). The one gate keeps
`jira_is_requested_only_on_named_events.rs`'s "the source is asked for once" true.

**Rejected.** Fields spread over `WorktreeForm` next to `issue_query`: the struct #1185 extends
with its filters, so every field would be a rebase conflict.
