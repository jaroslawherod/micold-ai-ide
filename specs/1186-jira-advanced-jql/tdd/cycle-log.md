# TDD cycle log: 1186

The feature has no `tdd/test-list.md` (as 482, 837, 946 and 972): tasks.md's test tasks are the
list, and each row names the task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `jira_query` module, `lib.rs` declaration, empty test file | structural | `cargo test -p micold-core --test jira_query` compiles |
| T002–T004 (red), T005–T006 | cleaning, `UserQuery::parse` (sort split outside strings, word-bounded `ORDER BY`, quotes, parentheses, 2,000-char limit, precedence), `ScopedQuery::new` (four §3 rows, US1 AS2/AS3, no `statusCategory`, equality), the §5 texts, `QueryMode` default/serde | `cargo test -p micold-core --test jira_query`: `error[E0432]: unresolved imports micold_core::jira_query::QueryRefusal, …ScopedQuery, …UserQuery, …CAPPED, …EMPTY, …NOT_VALID_JQL, …PROMPT` | `jira_query` 17 passed (18 with the `QueryMode` test) |
| T007 | `load_pages` generic over its error (signature only) | structural, behaviour-preserving | `jira_load` 8 passed unchanged |
| T008 | `JiraSource::query_page` default body; `FakeJiraSource::with_query`/`query_calls` | seam for T009 | compiles; covered by T009's `an_unscripted_query_answers_a_load_failure` |
| T009 (red), T010 | `jira_query::load`: cursor paging in Jira's order, 1,000 cap with `complete == false`, exhausted → complete, other projects' keys dropped (`XYZ-1`, `ABC-`, `ABCD-3`, `ABC-1x`, `abc-4`, keyless), Done kept, first error aborts, `Rejected` passes through, scoped JQL recorded | `cargo test -p micold-core --test jira_query`: `error[E0432]: unresolved import micold_core::jira_query::load`; mutant (the `retain` in `keep_project` removed): `issues_of_another_project_are_dropped_and_moved_issues_kept ... FAILED` | `jira_query` 25 passed |
| T011 (red), T012 | `JiraCloud::query_page`: basic POST with the scoped JQL, basic fields, `maxResults` 100, `nextPageToken` only with a cursor, four headers; 2xx parses as `list_open`; 401/429/not-Jira/timeout → `QueryFailure::Load` as basic | `cargo test -p micold-client --test jira_http_adapter`: 5 FAILED, e.g. `left: Err(Load(Other("this Jira source cannot run a query"))) right: Err(Load(CredentialsRefused))` (the trait's default body) | `jira_http_adapter` 18 passed |
