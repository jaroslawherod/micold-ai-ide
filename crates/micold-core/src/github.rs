//! Creating a worktree from a GitHub issue (feature 034): which GitHub repository a project's
//! remotes name, where `gh` is, and reading open issues through it.
//!
//! Everything that decides something is a pure function here, tested in `micold-core`; the
//! [`IssueSource`] implementation only runs `gh` and returns its bytes. Contracts:
//! `specs/034-github-issue-worktree/contracts/github-issue-source.md` and `remote-list-rpc.md`.

use std::fmt;
use std::ops::Range;
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
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
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

    /// Whether `dir` is absolute on this OS, judged by text so every OS's rule is testable on
    /// any host: `/…` on Unix; `X:\…`, `X:/…` or a `\\server\share` UNC path on Windows. `gh`
    /// runs in the user's home, so a relative candidate would resolve somewhere else.
    fn is_absolute(self, dir: &str) -> bool {
        match self {
            HostOs::Linux | HostOs::MacOs => dir.starts_with('/'),
            HostOs::Windows => {
                let bytes = dir.as_bytes();
                dir.starts_with("\\\\")
                    || (bytes.len() >= 3
                        && bytes[0].is_ascii_alphabetic()
                        && bytes[1] == b':'
                        && matches!(bytes[2], b'\\' | b'/'))
            }
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
        // Windows allows a quoted entry; on Unix `"` is an ordinary file-name character.
        .map(|entry| match inputs.os {
            HostOs::Windows => entry.trim_matches('"'),
            HostOs::Linux | HostOs::MacOs => entry,
        })
        .filter(|entry| inputs.os.is_absolute(entry))
        .map(PathBuf::from);
    let well_known = inputs
        .os
        .well_known_dirs(inputs.home, inputs.env)
        .into_iter()
        .filter(|dir| inputs.os.is_absolute(&dir.to_string_lossy()));
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

/// [`locate_gh`] on this machine: its OS, its home directory, its environment and its files,
/// with `process_path` standing for this process's `PATH` — the client passes its own, and the
/// desktop-launch test passes the one a launcher would hand it.
pub fn locate_gh_on_host(env_include_path: Option<&str>, process_path: &str) -> Option<PathBuf> {
    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    locate_gh(&LocateInputs {
        os: HostOs::current(),
        env_include_path,
        process_path,
        home: home.as_deref(),
        env: &|name| std::env::var(name).ok(),
        exists: &|path| path.is_file(),
    })
}

/// The login GitHub shows for an issue whose author's account is gone (038 FR-002).
pub const GHOST_LOGIN: &str = "ghost";

/// What stands between the parts of a row's line and of its match text.
const PART_SEPARATOR: &str = "  ·  ";

/// One open issue, as the picker shows and ranks it (034 data-model §2, 038 data-model §1).
///
/// Held only in the open form (034 FR-023). No `Serialize`, so no code path can persist it
/// (SC-006), and a hand-written `Debug` that redacts the reporter, so no log can print it (038
/// FR-025).
#[derive(Clone, PartialEq, Eq)]
pub struct Issue {
    number: u64,
    title: String,
    labels: Vec<String>,
    updated_at: String,
    /// The author's login; [`GHOST_LOGIN`] when GitHub reports none. Never empty.
    reporter: String,
    row_text: String,
}

/// Where a match's emphasis lands on a row's two lines (038 data-model §3): byte ranges of
/// [`Issue::title_line`] and of [`Issue::details_line`], each list sorted, non-overlapping and on
/// character boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowEmphasis {
    /// Ranges of the title line.
    pub title: Vec<Range<usize>>,
    /// Ranges of the details line.
    pub details: Vec<Range<usize>>,
}

/// One of a row's two lines.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Line {
    Title,
    Details,
}

/// A stretch of the match text that is also shown on a line: `len` bytes, at `in_row` of
/// [`Issue::row_text`] and at `in_line` of `line`.
struct Part {
    line: Line,
    in_row: usize,
    in_line: usize,
    len: usize,
}

impl Issue {
    /// An issue reported by [`GHOST_LOGIN`], with its match text derived once, here. The reporter
    /// is set with [`Issue::reported_by`].
    pub fn new(number: u64, title: String, labels: Vec<String>, updated_at: String) -> Issue {
        let mut row_text = format!("#{number} {title}");
        if !labels.is_empty() {
            row_text.push_str(PART_SEPARATOR);
            row_text.push_str(&labels.join(", "));
        }
        Issue {
            number,
            title,
            labels,
            updated_at,
            reporter: GHOST_LOGIN.to_string(),
            row_text,
        }
    }

    /// The same issue, reported by `login`. An empty login is no author: [`GHOST_LOGIN`].
    pub fn reported_by(mut self, login: &str) -> Issue {
        self.reporter = if login.is_empty() { GHOST_LOGIN } else { login }.to_string();
        self
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

    /// The login of the issue's author as GitHub reports it, or [`GHOST_LOGIN`] (038 FR-002).
    pub fn reporter(&self) -> &str {
        &self.reporter
    }

    /// The match text, what the picker ranks: `#<number> <title>`, plus `  ·  <l1>, <l2>` when
    /// labelled (034 research R12). A row shows [`Issue::title_line`] and [`Issue::details_line`]
    /// instead (038 data-model §4).
    pub fn row_text(&self) -> &str {
        &self.row_text
    }

    /// A row's first line: `#<number> <title>` (038 FR-001).
    pub fn title_line(&self) -> String {
        format!("#{} {}", self.number, self.title)
    }

    /// A row's second line: the reporter, then `  ·  <l1>, <l2>` when labelled (038 FR-002,
    /// FR-003).
    pub fn details_line(&self) -> String {
        let mut line = self.reporter.clone();
        if !self.labels.is_empty() {
            line.push_str(PART_SEPARATOR);
            line.push_str(&self.labels.join(", "));
        }
        line
    }

    /// The parts of the match text, in its order. The separators between them belong to no part.
    fn parts(&self) -> Vec<Part> {
        let title_len = self.title_line().len();
        let mut parts = vec![Part {
            line: Line::Title,
            in_row: 0,
            in_line: 0,
            len: title_len,
        }];
        if !self.labels.is_empty() {
            parts.push(Part {
                line: Line::Details,
                in_row: title_len + PART_SEPARATOR.len(),
                in_line: self.reporter.len() + PART_SEPARATOR.len(),
                len: self.row_text.len() - title_len - PART_SEPARATOR.len(),
            });
        }
        parts
    }

    /// Where `spans` — byte ranges of [`Issue::row_text`], as a match reports them — fall on the
    /// two lines (038 data-model §4).
    ///
    /// Total: a span is cut to the parts it covers, so a separator is never emphasised and a span
    /// outside the match text is dropped; one that cuts through a character is widened to it.
    pub fn emphasis(&self, spans: &[Range<usize>]) -> RowEmphasis {
        let row = self.row_text.as_str();
        let mut emphasis = RowEmphasis::default();
        for span in spans {
            let start = floor_char_boundary(row, span.start);
            let end = ceil_char_boundary(row, span.end);
            for part in self.parts() {
                let from = start.max(part.in_row);
                let to = end.min(part.in_row + part.len);
                if from < to {
                    let rebased =
                        from - part.in_row + part.in_line..to - part.in_row + part.in_line;
                    match part.line {
                        Line::Title => emphasis.title.push(rebased),
                        Line::Details => emphasis.details.push(rebased),
                    }
                }
            }
        }
        emphasis.title = merged(emphasis.title);
        emphasis.details = merged(emphasis.details);
        emphasis
    }
}

/// The last character boundary of `text` at or before `at`.
fn floor_char_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// The first character boundary of `text` at or after `at`.
fn ceil_char_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at += 1;
    }
    at
}

/// `ranges` sorted, with overlapping and touching ranges made one.
fn merged(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

/// What `Debug` prints in place of a reporter.
const REDACTED: &str = "<redacted>";

/// Hand-written, so that the reporter is never printed: a `{:?}` in a log line or a panic message
/// shows which issue, not who reported it (038 FR-025). The match text is left out for the same
/// reason; it repeats the number, the title and the labels.
impl fmt::Debug for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Issue")
            .field("number", &self.number)
            .field("title", &self.title)
            .field("labels", &self.labels)
            .field("updated_at", &self.updated_at)
            .field("reporter", &REDACTED)
            .finish_non_exhaustive()
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

/// The fields read from an issue node, wherever the node comes from. A macro, so `concat!` can
/// put the one text into each query (038 contracts/issue-fields.md §1).
macro_rules! issue_node_selection {
    () => {
        "number title updatedAt labels(first: 20) { nodes { name } } author { login }"
    };
}

/// The selection [`LIST_QUERY`], [`SEARCH_QUERY`] and [`SEARCH_WITH_NUMBER_QUERY`] share for an
/// issue node, so a node from any of them parses alike (038 FR-006).
pub const ISSUE_NODE_SELECTION: &str = issue_node_selection!();

/// The GraphQL document for one page of open issues (contracts/github-issue-source.md §3). One
/// line, so every argument `gh` receives is one line.
pub const LIST_QUERY: &str = concat!(
    "query($owner: String!, $name: String!, $cursor: String) { ",
    "repository(owner: $owner, name: $name) { issues(states: OPEN, first: 100, after: $cursor, ",
    "orderBy: {field: UPDATED_AT, direction: DESC}) { totalCount pageInfo { hasNextPage endCursor } ",
    "nodes { ",
    issue_node_selection!(),
    " } } } }"
);

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

/// One `Issue` node, as [`ISSUE_NODE_SELECTION`] reads it: `number`, `title`, `updatedAt`,
/// `labels.nodes[].name`, `author.login`. A node without an author — a deleted account — is still
/// an issue, reported by [`GHOST_LOGIN`].
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
    let issue = Issue::new(
        node["number"].as_u64()?,
        node["title"].as_str()?.to_string(),
        labels,
        node["updatedAt"].as_str().unwrap_or_default().to_string(),
    );
    Some(issue.reported_by(node["author"]["login"].as_str().unwrap_or_default()))
}

/// The error a GraphQL `errors[]` array reports, if it has one.
fn graphql_error(json: &serde_json::Value) -> Option<IssueLoadError> {
    json["errors"].as_array()?.first().map(graphql_error_of)
}

/// What one GraphQL `errors[]` entry means for the user.
fn graphql_error_of(error: &serde_json::Value) -> IssueLoadError {
    match error["type"].as_str() {
        Some("NOT_FOUND") => IssueLoadError::NoAccess,
        Some("RATE_LIMITED") => IssueLoadError::RateLimited,
        _ => IssueLoadError::Other(
            error["message"]
                .as_str()
                .unwrap_or("GitHub reported an error")
                .to_string(),
        ),
    }
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

/// The GraphQL document for a search beyond the loaded issues (contracts/github-issue-source.md
/// §3). One line, like [`LIST_QUERY`].
pub const SEARCH_QUERY: &str = concat!(
    "query($q: String!) { search(type: ISSUE, query: $q, first: 50) { ",
    "nodes { ... on Issue { ",
    issue_node_selection!(),
    " state } } } }"
);

/// [`SEARCH_QUERY`] plus the issue whose number was typed: GitHub's search does not match an issue
/// by its number (research R2). A second document, because GraphQL rejects a declared variable the
/// query does not use.
pub const SEARCH_WITH_NUMBER_QUERY: &str = concat!(
    "query($q: String!, $owner: String!, $name: String!, ",
    "$n: Int!) { search(type: ISSUE, query: $q, first: 50) { nodes { ... on Issue { ",
    issue_node_selection!(),
    " state } } } repository(owner: $owner, name: $name) ",
    "{ issue(number: $n) { ",
    issue_node_selection!(),
    " state } } }"
);

/// The arguments after `gh` for a search beyond the loaded issues: only the typed text and — for a
/// typed number — the repository and that number leave the machine (FR-025).
///
/// `N` or `#N` with `N` fitting GraphQL's `Int` also looks the number up, because search never
/// matches an issue by its number. The number is the only `-F` (typed) variable; every string stays
/// `-f` (contracts/github-issue-source.md §3).
pub fn search_args(repo: &GithubRepo, text: &str) -> Vec<String> {
    let number = typed_issue_number(text);
    let query = if number.is_some() {
        SEARCH_WITH_NUMBER_QUERY
    } else {
        SEARCH_QUERY
    };
    let mut args: Vec<String> = [
        "api",
        "graphql",
        "--hostname",
        "github.com",
        "-f",
        &format!("query={query}"),
        "-f",
        &format!("q=repo:{repo} is:issue is:open {text}"),
    ]
    .map(str::to_string)
    .to_vec();
    if let Some(n) = number {
        args.extend([
            "-f".to_string(),
            format!("owner={}", repo.owner),
            "-f".to_string(),
            format!("name={}", repo.name),
            "-F".to_string(),
            format!("n={n}"),
        ]);
    }
    args
}

/// The issue number `text` names, when it is nothing but `N` or `#N` and `N` fits GraphQL's `Int`.
fn typed_issue_number(text: &str) -> Option<i32> {
    let digits = text.trim();
    let digits = digits.strip_prefix('#').unwrap_or(digits);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Parse `gh api graphql` stdout for [`SEARCH_QUERY`] or [`SEARCH_WITH_NUMBER_QUERY`].
///
/// The result is the search hits followed by the looked-up issue, open only, each number once. A
/// number that is a pull request's or does not exist answers `NOT_FOUND` at
/// `["repository","issue"]`; that alone is "no such open issue", not a failure. Any other GraphQL
/// error is classified as [`parse_list_page`] classifies it.
pub fn parse_search(stdout: &[u8]) -> Result<Vec<Issue>, IssueLoadError> {
    let json: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|e| IssueLoadError::Other(format!("GitHub's answer could not be read: {e}")))?;
    let lookup_not_found = |error: &serde_json::Value| {
        error["type"] == "NOT_FOUND" && error["path"] == serde_json::json!(["repository", "issue"])
    };
    if let Some(errors) = json["errors"].as_array() {
        if let Some(other) = errors.iter().find(|e| !lookup_not_found(e)) {
            return Err(graphql_error_of(other));
        }
    }
    let data = &json["data"];
    let hits = data["search"]["nodes"]
        .as_array()
        .ok_or_else(|| IssueLoadError::Other("GitHub's answer had no search results".into()))?;
    let mut found: Vec<Issue> = Vec::new();
    let lookup = &data["repository"]["issue"];
    for node in hits.iter().chain(lookup.is_object().then_some(lookup)) {
        // A pull request is an empty node under `... on Issue`; a closed issue is not open.
        if node["state"] != "OPEN" {
            continue;
        }
        if let Some(issue) = issue_from_node(node) {
            if !found.iter().any(|held| held.number == issue.number) {
                found.push(issue);
            }
        }
    }
    Ok(found)
}

/// The searched issues the loaded list does not already hold, in GitHub's order (FR-005a
/// "without duplicates").
pub fn merge_searched(loaded: &[Issue], searched: Vec<Issue>) -> Vec<Issue> {
    searched
        .into_iter()
        .filter(|issue| !loaded.iter().any(|held| held.number == issue.number))
        .collect()
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

    /// Open issues matching `text` on GitHub's side (FR-005a), unfiltered: the caller holds them
    /// to FR-005's rule.
    fn search_open(&self, repo: &GithubRepo, text: &str) -> Result<Vec<Issue>, IssueLoadError>;
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
        // A page that adds nothing, or hands back the cursor it was asked with, would be asked
        // for again forever: end the load with what is held.
        let stalled = page.issues.is_empty() || page.next_cursor == cursor;
        issues.extend(page.issues);
        if stalled || issues.len() >= ISSUE_LOAD_CAP || page.next_cursor.is_none() {
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

/// A scripted [`IssueSource`] for tests: answers each call with the next scripted page, search
/// result or error, and records every call. Public (not `#[cfg(test)]`) so every crate's tests can use it, like
/// [`crate::git::FakeGit`].
#[derive(Debug, Default)]
pub struct FakeIssueSource {
    script: std::sync::Mutex<std::collections::VecDeque<Result<IssuePage, IssueLoadError>>>,
    calls: std::sync::Mutex<Vec<(String, Option<String>)>>,
    searches: std::sync::Mutex<std::collections::VecDeque<Result<Vec<Issue>, IssueLoadError>>>,
    search_calls: std::sync::Mutex<Vec<(String, String)>>,
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

    /// Answer the next unanswered `search_open` call with `result`.
    pub fn with_search(self, result: Result<Vec<Issue>, IssueLoadError>) -> Self {
        self.searches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(result);
        self
    }

    /// Every `search_open` call so far, as (`owner/name`, text).
    pub fn search_calls(&self) -> Vec<(String, String)> {
        self.search_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
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

    fn search_open(&self, repo: &GithubRepo, text: &str) -> Result<Vec<Issue>, IssueLoadError> {
        self.search_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((repo.to_string(), text.to_string()));
        self.searches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
            .unwrap_or_else(|| {
                Err(IssueLoadError::Other(
                    "FakeIssueSource: no search scripted for this call".into(),
                ))
            })
    }
}

impl IssueLoadError {
    /// The plain-language text the form shows: the cause, and what to do (FR-007,
    /// contracts/github-issue-source.md §5).
    pub fn message(&self, repo: &GithubRepo) -> String {
        match self {
            Self::ToolMissing => "Couldn't read issues: the GitHub CLI (`gh`) isn't installed. \
                                  Install it from cli.github.com, then sign in with `gh auth login`."
                .to_owned(),
            Self::NotSignedIn => "Couldn't read issues: you're not signed in to GitHub. \
                                  Run `gh auth login` in a terminal, then retry."
                .to_owned(),
            Self::NoAccess => format!("Couldn't read issues: your GitHub sign-in can't access {repo}."),
            Self::Offline => "Couldn't reach GitHub. Check your connection, then retry.".to_owned(),
            Self::RateLimited => {
                "GitHub's rate limit was reached. Wait a minute, then retry.".to_owned()
            }
            Self::TimedOut => "GitHub didn't answer within 10 seconds.".to_owned(),
            Self::Other(detail) => format!("Couldn't read issues: {detail}"),
        }
    }
}

/// The reason a failed `gh` run gives (research R8), read from its exit status and stderr.
///
/// Order matters: a rate limit can arrive as HTTP 403, so it is checked before the
/// access failures that HTTP 403 otherwise means.
pub fn classify(outcome: &crate::process::RunOutcome) -> IssueLoadError {
    use crate::process::RunOutcome;
    let (code, stderr) = match outcome {
        RunOutcome::TimedOut { .. } => return IssueLoadError::TimedOut,
        RunOutcome::SpawnFailed(reason) => {
            let lower = reason.to_ascii_lowercase();
            return if lower.contains("os error 2") || lower.contains("not found") {
                IssueLoadError::ToolMissing
            } else {
                IssueLoadError::Other(reason.clone())
            };
        }
        RunOutcome::Exited { code, stderr, .. } => (*code, stderr),
    };
    let lower = stderr.to_ascii_lowercase();
    let says = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
    if code == GH_EXIT_AUTH
        || says(&[
            "gh auth login",
            "not logged in",
            "http 401",
            "bad credentials",
        ])
    {
        IssueLoadError::NotSignedIn
    } else if says(&["rate limit", "rate_limited", "http 429"]) {
        IssueLoadError::RateLimited
    } else if says(&[
        "could not resolve to a repository",
        "http 404",
        "http 403",
        "saml",
        "resource not accessible",
        "required scopes",
        "not_found",
    ]) {
        IssueLoadError::NoAccess
    } else if says(&[
        "error connecting to",
        "dial tcp",
        "no such host",
        "could not resolve host",
        "connection refused",
        "network is unreachable",
        "tls handshake timeout",
        "i/o timeout",
    ]) {
        IssueLoadError::Offline
    } else {
        IssueLoadError::Other(
            stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map_or_else(|| format!("gh exited with status {code}"), str::to_owned),
        )
    }
}

/// `gh`'s documented exit status for "authentication required".
const GH_EXIT_AUTH: i32 = 4;

/// The production [`IssueSource`]: the user's own `gh`, run non-interactively
/// (contracts/github-issue-source.md §3, research R6).
#[derive(Debug, Clone)]
pub struct GhCli {
    gh: PathBuf,
    timeout: std::time::Duration,
}

impl GhCli {
    /// A source running the `gh` at `gh` (from [`locate_gh`]).
    pub fn new(gh: PathBuf) -> Self {
        Self {
            gh,
            timeout: GH_TIMEOUT,
        }
    }

    /// The same source with a different bound on each `gh` run. Tests only; production keeps
    /// the default.
    pub fn with_timeout(self, timeout: std::time::Duration) -> Self {
        Self { timeout, ..self }
    }
}

impl GhCli {
    /// Run `gh` with `args` and read its answer with `parse`.
    ///
    /// `gh` exits non-zero on any GraphQL error, including the numbered lookup's NOT_FOUND beside
    /// good search hits, so an answer `parse` accepts is used whatever the exit status
    /// (contracts/github-issue-source.md §3), and so is an error it types exactly. An answer it
    /// cannot type is classified from the exit status and stderr, which name what GraphQL's error
    /// types do not (SAML, missing scopes).
    fn run<T>(
        &self,
        args: Vec<String>,
        parse: fn(&[u8]) -> Result<T, IssueLoadError>,
    ) -> Result<T, IssueLoadError> {
        let mut cmd = std::process::Command::new(&self.gh);
        crate::process::no_window(&mut cmd)
            .args(args)
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .env("GH_PAGER", "")
            // Debug traces would reach the stderr `classify` reads.
            .env_remove("GH_DEBUG")
            .stdin(std::process::Stdio::null());
        // The user's home, never a project: a repository's local config must not steer `gh`.
        if let Some(dirs) = directories::BaseDirs::new() {
            cmd.current_dir(dirs.home_dir());
        }
        let outcome = crate::process::run_bounded(cmd, self.timeout);
        match &outcome {
            crate::process::RunOutcome::Exited {
                code: 0, stdout, ..
            } => parse(stdout),
            // An error the answer types exactly (NOT_FOUND, RATE_LIMITED) stands; one it only
            // names (`Other`) is classified from stderr, which knows SAML and scope refusals, and
            // keeps GitHub's own words when stderr adds nothing better.
            crate::process::RunOutcome::Exited { stdout, .. } => match parse(stdout) {
                Err(named @ IssueLoadError::Other(_)) => match classify(&outcome) {
                    IssueLoadError::Other(_) if !stdout.is_empty() => Err(named),
                    typed => Err(typed),
                },
                answer => answer,
            },
            _ => Err(classify(&outcome)),
        }
    }
}

impl IssueSource for GhCli {
    fn list_open(
        &self,
        repo: &GithubRepo,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        self.run(list_args(repo, cursor), parse_list_page)
    }

    fn search_open(&self, repo: &GithubRepo, text: &str) -> Result<Vec<Issue>, IssueLoadError> {
        self.run(search_args(repo, text), parse_search)
    }
}

/// The bound on one `gh` run (FR-007's "didn't answer within 10 seconds").
const GH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
