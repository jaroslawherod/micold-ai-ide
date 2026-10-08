# Research: MCP create_worktree form inputs

## D1. Where the GitHub issue lookup runs

**Decision**: in the daemon process (`micold-daemon/src/mcp/tools.rs`), built from `micold-core` types.

**Finding**: the planning concern assumed the lookup lives in `micold-client`. `grep` shows it does not: `GhCli`, `IssueSource`, `GithubRepo`, `choose_remote`, `locate_gh_on_host`, `classify`, `IssueLoadError::message` are in `crates/micold-core/src/github.rs`; `type_for_labels` in `issue_types.rs`; `name_from_title` and `derive` in `naming.rs`. `micold-client` holds only the form glue (`shell/issues.rs`: paging, debounce, tasks; `features/worktree_form.rs`: state). The daemon already depends on `micold-core`.

**Rejected**:
- Move the issue source to a new shared crate: nothing to move; a crate for no gain.
- Daemon asks a connected client to run the lookup: the client may be absent, there can be several windows, and the MCP call would depend on UI state. Also needs a new protocol message.
- A second `gh` wrapper in the daemon: violates the spec's "reuse the form's issue source".

## D2. How a single issue is read

**Decision**: add a trait `IssueLookup { fn read_issue(&self, repo, number) -> Result<IssueSnapshot, IssueLookupError> }` in `github.rs`, implemented by `GhCli` (so it shares `GH_TIMEOUT`, `run_bounded`, `classify` and the environment of the form's runs) and by `FakeIssueSource` (scripted).

`IssueSource` has no single-issue read (list/search/describe only). `SEARCH_WITH_NUMBER_QUERY` does fetch `repository.issue(number)` but with search noise, and its parser drops `state`. A dedicated GraphQL document is smaller and exact:
`query($owner: String!, $name: String!, $n: Int!) { repository(owner: $owner, name: $name) { issue(number: $n) { number title state labels(first: 20) { nodes { name } } } } }`, reusing the `issue_node_selection!` label selection (first 20 labels, same as the form).

**Classification** (by data shape, not by `errors[].type`, because `NOT_FOUND` is reported for both an unreadable repository and a missing issue). `gh` exits non-zero on any GraphQL error, and a missing issue arrives as `issue: null` plus a NOT_FOUND error, which `graphql_error_of` maps to `NoAccess`. So `parse_lookup` inspects `data.repository` / `repository.issue` first and consults `graphql_error` only when `data` gives no answer; it returns `Result<Result<IssueSnapshot, NotOpen>, IssueLoadError>`, and `read_issue` reuses `GhCli::run` with `T = Result<IssueSnapshot, NotOpen>`: `run` already calls `parse` on non-zero exit and falls back to `classify` only on `Err(Other)`, so `parse_lookup` must return `Ok(Err(NotOpen))` for NOT_FOUND + `issue: null` (a test pins it). Fixtures: exit 1 + `issue: null` + NOT_FOUND; `repository: null` + NOT_FOUND.
- `data.repository` null: no access (existing `NoAccess` text).
- `repository.issue` null: not an open issue (a pull request number, a missing number): `NotOpenIssue`.
- `state == "CLOSED"`: `NotOpenIssue` with the closed wording.
- `state == "OPEN"`: the snapshot.
- Transport failures: the existing `classify`, so the texts are the form's.

**Spec note**: the form lists only open issues, so it has no text for closed/PR/missing. Those three texts are new (one plain sentence each, naming the number and repo). Every load failure reuses `IssueLoadError::message` verbatim.

**Rejected**: adding `read_issue` to `IssueSource` (forces every implementor and test double to change for a method the form never calls); `gh issue view --json` (a second output format and parser; cannot tell a PR from a missing issue as cleanly, and the form already uses `gh api graphql`).

Number type: `-F n=<number>` as in `search_args`' typed number; `github_issue` is bounded to GraphQL `Int` (<= 2147483647), larger is `invalid_input`.

## D3. Where the label mapping comes from, and when

**Decision**: `Catalog::label_mapping()` (new, `micold-daemon/src/catalog.rs`) calls `settings_store.load().settings.issue_label_types` on each call, falling back to `default_mapping()` when there is no store. Read at call time satisfies FR-005.

**Why not `Catalog.settings`**: that copy is loaded at daemon start and refreshed only for the service-owned fields; `persist_service_settings` re-reads the file precisely because the client owns the other fields (including `issue_label_types`). A copy would be stale after the user edits the mapping.

**Rejected**: adding the mapping to `DaemonSettings` / `SettingsSet` (the catalog comment says the daemon must not become the authority on client-owned fields); reading it from the client (D1).

A corrupt or missing file yields `Settings::default()` with the default mapping (`SettingsStore::load` never fails), which is the spec's stated fallback.

## D4. Resolution and overrides, shared with the form

**Decision**: two small pure functions in `naming.rs`:
- `naming_for_issue(issue: &Issue, mapping) -> WorktreeNaming` : ticket = number, name = `name_from_title(title)`, type = `type_for_labels(mapping, labels)`.
- `WorktreeNaming::overridden_by(self, type_, ticket, name) -> WorktreeNaming`: an explicit value replaces the issue's when present. A `ticket` or `name` that is blank after trimming counts as not passed (so `ticket: ""` with an issue keeps the issue number; without an issue it means no ticket, FR-002). An explicit `name` is never shortened.

`worktree_form::issue_picked` is changed to call `naming_for_issue`, so form and tool share one resolution (FR-016) and a client test pins them equal. Behaviour of the form does not change.

**Rejected**: duplicating the three-line resolution in the daemon (the drift FR-016 forbids).

## D5. Request shape

**Decision**: `Operation::CreateWorktree` carries `CreateWorktreeRequest::{Literal{branch,name,mode}, Derived{type_,ticket,name,github_issue}}` built by the core parser. Mixed input is refused there (FR-009), so downstream code cannot see a mixed request.

**Rejected**: keep the flat struct with all-optional fields and check in the daemon (the invalid state stays representable; policy and audit code must each re-check).

`remote` without `mode: track_remote` is accepted today only as before (unchanged); `remote` alongside derived inputs is refused (FR-009).

## D6. Collision handling

**Decision**: reuse `ops::branch_situation`, but not `is_compatible_with`: `(NewBranch, RemoteOnly)` is compatible there (`worktree.rs:802`) and would create silently at HEAD. A derived request accepts only `BranchSituation::Free`; `LocalAvailable`, `RemoteOnly` and `Blocked` map to the FR-011 refusal; `DirectoryTaken` keeps its own text; `preflight` returns `DirectoryTaken` first (`worktree.rs:952`), so on `DirectoryTaken` the daemon probes the branch alone with `Git::branch_exists` and gives the FR-011 text when it exists (the common repeat-the-same-call case), else the directory text. The concurrent case is already serialised by `worktree_gate`; the loser's `git worktree add` fails the preflight and is reported by `create_failure`.

**Rejected**: auto-suffixing the name (silently different from the request; the spec says refuse); an overwrite mode (out of scope).

## D7. Locating `gh` in the daemon

**Decision**: `locate_gh_on_host(None, $PATH)`. `candidate_dirs` adds the OS's well-known install directories, so a daemon started from a service with a thin `PATH` still finds a normal `gh`. The client's env-include `PATH` is not available to the daemon and is not needed for the common case. If `gh` is not found: `IssueLoadError::ToolMissing` text. A sandboxed daemon (docs/user-guide/sandboxed-daemon.md) may have no `gh` or no network; it then gets the same plain-language refusal, not a hang (10 s bound).

**Rejected**: honouring the env-include script's `PATH` in the daemon (it is resolved by the client; extra coupling for an edge case; revisit if the quickstart shows a real failure).

## D8. Result fields for literal calls

`branch` and `directory` are known for every call and are added to the row. `type` and `ticket` are added only for derived requests; deriving them from a foreign literal branch would invent values. This is the "additional derived fields" US4 allows.
