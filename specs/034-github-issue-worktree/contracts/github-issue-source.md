# Contract: GitHub issue source (`micold_core::github`)

**Feature**: [spec.md](../spec.md) · Research [R1](../research.md#r1--how-github-is-reached-the-github-cli-gh-as-a-subprocess)–[R8](../research.md#r8--classifying-gh-failures-into-fr-007s-plain-language-reasons)

The boundary between the application and GitHub. Everything that decides something is a pure
function tested in `micold-core`; the trait implementation only runs `gh` and returns its bytes.

## 1. Locating `gh` (FR-026, research R3)

```rust
pub enum HostOs { Linux, MacOs, Windows }
impl HostOs { pub fn current() -> HostOs }          // the one cfg

impl HostOs {
    pub fn exe_name(self) -> &'static str;          // "gh" | "gh.exe"
    pub fn path_separator(self) -> char;            // ':' | ';'
}

pub struct LocateInputs<'a> {
    pub os: HostOs,
    pub env_include_path: Option<&'a str>,          // PATH/Path from the env-include snapshot (key matched ignoring case)
    pub process_path: &'a str,
    pub home: Option<&'a Path>,
    pub env: &'a dyn Fn(&str) -> Option<String>,    // ProgramFiles, LOCALAPPDATA, …
    pub exists: &'a dyn Fn(&Path) -> bool,          // production: Path::is_file; tests: a fake set
}

pub fn candidate_dirs(inputs: &LocateInputs) -> Vec<PathBuf>;   // pure, ordered, deduplicated
pub fn locate_gh(inputs: &LocateInputs) -> Option<PathBuf>;      // first dir where exists(dir/exe_name)
```

- Order: env-include `PATH` entries, then process `PATH` entries, then the OS's well-known
  directories (research R3 table). Duplicates keep their first position; empty components are
  dropped. Splitting uses `os.path_separator()`, never the host's `split_paths`, and the file name
  is `os.exe_name()`, never the host's `PATHEXT` — so every OS's behaviour is testable on every
  host (research R3).
- The shell supplies `HostOs::current()`, the real variables, and `Path::is_file`.
- **Tests** (`crates/micold-core/tests/github_locate.rs`, all against a fake `exists`): each
  `HostOs` value's table, separator and executable name, on every host; env-include before process
  `PATH`; process `PATH` before well-known; `gh` only in `/opt/homebrew/bin` found for `MacOs` with
  the Dock `PATH` `/usr/bin:/bin:/usr/sbin:/sbin`; `gh.exe` only under `%LOCALAPPDATA%\Microsoft\WinGet\Links`
  found for `Windows`; a `Path` (not `PATH`) key from env-include honoured; none → `None`.

### Where the env-include `PATH` comes from (research R3 step 1)

The load task receives the cached snapshot for the project root when `App::env_include_cache` holds
one. On a miss it calls `env_include::snapshot_for(caps.env_include(), enabled, script, timeout,
project_root)` inside the same `spawn_blocking`, returns the snapshot with the load result, and the
shell inserts it into the cache. The located `gh` path is returned too and kept in
`IssueList::Loaded` for that load's searches.

## 2. The trait

```rust
pub trait IssueSource {
    /// One page of open issues, most recently updated first.
    fn list_open(&self, repo: &GithubRepo, cursor: Option<&str>) -> Result<IssuePage, IssueLoadError>;
    /// Open issues matching `text` on GitHub's side (FR-005a). Unfiltered here.
    fn search_open(&self, repo: &GithubRepo, text: &str) -> Result<Vec<Issue>, IssueLoadError>;
}

pub struct GhCli { gh: PathBuf }                 // production; built with the located path
pub struct FakeIssueSource { … }                 // scripted pages / errors; records every call
```

`GhCli` is constructed per load by the client shell from `locate_gh(…)`; when that is `None`
the shell reports `IssueLoadError::ToolMissing` without constructing it.
`Capabilities` holds an `Arc<dyn Fn(PathBuf) -> Arc<dyn IssueSource + Send + Sync>>` factory, so
`no_concrete_implementations` still sees `GhCli` named only in `Capabilities::real()`.

## 3. What `GhCli` runs

```text
<gh> api graphql --hostname github.com \
     -f query=<LIST_QUERY> -f owner=<owner> -f name=<name> [-f cursor=<cursor>]
```

Every string variable uses `-f` (raw string). `-F` is used only for the integer `n` below: `-F`
converts `true`/`false`/`null`/digits and reads `@file`, so a repository named `1` or `true` would
become a type error under it.

```graphql
query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    issues(states: OPEN, first: 100, after: $cursor,
           orderBy: {field: UPDATED_AT, direction: DESC}) {
      totalCount
      pageInfo { hasNextPage endCursor }
      nodes { number title updatedAt labels(first: 20) { nodes { name } } }
    }
  }
}
```

Search (FR-005a), one request; `$q` = `repo:<owner>/<name> is:issue is:open <text>`. Two query
documents, because GraphQL rejects a declared variable the query does not use:

```graphql
# SEARCH_QUERY — any text                              (-f q=…)
query($q: String!) {
  search(type: ISSUE, query: $q, first: 50) {
    nodes { ... on Issue { number title updatedAt state labels(first: 20) { nodes { name } } } }
  }
}

# SEARCH_WITH_NUMBER_QUERY — text is `N` or `#N`, N a u64 fitting Int   (-f q=… -f owner=… -f name=… -F n=N)
query($q: String!, $owner: String!, $name: String!, $n: Int!) {
  search(type: ISSUE, query: $q, first: 50) {
    nodes { ... on Issue { number title updatedAt state labels(first: 20) { nodes { name } } } }
  }
  repository(owner: $owner, name: $name) {
    issue(number: $n) { number title updatedAt state labels(first: 20) { nodes { name } } }
  }
}
```

**Partial responses.** For a number that does not exist, or that is a pull request's, GitHub
answers `data.repository.issue = null` with an `errors[]` entry of type `NOT_FOUND` at path
`["repository","issue"]`, and `gh` exits non-zero. `GhCli` therefore returns stdout whenever it
parses as JSON with a `data` member, whatever the exit status, and `parse_search` treats that
specific error as "no such open issue" — the `search.nodes` hits are kept and no error is raised.
Any other `errors[]` entry, or a non-zero exit with no `data`, goes to `classify`.

- Environment: inherited, plus `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, `NO_COLOR=1`,
  `CLICOLOR=0`, `GH_PAGER=` (empty); `GH_DEBUG` removed, so debug traces never reach the stderr
  `classify` reads. `no_window` on Windows. No working directory dependence
  (cwd = the user's home, so `gh` never reads a repository's local config).
- Each invocation is `process::run_bounded(cmd, 10 s)` (FR-007, research R6), which drains stdout
  and stderr concurrently so a large page cannot fill a pipe and stall.
- **Only** `owner`, `name`, the cursor and — for search — the typed text leave the machine
  (FR-025). No path, branch or file name is ever an argument.
- Non-`OPEN` nodes from search/issue lookup are dropped (a closed issue is not "open"; the numeric
  lookup can return one).

## 4. Pure functions around the trait

| Function | Contract | Test file |
|---|---|---|
| `parse_list_page(&[u8]) -> Result<IssuePage, IssueLoadError>` | GraphQL JSON → page; `errors[]` with `type: NOT_FOUND` → `NoAccess`, `RATE_LIMITED` → `RateLimited`; malformed → `Other` | `github_parse.rs` |
| `parse_search(&[u8]) -> Result<Vec<Issue>, IssueLoadError>` | Union of `search.nodes` and `repository.issue`, open only, deduped by number; a sole `NOT_FOUND` at `["repository","issue"]` is not an error (§3) | `github_parse.rs` + fixtures `search_pr_number.json`, `search_missing_number.json`, `search_closed_number.json` |
| `load_listing(&dyn IssueSource, &GithubRepo) -> Result<IssueListing, IssueLoadError>` | Pages until no next cursor or `ISSUE_LOAD_CAP` (1,000) held; truncates the last page to the cap; `complete = held >= total_open`; first error aborts the whole load | `github_load.rs` (with `FakeIssueSource`) |
| `classify(&process::RunOutcome) -> IssueLoadError` | Research R8 table; unknown → `Other(first non-empty stderr line)`. `RunOutcome` moves from `env_include` to `process` with `run_bounded` (R6) | `github_classify.rs` + `tests/fixtures/gh/*.stderr` |
| `IssueLoadError::message(&GithubRepo) -> String` | One sentence of cause, one of remedy (FR-007); names `owner/name` for `NoAccess` | `github_classify.rs` |
| `merge_searched(loaded: &[Issue], searched: Vec<Issue>) -> Vec<Issue>` | Drops numbers already loaded (FR-005a) | `github_search.rs` |

## 5. Failure outcomes as the user reads them (FR-007, spec Edge Cases)

| `IssueLoadError` | Text (template) |
|---|---|
| `ToolMissing` | "Couldn't read issues: the GitHub CLI (`gh`) isn't installed. Install it from cli.github.com, then sign in with `gh auth login`." |
| `NotSignedIn` | "Couldn't read issues: you're not signed in to GitHub. Run `gh auth login` in a terminal, then retry." |
| `NoAccess` | "Couldn't read issues: your GitHub sign-in can't access owner/name." |
| `Offline` | "Couldn't reach GitHub. Check your connection, then retry." |
| `RateLimited` | "GitHub's rate limit was reached. Wait a minute, then retry." |
| `TimedOut` | "GitHub didn't answer within 10 seconds." |
| `Other(s)` | "Couldn't read issues: s" |

Every outcome is shown with a **Retry** action. The same set, prefixed "Search beyond the loaded
issues failed —", is used for `SearchState::Failed`, with the loaded matches left in place.
