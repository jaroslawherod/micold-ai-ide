# Contract: `create_worktree` tool (amends 034-daemon-mcp-server contracts/mcp-tools.md)

## Input

| Property | Type | Notes |
|---|---|---|
| `branch` | string, minLength 1 | Literal branch. No longer schema-required; required unless the derived inputs are used. |
| `name` | string, minLength 1 | With `branch`: the directory name (unchanged). Without `branch`: the description. |
| `mode` | `new_branch` (default) / `existing_local` / `track_remote` | Literal only. |
| `remote` | string, minLength 1 | Literal only; required with `track_remote`. |
| `type` | enum: `feat fix chore docs refactor test build ci perf style` | Derived only. |
| `ticket` | string | Derived only. Slugified; blank or empty slug means none. |
| `github_issue` | integer, minimum 1 | Derived only. Fills ticket, name and type; explicit `type`, `ticket`, `name` replace them. |

`required: []`. The description states the two alternatives, the combination rule, the branch/directory
shape (`${type}/${ticket}_${name}`, `${type}-${ticket}_${name}`), and that a name colliding with an
existing branch is refused with advice to use `branch` + `mode`.

## Refusals (all `invalid_input` unless noted; nothing is created)

| Condition | Reason |
|---|---|
| `branch`/`mode`/`remote` with `type`/`ticket`/`github_issue` | literal and derived inputs are alternatives |
| nothing given | say what to provide |
| `type` not in list | names the allowed values |
| derived, no type (after issue resolution) | "Select a type" |
| name slugifies to nothing | "Enter a name (letters or digits)" |
| derived branch fails ref check | "The resulting branch name is not valid" |
| `github_issue` not integer in 1..=2147483647 | invalid input |
| derived branch already exists | `conflict`: names the branch, retry with `branch` and `mode` `existing_local` or `track_remote` |
| derived directory taken | `conflict`: existing directory-taken text |
| issue lookup failure | the form's `IssueLoadError::message` text, plus not-open texts (issue-lookup.md) |

## Result

The `list_worktrees` row (ref, path, ...) plus `branch`, `directory`; derived requests also `type` and `ticket` (omitted when none).
