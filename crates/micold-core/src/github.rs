//! Creating a worktree from a GitHub issue (feature 034): which GitHub repository a project's
//! remotes name, where `gh` is, and reading open issues through it.
//!
//! Everything that decides something is a pure function here, tested in `micold-core`; the
//! [`IssueSource`] implementation only runs `gh` and returns its bytes. Contracts:
//! `specs/034-github-issue-worktree/contracts/github-issue-source.md` and `remote-list-rpc.md`.

use std::fmt;

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
