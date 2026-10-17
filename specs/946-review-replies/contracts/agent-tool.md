# Contract: the `report_review_comment` agent tool

A shipped tool of the daemon's agent tool server (feature 034, `specs/034-daemon-mcp-server/
contracts/mcp-tools.md`). Listed by `tools/list` after `send_session_input`. Annotations:
`readOnlyHint: false`, `destructiveHint: false`. Audited (034 FR-018) with target = the reference;
the reply is never logged. Never waits for the user (FR-003).

## Input schema

```json
{
  "type": "object",
  "properties": {
    "comment": { "type": "string", "minLength": 1,
                 "description": "The comment's reference from the review message, such as C-1a2b3c4d." },
    "outcome": { "type": "string", "enum": ["applied", "answered"],
                 "description": "applied: you made the change. answered: you did not, and the reply says why." },
    "reply":   { "type": "string",
                 "description": "What you did, or why not. Required for answered; at most 20000 characters." }
  },
  "required": ["comment", "outcome"]
}
```

## Description (catalog text)

"Report the outcome of one review comment the user sent you, by its reference: applied when you
made the change, or answered with a reply explaining why not. The comment's status and your reply
show next to it in the user's Changes view. Only comments of your own worktree (or of the project
root, for a session running there) can be reported."

## Order of checks

| # | Check | Error category | Message (substance) |
|---|---|---|---|
| 1 | `comment` present and a well-formed reference | `invalid_input` | "not a comment reference; references look like C-1a2b3c4d" |
| 2 | `outcome` is `applied` or `answered` | `invalid_input` | names the two values |
| 3 | `answered` with a missing or blank `reply` | `invalid_input` | "an answered comment needs an explanation in reply" |
| 4 | `reply` over 20,000 characters | `invalid_input` | "the reply is over the 20000-character limit" |
| 5 | reference names a comment of the caller's entry | — | continue |
| 5a | …else of another entry of the project | `refused_by_policy` | "that comment belongs to another worktree; a session reports only on its own entry's comments" |
| 5b | …else nowhere | `not_found` | "no such comment; it may have been cleared or discarded" |
| 6 | comment is inside an open send | `conflict` | "that comment is being sent and has not been delivered yet" |
| 7 | comment is a Draft | `conflict` | "that comment is still a draft; it has not been sent to you" |
| 8 | review file written | `service_error` on failure | nothing changes |

The caller's entry is its session's location: its worktree directory, or the Default entry (`""`)
for a session in the project root. A Default session may report (D5): a report changes no worktree,
so Principle III's limits on Default sessions do not apply.

## Success

Status and reply replace any earlier report (FR-005); `first_at` kept, `last_at` = now, `by` =
the calling session. `ReviewChanged` is pushed to every window attached to the project. Result:

```json
{ "comment": "C-1a2b3c4d", "status": "applied", "file": "src/a.rs", "lines": "12-14",
  "previous_status": "sent" }
```

`previous_status` is `sent`, `applied` or `answered`.

## Tests

Core (`crates/micold-core/tests/mcp_tools_catalog.rs`, `review/comment.rs` units): parsing,
validation 1–4, catalog listing and annotations, `REPORT_TOOL` equals the tool name, policy
`Proceed` for worktree and Default callers (`mcp_policy.rs`). Daemon
(`crates/micold-daemon/tests/review_reports.rs`, new, fake CLI): success pushes `ReviewChanged` and
persists across a daemon restart; 5a for a session of another worktree; a Default session reports
on the Default entry; 5b after Clear sent; 6 while a send is held open; 7 on a draft; a re-report
keeps `first_at`; the audit line names the reference and not the reply; no confirmation is asked.
