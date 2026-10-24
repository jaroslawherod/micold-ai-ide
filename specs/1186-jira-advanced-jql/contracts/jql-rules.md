# Contract: the advanced-JQL rules (`micold_core::jira_query`)

**Feature**: [spec.md](../spec.md) | Research [R1](../research.md#r1--a-separate-jira_query-module-beside-search_jql), [R2](../research.md#r2--one-left-to-right-scanner-for-quotes-parentheses-and-the-sort-clause), [R3](../research.md#r3--cleaning-and-the-2000-character-limit), [R4](../research.md#r4--jiras-error-messages-on-http-400) | Data model [§1–§4](../data-model.md)

Pure functions, no I/O. Tested in `crates/micold-core/tests/jira_query.rs` (new).

## 1. Cleaning (FR-007, Edge "Control characters")

`\n`, `\r`, `\t` → one space each; any other `char::is_control` removed; then trimmed. Nothing else
changes: no case folding, no space collapsing.

| Raw | Cleaned |
|---|---|
| `"  type = Bug\n"` | `type = Bug` |
| `"a\u{0007}b"` | `ab` |
| `"a\tAND\nb"` | `a AND b` |

## 2. Parse: limit, quotes, parentheses, sort clause

Order: clean; refuse `TooLong` when the cleaned text has more than 2,000 `char`s; scan once.

Scanner state: `quote: Option<char>`, `escaped: bool`, `depth: i32`, `sort_at: Option<usize>`.

- Outside a string: `"` or `'` opens one; `(` adds 1; `)` subtracts 1 and `UnbalancedParentheses`
  when below 0; while `sort_at` is `None` and `depth` is 0, a sort clause starting here sets it (an
  `ORDER BY` inside parentheses stays in the condition, so the split never cuts a group).
- Inside a string: `\` sets `escaped` (the next char is text); the opening quote char, unescaped,
  closes it; anything else is text.
- End: inside a string → `UnclosedQuote`; `depth != 0` → `UnbalancedParentheses`.

Sort clause: `ORDER` (ASCII case-insensitive), the char before it absent or not `[A-Za-z0-9_]`, one
or more whitespace chars (after cleaning: spaces), `BY` (case-insensitive), the char after it
absent or not `[A-Za-z0-9_]`.

| User query | Result |
|---|---|
| `` (empty) | ok, condition empty, no sort |
| `assignee = currentUser() AND priority = High` | ok, no sort |
| `status = "In Review" ORDER BY priority DESC, created ASC` | condition `status = "In Review"`, sort `ORDER BY priority DESC, created ASC` |
| `ORDER BY created ASC` | condition empty, sort `ORDER BY created ASC` |
| `type = Bug order  by created` / `type = Bug ORDER` ⏎ `BY created` (newline cleaned) | sort at `order` / `ORDER` |
| `summary ~ "order by"` / `summary ~ 'order by'` | no sort |
| `reorder by = 1` / `x ORDER BYTES` | no sort |
| `(a ORDER BY b) OR (project = X)` | no sort: all condition, scoped inside `AND (…)` |
| `summary ~ 'a)'` / `summary ~ "a\"b)"` | ok |
| `summary ~ "abc` | `UnclosedQuote` |
| `x = 1) OR project = OTHER OR (x = 1` | `UnbalancedParentheses` (depth −1) |
| `(x = 1` | `UnbalancedParentheses` (depth 1 at end) |
| `x = 1 ORDER BY created) OR (project = X` | `UnbalancedParentheses` |
| 2,001 × `a` | `TooLong` |
| 2,000 × `a` followed by `\n` | ok (trimmed to 2,000) |

## 3. The scoped query (FR-005)

`ScopedQuery::new(key, query)`:

| Condition | Sort | JQL |
|---|---|---|
| non-empty `C` | user's `S` | `project = "KEY" AND (C) S` |
| non-empty `C` | none | `project = "KEY" AND (C) ORDER BY updated DESC` |
| empty | user's `S` | `project = "KEY" S` |
| empty | none | `project = "KEY" ORDER BY updated DESC` |

Examples (key `ABC`): US1 AS2 → `project = "ABC" AND (assignee = currentUser() AND priority =
High) ORDER BY updated DESC`; US1 AS3 → `project = "ABC" AND (status = "In Review") ORDER BY
priority DESC, created ASC`. No `statusCategory` clause is ever added (US1 AS5).

## 4. Jira's messages (FR-010)

`rejection_messages(body: &[u8]) -> Vec<String>`: `errorMessages` strings in order, then `errors`
object values (strings) in key order; each cleaned (§1), trimmed, empty ones dropped, cut at 300
`char`s plus `…` when longer; at most 10 kept. Not JSON / no such fields → `vec![]`.

| Body | Messages |
|---|---|
| `{"errorMessages":["Field 'asignee' does not exist or you do not have permission to view it."],"errors":{}}` | that one |
| `{"errorMessages":[],"errors":{"jql":"Expecting ')' before the end of the query."}}` | that one |
| `{"errorMessages":["a","b"]}` | `a`, `b` |
| `<html>…` / `{}` | none → plain text |
| 12 messages / one of 400 chars | first 10 / 300 chars + `…` |

## 5. Texts

| Case | Text |
|---|---|
| `UnclosedQuote` | `The query has a quote that is not closed. Close it and run the query again.` |
| `UnbalancedParentheses` | `The query's parentheses do not match. Fix them and run the query again.` |
| `TooLong` | `The query is longer than 2,000 characters. Shorten it and run the query again.` |
| `Rejected([])` | `Jira did not accept the query: it is not valid JQL.` |
| Empty result (FR-012) | `No issue matches this query.` |
| Prompt (FR-004a) | `Type a JQL query and press Enter to list issues.` |
| Cap (FR-006) | `Showing the first 1,000 issues this query returns.` |

These are constants of `jira_query`, so the reducer tests and the view share one spelling. None of
them contains the user query (FR-016).

## 6. The advanced load (FR-006, FR-008)

`load(source: &dyn JiraSource, conn, token, scoped: &ScopedQuery) -> Result<IssueListing, QueryFailure>`:
`load_pages(|cursor| source.query_page(conn, token, scoped, cursor).map(keep_project(key)))`,
where `keep_project` retains issues whose `key()` is `<conn.project_key>-<digits>`. Cap 1,000
(`ISSUE_LOAD_CAP`), `complete` = exhausted, `total_open` = issues held. Errors pass through
unchanged; the first aborts the load (533 rule: a partial list is never shown as the list).
