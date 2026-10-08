# Data Model: MCP create_worktree form inputs

All types are new unless marked existing. No persisted state is added.

## micold-core

| Type | Module | Shape |
|---|---|---|
| `CreateWorktreeRequest` | `mcp/tools.rs` | `Literal { branch: String, name: Option<String>, mode: CreateMode }` or `Derived { type_: Option<ConventionalType>, ticket: Option<String>, name: Option<String>, github_issue: Option<u32> }`. Invariant: built only by the parser; a `Derived` has at least one of the four fields (name, type, ticket or issue) and never a `branch`, `mode` or `remote`. |
| `Operation::CreateWorktree(CreateWorktreeRequest)` | `mcp/tools.rs` | replaces the `{ branch, name, mode }` variant (existing, reshaped) |
| `WorktreeNaming` | `naming.rs` (existing) | gains `overridden_by(type_, ticket, name)` |
| `naming_for_issue` | `naming.rs` | `(&Issue, &[LabelTypeEntry]) -> WorktreeNaming` |
| `IssueSnapshot` | `github.rs` | `{ number: u64, title: String, labels: Vec<String> }` of an issue known to be open |
| `IssueLookupError` | `github.rs` | `Load(IssueLoadError)` (existing enum, form's texts) or `NotOpenIssue { number, closed: bool }` |
| `IssueLookup` | `github.rs` | trait `read_issue(&self, &GithubRepo, u32) -> Result<IssueSnapshot, IssueLookupError>`; impls `GhCli`, `FakeIssueSource` |

`DerivedNames`, `NamingError`, `ConventionalType`, `LabelTypeEntry`, `GithubRepo`, `IssueLoadError` are existing and unchanged.

`naming_for_issue` takes `&Issue` today's type; `IssueSnapshot` converts into `Issue::new(number, title, labels, String::new())`, so the form and the tool call the same function with the same type.

## micold-daemon

| Item | Where | Note |
|---|---|---|
| `Catalog::label_mapping()` + `DaemonState::label_mapping()` | `catalog.rs`, `state.rs` | `Vec<LabelTypeEntry>` read from the settings store at call time; default when the store is `None` (`Catalog::ephemeral`). The accessor takes the `Inner` lock only to reach the store handle (or clones it out), reads the file outside the lock, and runs on the blocking pool |
| issue-lookup factory | `DaemonState` | field `Mutex<Option<Arc<dyn Fn() -> Result<Arc<dyn IssueLookup>, IssueLoadError> + Send + Sync>>>` (precedent: `first_prompt_bound: Mutex<Duration>`), set by `set_issue_lookup` for tests; `None` means production: build a `GhCli` per call from `locate_gh_on_host`, `Err(ToolMissing)` when none is found |

## Result row (JSON)

Existing `list_worktrees` row fields, plus `branch` (string), `directory` (string), and, for derived requests, `type` (string) and `ticket` (string, omitted when none).

## State transitions

None. A request resolves linearly: parse, policy, (lookup), derive, collision check, create, row.
