# Contract: wire and readings (protocol 49)

**Feature**: 813 | Research: [R5, R6, R10](../research.md)

## W1 — `SessionSummary::hook_activity`

`SessionSummary` (`crates/micold-core/src/protocol/messages.rs`) gains `hook_activity:
Option<HookActivity>`, `#[serde(default)]`. `PROTOCOL_VERSION` 48 → 49 with a doc line naming
feature 813 (`protocol/version.rs`); `tests/schema_hash.rs` re-pinned. Every `SessionSummary { .. }`
literal sets `None`.

## W2 — Its value (service)

- `Some(..)` exactly while the session's `LiveSession::hook_activity` holds a todo list or a prompt;
  `None` for a session with no live entry and in `catalog.rs` `session_summary` (never on disk).
- Updated only from that session's own hook route (`/hook/<uuid>`); a body of another session
  never touches it (Principle II).
- The catalog is broadcast after a hook when `note_activity` **or** `note_hook_activity` returns
  `true`.

| Hook body | Effect on `hook_activity` |
|---|---|
| `UserPromptSubmit` with `prompt: "fix the bug\nmore"` | `prompt = "fix the bug"` |
| `PostToolUse`, `tool_name: "TodoWrite"`, `tool_input.todos` of 5 (2 `completed`, 1 `in_progress`) | `todos` replaced; `current()` = in-progress `activeForm`, ratio 2/5 |
| `PostToolUse`, other `tool_name` | unchanged, returns `false` |
| `TodoWrite` with malformed `todos` (not an array, item without `content`) | malformed items skipped; not an array → unchanged |
| any other event | unchanged |

## W3 — Privacy (FR-020)

- No log line contains a prompt, a todo text, or an issue title (daemon and client); `HookActivity`
  has no `Debug` used in logging (test: a log capture after hooks holds neither string).
- The daemon's MCP tools (`crates/micold-daemon/src/mcp/tools.rs`) never return `hook_activity`
  (test).
- The client never persists it (runtime field on `Session`, like `activity`).

## W4 — `Prompt::session` (client only)

`features/agent_confirm.rs` `Prompt` gains `session: SessionId`, set from
`DaemonMsg::ConfirmationRequested.caller` in `shell/daemon_sync.rs`. No wire change.

## W5 — Pull request status reading

`crates/micold-core/src/pull_request.rs`:

- `FRAGMENT` + `mergeable`; `PullRequestStatus::conflict: bool` (`CONFLICTING` → `true`, else
  `false`; missing field → `false`).
- `status_query(n)`: per branch also `oc{i}` and `mc{i}`:
  `pullRequests(headRefName: $b{i}, states: OPEN|MERGED, first: 100) { nodes { isCrossRepository } }`.
- `parse_status` → `StatusReading { statuses: BTreeMap<String, PullRequestStatus>, counts:
  BTreeMap<String, PrCounts> }`; `counts` has an entry for every asked branch; a missing or
  malformed `oc`/`mc` alias is a `ReadingFailure::Passing` like the other aliases.
- `read_outcome` and client `pr_status::Outcome::Ok` / `Msg::StatusesRead` carry `counts`;
  `State::counts` is cleared exactly when `statuses` is (`Unavailable`, `Released`, disabled).
- No new interval, trigger or `gh` call (FR-019): the same single `gh api graphql` per reading.

| Answer for branch `b` | `counts[b]` |
|---|---|
| `oc` 2 nodes (1 cross-repo), `mc` 3 nodes | open 1, merged 3 |
| `oc` 0, `mc` 0 | open 0, merged 0 |
| `mc` 100 nodes | merged 100, `merged_capped` |
| `oc` alias missing | whole reading `Passing` |
