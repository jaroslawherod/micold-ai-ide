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
