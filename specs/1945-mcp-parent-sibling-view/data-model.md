# Data Model: 1945

No persisted state changes. New in-memory types in `micold-core::worktree_parent`:

| Type | Fields | Rule |
|---|---|---|
| `Neighbourhood` | `parent: Option<ParentSide>`, `siblings: Vec<String>`, `children: Vec<Child>` | Worktree names are directory names; `default` is the name `"default"`. `siblings` is empty when `parent` is `None` or `Removed`. Sorted by name. |
| `ParentSide` | `Known { name, recorded: bool }` \| `Removed { last_name }` | `recorded == false` is the implicit `default`. A removed parent has no sessions and no siblings. |
| `Child` | `name: String`, `implicit: bool` | `implicit` only for unparented children of `default`, so only when the caller is in `default`. |

`neighbourhood(live, views, own)` is total and pure. `own: None` is `default` (no parent). An `own`
missing from `live` returns the empty `Neighbourhood`.

`Operation` (core, `mcp/tools.rs`) gains `Neighbourhood` (no fields) and `SendParentInput { text:
NonEmptyText }`. `StartDefinition` (daemon `ops.rs`) gains `parent: Option<&ParentChoice>`.

Parent target: a function `parent_target(sessions: &[SessionSummary], parent_dir: Option<&str>) ->
Option<SessionId>`: first `Running` session with `worktree_dir == parent_dir`.
