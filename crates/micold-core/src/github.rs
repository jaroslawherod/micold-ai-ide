//! Creating a worktree from a GitHub issue (feature 034): which GitHub repository a project's
//! remotes name, where `gh` is, and reading open issues through it.
//!
//! Everything that decides something is a pure function here, tested in `micold-core`; the
//! [`IssueSource`] implementation only runs `gh` and returns its bytes. Contracts:
//! `specs/034-github-issue-worktree/contracts/github-issue-source.md` and `remote-list-rpc.md`.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::git::GitRemote;

/// A repository on github.com, as `owner/name` (data-model §1).
///
/// Constructed only by [`GithubRepo::from_remote_url`], so an Enterprise or non-GitHub repository
/// is unrepresentable and a token in a remote URL can never reach the display (FR-022).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GithubRepo {
    owner: String,
    name: String,
}

impl GithubRepo {
    /// The repository a remote URL names, when it is on github.com (FR-002, research R5).
    ///
    /// Accepts `http(s)://[userinfo@]github.com/o/r`, `git://github.com/o/r`,
    /// `ssh://[user@]github.com[:port]/o/r`, `ssh://git@ssh.github.com:443/o/r` and the scp form
    /// `[user@]github.com:o/r`, each with an optional `.git` and trailing `/`; the host is compared
    /// ignoring case. Userinfo is discarded. Anything else — another host, a local path, an alias
    /// a global `insteadOf` would expand — is not a GitHub remote.
    pub fn from_remote_url(url: &str) -> Option<GithubRepo> {
        let url = url.trim();
        let (host, path, over_ssh) = match url.split_once("://") {
            Some((scheme, rest)) => {
                let over_ssh = match scheme.to_ascii_lowercase().as_str() {
                    "https" | "http" | "git" => false,
                    "ssh" => true,
                    _ => return None,
                };
                let (authority, path) = rest.split_once('/')?;
                let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
                let host = match host_port.split_once(':') {
                    Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => host,
                    Some(_) => return None,
                    None => host_port,
                };
                (host, path, over_ssh)
            }
            None => {
                // scp form: `[user@]host:path`, with no `/` before the colon.
                let (user_host, path) = url.split_once(':')?;
                if user_host.contains('/') {
                    return None;
                }
                let host = user_host.rsplit_once('@').map_or(user_host, |(_, h)| h);
                (host, path, true)
            }
        };
        let on_github = host.eq_ignore_ascii_case("github.com")
            || (over_ssh && host.eq_ignore_ascii_case("ssh.github.com"));
        if !on_github {
            return None;
        }
        let path = path.trim_start_matches('/').trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let (owner, name) = path.split_once('/')?;
        let owner_ok = !owner.is_empty()
            && owner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
        let name_ok = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
        (owner_ok && name_ok).then(|| GithubRepo {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    /// The owning user or organization.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The repository's name, without `.git`.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for GithubRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// Which remote the issue source reads (contracts/remote-list-rpc.md §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteChoice {
    /// `origin` if it is on GitHub, else the first GitHub remote in config order.
    Github {
        /// The remote's name.
        remote: String,
        /// The repository it names.
        repo: GithubRepo,
    },
    /// No remote, or none on github.com (FR-002).
    NoGithubRemote,
}

/// Choose the remote whose issues are offered (spec Edge Cases, "several remotes").
pub fn choose_remote(remotes: &[GitRemote]) -> RemoteChoice {
    let on_github = |r: &GitRemote| {
        GithubRepo::from_remote_url(&r.url).map(|repo| RemoteChoice::Github {
            remote: r.name.clone(),
            repo,
        })
    };
    remotes
        .iter()
        .filter(|r| r.name == "origin")
        .find_map(on_github)
        .or_else(|| remotes.iter().find_map(on_github))
        .unwrap_or(RemoteChoice::NoGithubRemote)
}

/// The OS whose conventions [`locate_gh`] follows. Passed in rather than read from the host, so
/// every OS's table, separator and file name is tested on every CI host (research R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    /// Linux and other Unix desktops.
    Linux,
    /// macOS.
    MacOs,
    /// Windows.
    Windows,
}

impl HostOs {
    /// The OS this build runs on — the one `cfg` in the lookup.
    pub fn current() -> HostOs {
        if cfg!(windows) {
            HostOs::Windows
        } else if cfg!(target_os = "macos") {
            HostOs::MacOs
        } else {
            HostOs::Linux
        }
    }

    /// The GitHub CLI's file name on this OS.
    pub fn exe_name(self) -> &'static str {
        match self {
            HostOs::Windows => "gh.exe",
            HostOs::Linux | HostOs::MacOs => "gh",
        }
    }

    /// The separator between `PATH` entries on this OS.
    pub fn path_separator(self) -> char {
        match self {
            HostOs::Windows => ';',
            HostOs::Linux | HostOs::MacOs => ':',
        }
    }

    /// Where `gh`'s installers put it, for a launch whose `PATH` does not say (research R3).
    fn well_known_dirs(
        self,
        home: Option<&Path>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<PathBuf> {
        let under_home = |parts: &[&str]| {
            home.map(|h| {
                parts
                    .iter()
                    .fold(h.to_path_buf(), |dir, part| dir.join(part))
            })
        };
        match self {
            HostOs::MacOs => [
                Some(PathBuf::from("/opt/homebrew/bin")),
                Some(PathBuf::from("/usr/local/bin")),
                Some(PathBuf::from("/opt/local/bin")),
                under_home(&[".local", "bin"]),
            ]
            .into_iter()
            .flatten()
            .collect(),
            HostOs::Linux => [
                Some(PathBuf::from("/usr/local/bin")),
                Some(PathBuf::from("/usr/bin")),
                Some(PathBuf::from("/snap/bin")),
                Some(PathBuf::from("/home/linuxbrew/.linuxbrew/bin")),
                under_home(&[".linuxbrew", "bin"]),
                under_home(&[".local", "bin"]),
                under_home(&["bin"]),
            ]
            .into_iter()
            .flatten()
            .collect(),
            // Built as text with `\`, not with `Path::join`, so the Windows table is the same
            // strings on every host.
            HostOs::Windows => [
                ("ProgramFiles", r"\GitHub CLI"),
                ("ProgramFiles(x86)", r"\GitHub CLI"),
                ("LOCALAPPDATA", r"\Microsoft\WinGet\Links"),
                ("USERPROFILE", r"\scoop\shims"),
                ("ProgramData", r"\chocolatey\bin"),
            ]
            .into_iter()
            .filter_map(|(var, tail)| {
                env(var)
                    .filter(|base| !base.is_empty())
                    .map(|base| PathBuf::from(format!("{}{tail}", base.trim_end_matches('\\'))))
            })
            .collect(),
        }
    }
}

/// Everything [`locate_gh`] reads, injected so the walk is a pure function (research R3).
pub struct LocateInputs<'a> {
    /// Whose conventions to follow.
    pub os: HostOs,
    /// The `PATH` the environment-include snapshot contributes, if any ([`env_include_path`]).
    pub env_include_path: Option<&'a str>,
    /// This process's own `PATH`.
    pub process_path: &'a str,
    /// The user's home directory.
    pub home: Option<&'a Path>,
    /// Reads an environment variable (`ProgramFiles`, `LOCALAPPDATA`, …).
    pub env: &'a dyn Fn(&str) -> Option<String>,
    /// Whether a file exists: `Path::is_file` in production, a fake set in tests.
    pub exists: &'a dyn Fn(&Path) -> bool,
}

/// The `PATH` an environment-include snapshot contributes, its key matched ignoring case
/// (Windows spells it `Path`).
pub fn env_include_path(vars: &[(String, String)]) -> Option<&str> {
    vars.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| value.as_str())
}

/// The directories searched for `gh`, in order, each once (contracts/github-issue-source.md §1).
///
/// Env-include `PATH`, then process `PATH`, then the OS's well-known directories. Split on
/// [`HostOs::path_separator`], never the host's `split_paths`; duplicates keep their first position
/// and empty components are dropped.
pub fn candidate_dirs(inputs: &LocateInputs) -> Vec<PathBuf> {
    let separator = inputs.os.path_separator();
    let from_paths = [inputs.env_include_path, Some(inputs.process_path)]
        .into_iter()
        .flatten()
        .flat_map(|path| path.split(separator))
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from);
    let well_known = inputs.os.well_known_dirs(inputs.home, inputs.env);
    let mut dirs: Vec<PathBuf> = Vec::new();
    for dir in from_paths.chain(well_known) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

/// The first candidate directory holding `gh`, as an absolute path to spawn.
///
/// The file name is [`HostOs::exe_name`], never the host's `PATHEXT`.
pub fn locate_gh(inputs: &LocateInputs) -> Option<PathBuf> {
    candidate_dirs(inputs)
        .into_iter()
        .map(|dir| dir.join(inputs.os.exe_name()))
        .find(|candidate| (inputs.exists)(candidate))
}

/// One open issue, as the picker shows and ranks it (data-model §2).
///
/// Held only in the open form (FR-023). No `Serialize`, so no code path can persist it (SC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    number: u64,
    title: String,
    labels: Vec<String>,
    updated_at: String,
    row_text: String,
}

impl Issue {
    /// An issue, with its row text derived once, here.
    pub fn new(number: u64, title: String, labels: Vec<String>, updated_at: String) -> Issue {
        let mut row_text = format!("#{number} {title}");
        if !labels.is_empty() {
            row_text.push_str("  ·  ");
            row_text.push_str(&labels.join(", "));
        }
        Issue {
            number,
            title,
            labels,
            updated_at,
            row_text,
        }
    }

    /// The issue number; the ticket is its decimal text (FR-009).
    pub fn number(&self) -> u64 {
        self.number
    }

    /// The title as GitHub returns it.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Label names, at most 20 (research R2).
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// RFC 3339 last update, as GitHub returns it.
    pub fn updated_at(&self) -> &str {
        &self.updated_at
    }

    /// `#<number> <title>`, plus `  ·  <l1>, <l2>` when labelled: the text the picker shows and
    /// ranks (research R12).
    pub fn row_text(&self) -> &str {
        &self.row_text
    }
}

/// One page of the open-issue connection (data-model §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuePage {
    /// This page's issues, most recently updated first.
    pub issues: Vec<Issue>,
    /// GitHub's `totalCount` of open issues.
    pub total_open: u64,
    /// Where the next page starts; `None` on the last page.
    pub next_cursor: Option<String>,
}

/// Why issues could not be read (FR-007, research R8). Every variant offers a retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueLoadError {
    /// `gh` was not found (research R3).
    ToolMissing,
    /// Not signed in to GitHub, or the sign-in is no longer valid.
    NotSignedIn,
    /// The sign-in cannot see the repository.
    NoAccess,
    /// GitHub could not be reached.
    Offline,
    /// GitHub's rate limit was reached.
    RateLimited,
    /// No answer within 10 seconds; `gh` was killed (research R6).
    TimedOut,
    /// Anything else: the first non-empty line `gh` printed.
    Other(String),
}

/// The GraphQL document for one page of open issues (contracts/github-issue-source.md §3). One
/// line, so every argument `gh` receives is one line.
pub const LIST_QUERY: &str = "query($owner: String!, $name: String!, $cursor: String) { \
repository(owner: $owner, name: $name) { issues(states: OPEN, first: 100, after: $cursor, \
orderBy: {field: UPDATED_AT, direction: DESC}) { totalCount pageInfo { hasNextPage endCursor } \
nodes { number title updatedAt labels(first: 20) { nodes { name } } } } } }";

/// Parse `gh api graphql` stdout for [`LIST_QUERY`] into a page.
///
/// A GraphQL `errors[]` entry of type `NOT_FOUND` is [`IssueLoadError::NoAccess`] (GitHub answers
/// a repository the sign-in cannot see exactly as one that does not exist); `RATE_LIMITED` is
/// [`IssueLoadError::RateLimited`]; any other error, or JSON that is not the expected shape, is
/// [`IssueLoadError::Other`].
pub fn parse_list_page(stdout: &[u8]) -> Result<IssuePage, IssueLoadError> {
    let json: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|e| IssueLoadError::Other(format!("GitHub's answer could not be read: {e}")))?;
    if let Some(error) = graphql_error(&json) {
        return Err(error);
    }
    let issues = &json["data"]["repository"]["issues"];
    let unexpected = || IssueLoadError::Other("GitHub's answer had no issue list".into());
    let total_open = issues["totalCount"].as_u64().ok_or_else(unexpected)?;
    let nodes = issues["nodes"].as_array().ok_or_else(unexpected)?;
    let next_cursor = if issues["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
        issues["pageInfo"]["endCursor"].as_str().map(str::to_string)
    } else {
        None
    };
    let issues = nodes
        .iter()
        .map(|node| issue_from_node(node).ok_or_else(unexpected))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IssuePage {
        issues,
        total_open,
        next_cursor,
    })
}

/// Labels held per issue; the query asks for no more (research R2).
const LABELS_PER_ISSUE: usize = 20;

/// One `Issue` node: `number`, `title`, `updatedAt`, `labels.nodes[].name`.
fn issue_from_node(node: &serde_json::Value) -> Option<Issue> {
    let labels = node["labels"]["nodes"]
        .as_array()
        .map(|labels| {
            labels
                .iter()
                .filter_map(|label| label["name"].as_str().map(str::to_string))
                .take(LABELS_PER_ISSUE)
                .collect()
        })
        .unwrap_or_default();
    Some(Issue::new(
        node["number"].as_u64()?,
        node["title"].as_str()?.to_string(),
        labels,
        node["updatedAt"].as_str().unwrap_or_default().to_string(),
    ))
}

/// The error a GraphQL `errors[]` array reports, if it has one.
fn graphql_error(json: &serde_json::Value) -> Option<IssueLoadError> {
    let first = json["errors"].as_array()?.first()?;
    Some(match first["type"].as_str() {
        Some("NOT_FOUND") => IssueLoadError::NoAccess,
        Some("RATE_LIMITED") => IssueLoadError::RateLimited,
        _ => IssueLoadError::Other(
            first["message"]
                .as_str()
                .unwrap_or("GitHub reported an error")
                .to_string(),
        ),
    })
}

/// The arguments after `gh` for one page: only the repository and the cursor leave the machine
/// (FR-025).
///
/// Every variable is a raw string (`-f`): `-F` would turn a repository named `1` or `true` into a
/// number or a boolean (contracts/github-issue-source.md §3).
pub fn list_args(repo: &GithubRepo, cursor: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = [
        "api",
        "graphql",
        "--hostname",
        "github.com",
        "-f",
        &format!("query={LIST_QUERY}"),
        "-f",
        &format!("owner={}", repo.owner),
        "-f",
        &format!("name={}", repo.name),
    ]
    .map(str::to_string)
    .to_vec();
    if let Some(cursor) = cursor {
        args.push("-f".into());
        args.push(format!("cursor={cursor}"));
    }
    args
}

/// Open issues the form holds at most (FR-004).
pub const ISSUE_LOAD_CAP: usize = 1_000;

/// The open issues a load produced (data-model §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueListing {
    /// Most recently updated first, at most [`ISSUE_LOAD_CAP`].
    pub issues: Vec<Issue>,
    /// GitHub's count of open issues.
    pub total_open: u64,
    /// Every open issue is held, so search never needs to reach GitHub (FR-005a).
    pub complete: bool,
}

/// Where open issues come from (research R7): [`GhCli`] in production, [`FakeIssueSource`] in
/// tests. The paging, cap and classification around it are pure functions in this module.
pub trait IssueSource {
    /// One page of open issues, most recently updated first.
    fn list_open(
        &self,
        repo: &GithubRepo,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError>;
}

/// Load open issues page by page, up to [`ISSUE_LOAD_CAP`] (contracts/github-issue-source.md §4).
///
/// Pages until there is no next cursor or the cap is held, truncating the page that crosses it.
/// The first error aborts the whole load: a partial list is never shown as the list.
pub fn load_listing(
    source: &dyn IssueSource,
    repo: &GithubRepo,
) -> Result<IssueListing, IssueLoadError> {
    let mut issues: Vec<Issue> = Vec::new();
    let mut cursor: Option<String> = None;
    let total_open = loop {
        let page = source.list_open(repo, cursor.as_deref())?;
        issues.extend(page.issues);
        if issues.len() >= ISSUE_LOAD_CAP || page.next_cursor.is_none() {
            issues.truncate(ISSUE_LOAD_CAP);
            break page.total_open;
        }
        cursor = page.next_cursor;
    };
    Ok(IssueListing {
        complete: issues.len() as u64 >= total_open,
        issues,
        total_open,
    })
}

/// A scripted [`IssueSource`] for tests: answers each call with the next scripted page or error,
/// and records every call. Public (not `#[cfg(test)]`) so every crate's tests can use it, like
/// [`crate::git::FakeGit`].
#[derive(Debug, Default)]
pub struct FakeIssueSource {
    script: std::sync::Mutex<std::collections::VecDeque<Result<IssuePage, IssueLoadError>>>,
    calls: std::sync::Mutex<Vec<(String, Option<String>)>>,
}

impl FakeIssueSource {
    /// A source with nothing scripted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer the next unanswered call with `page`.
    pub fn with_page(self, page: IssuePage) -> Self {
        self.lock_script().push_back(Ok(page));
        self
    }

    /// Answer the next unanswered call with `error`.
    pub fn with_error(self, error: IssueLoadError) -> Self {
        self.lock_script().push_back(Err(error));
        self
    }

    /// Every `list_open` call so far, as (`owner/name`, cursor).
    pub fn calls(&self) -> Vec<(String, Option<String>)> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn lock_script(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::VecDeque<Result<IssuePage, IssueLoadError>>>
    {
        self.script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl IssueSource for FakeIssueSource {
    fn list_open(
        &self,
        repo: &GithubRepo,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), cursor.map(str::to_string)));
        self.lock_script().pop_front().unwrap_or_else(|| {
            Err(IssueLoadError::Other(
                "FakeIssueSource: no page scripted for this call".into(),
            ))
        })
    }
}
