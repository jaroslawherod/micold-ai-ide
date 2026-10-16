# Contract: `create_worktree` result for a `github_issue` call

Extends 550 contracts/create-worktree-tool.md. A call **without** `github_issue` returns exactly what
it returns today and makes no details read (FR-010, US1-6). A refused `github_issue` call (not an
open issue, collision, invalid name) returns its error as today and carries no details.

## Shape

A successful `github_issue` call adds one key, `issue`, to the result object, beside `branch`,
`directory`, `type`, `ticket` and the optional session keys:

```json
"issue": {
  "number": 12,
  "blocked": true,
  "blocked_by": { "open": 1, "total": 2, "hidden": 0, "more": 0,
                  "items": [ {"ref": "#10", "title": "Parser rewrite", "state": "open"},
                             {"ref": "#9",  "title": "Old spike",      "state": "closed"} ] },
  "blocking":   { "total": 1, "hidden": 0, "more": 0, "items": [ {"ref": "#14", "title": "…", "state": "open"} ] },
  "parent":     { "ref": "acme/roadmap#3", "title": "Q4 epic", "state": "open" },
  "sub_issues": { "closed": 1, "total": 2, "hidden": 0, "more": 0, "items": [ … ] },
  "labels":     { "total": 2, "more": 0, "items": ["bug", "parser"] },
  "assignees":  { "total": 1, "more": 0, "items": ["octocat"] },
  "milestone":  "v1.4",
  "projects":   [ { "name": "Roadmap", "fields": { "Status": "In progress", "Estimate": "3" } } ],
  "closing_pull_requests": { "total": 1, "more": 0, "items": [ {"ref": "#20", "title": "…", "state": "draft"} ] },
  "linked_pull_requests":  { "total": 1, "more": 0, "items": [ {"ref": "#21", "title": "…", "state": "merged"} ] },
  "not_readable": { "projects": "your GitHub sign-in can't read projects (it lacks the read:project scope)" }
}
```

## Rules

- **Always present**: `number` (the identification FR-004 permits). Everything else follows the
  rules below.
- **Omitted kinds** (FR-004): a kind that was read and has nothing in it is left out entirely. An
  issue with no relationships and no optional metadata yields `"issue": {"number": 12}`.
- **`blocked`**: present, and `true`, only when `blocked_by.open > 0`. Never `false`, because an
  absent key already says it (SC-003).
- **Lists**:
  - `items` has at most 20 entries.
  - `more = total - len(items)`.
  - `hidden` (issue lists) counts the items the sign-in cannot see. They are included in `total`
    (and `open`). It is omitted when 0.
  - `"total_is_lower_bound": true` appears on `linked_pull_requests` only when the timeline was not
    read to its end (research R4).
- **States**: `ref` is `#N` or `owner/name#N`. Issue `state` is `open` or `closed`; PR `state` is
  `open`, `draft`, `merged` or `closed`.
- **`projects`**: each project lists only non-empty fields. A project with none has
  `"fields": {}`. Projects past 20 add `"projects_more": N`.
- **`not_readable`**: maps a kind's key to its plain reason (FR-007). A kind listed there does not
  also appear as data.
- **Whole-read failure** (timeout, rate limit, network, no `gh`): `"issue": {"number": 12, "details_error": "<IssueLoadError::message(repo)>"}`,
  for example "GitHub didn't answer within 10 seconds". The worktree has already been created
  (US3-2, FR-006).
- **Plain text**: all strings are `PlainText` (FR-013).

## Timing

The call waits for the read at most `details_bound` (10 s) after the read started, which is just
after the remote was resolved. In practice it waits only for whatever part of the read outlasts the
create (FR-011, SC-005).

## Published text (FR-014)

- **Tool description**: append "A github_issue call's result also carries issue: what blocks the
  issue and what it blocks, its parent and sub-issues, labels, assignees, milestone, project fields
  and linked pull requests (only the kinds it has; a kind that cannot be read is listed under
  not_readable; details_error when none could be read). Being blocked is reported, never enforced."
- **`github_issue` schema description**: append "The result's issue object reports its
  relationships."
