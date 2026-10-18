//! Where a Kubernetes daemon's worktrees come from (feature 686, FR-007, research K8): the rules for
//! choosing the project's remote, naming the clone on the daemon's volume, and wording a failed
//! clone. Pure: the clone itself is the daemon's (`micold-daemon::repo_clone`).

use crate::git::GitRemote;
use crate::kube::credentials::REPOS_ROOT;
use crate::kube::names;
use crate::protocol::hashing::sha256_hex;
use crate::protocol::messages::CloneFailureWire;

/// The remote a clone is made from, or why there is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteRefusal {
    /// The project has no remote with a URL.
    NoRemote,
    /// Several remotes and none is `origin`; the names, in config order.
    Ambiguous(Vec<String>),
}

impl RemoteRefusal {
    /// The refusal as shown to the user (contract §6, clone).
    pub fn message(&self, project: &str) -> String {
        match self {
            RemoteRefusal::NoRemote => {
                format!("{project} has no git remote; a Kubernetes daemon clones from it")
            }
            RemoteRefusal::Ambiguous(names) => format!(
                "{project} has several remotes and none is origin: {}",
                names.join(", ")
            ),
        }
    }
}

/// `origin` when there is one, else the only remote.
pub fn remote_for(remotes: &[GitRemote]) -> Result<&GitRemote, RemoteRefusal> {
    if let Some(origin) = remotes.iter().find(|r| r.name == "origin") {
        return Ok(origin);
    }
    match remotes {
        [] => Err(RemoteRefusal::NoRemote),
        [only] => Ok(only),
        many => Err(RemoteRefusal::Ambiguous(
            many.iter().map(|r| r.name.clone()).collect(),
        )),
    }
}

/// Where the clone of one URL lives on the daemon's volume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneTarget {
    /// The absolute path in the pod, POSIX.
    pub dest: String,
}

impl CloneTarget {
    /// `/data/repos/<slug(repo)>-<hash8(url)>`: the same for the same URL, so every client of the
    /// daemon meets the same clone.
    pub fn for_url(url: &str) -> Self {
        let url = url.trim();
        let tail = url
            .trim_end_matches('/')
            .rsplit(['/', ':'])
            .next()
            .unwrap_or("");
        let repo = tail.strip_suffix(".git").unwrap_or(tail);
        let slug = if repo.is_empty() {
            "repo".to_string()
        } else {
            names::slug(repo)
        };
        let hash = sha256_hex(url.as_bytes());
        CloneTarget {
            dest: format!("{REPOS_ROOT}/{slug}-{}", &hash[..8]),
        }
    }
}

/// Why a clone failed, with what the user needs to act on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneFailure {
    /// The remote needs a credential the daemon does not have.
    AuthRequired {
        /// The remote's URL.
        remote: String,
    },
    /// The volume has no room left.
    DiskFull,
    /// The pod could not reach the remote.
    Unreachable {
        /// The remote's URL.
        remote: String,
    },
    /// Anything else, with git's own words.
    Other(String),
}

impl CloneFailure {
    /// Classify git's stderr for a clone of `remote`.
    pub fn from_stderr(stderr: &str, remote: &str) -> Self {
        Self::from_wire(CloneFailureWire::classify(stderr), remote)
    }

    /// The wire form's failure, with the remote this client asked for.
    pub fn from_wire(wire: CloneFailureWire, remote: &str) -> Self {
        match wire {
            CloneFailureWire::AuthRequired => CloneFailure::AuthRequired {
                remote: remote.to_string(),
            },
            CloneFailureWire::DiskFull => CloneFailure::DiskFull,
            CloneFailureWire::Unreachable => CloneFailure::Unreachable {
                remote: remote.to_string(),
            },
            CloneFailureWire::Other(text) => CloneFailure::Other(text),
        }
    }

    /// The failure as shown to the user (contract §6, clone), naming `daemon`.
    pub fn message(&self, daemon: &str) -> String {
        match self {
            CloneFailure::AuthRequired { remote } => format!(
                "cloning {remote} needs a credential: allow Git credentials or an SSH key for {daemon}"
            ),
            CloneFailure::DiskFull => format!("the volume of {daemon} is full"),
            CloneFailure::Unreachable { remote } => {
                format!("{daemon} could not reach {remote}")
            }
            CloneFailure::Other(text) => text.clone(),
        }
    }
}

impl CloneFailureWire {
    /// Classify git's stderr. Authentication is looked for first: a refused key also prints
    /// "Could not read from remote repository".
    pub fn classify(stderr: &str) -> Self {
        let lower = stderr.to_lowercase();
        let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
        if has(&[
            "authentication failed",
            "could not read username",
            "could not read password",
            "terminal prompts disabled",
            "permission denied (publickey",
            "host key verification failed",
            "http basic: access denied",
        ]) {
            CloneFailureWire::AuthRequired
        } else if has(&["no space left on device", "disk quota exceeded"]) {
            CloneFailureWire::DiskFull
        } else if has(&[
            "could not resolve host",
            "connection timed out",
            "connection refused",
            "network is unreachable",
            "unable to access",
        ]) {
            CloneFailureWire::Unreachable
        } else {
            CloneFailureWire::Other(stderr.trim().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(name: &str, url: &str) -> GitRemote {
        GitRemote {
            name: name.into(),
            url: url.into(),
        }
    }

    #[test]
    fn origin_wins_else_the_only_remote() {
        let both = [remote("up", "u"), remote("origin", "o")];
        assert_eq!(remote_for(&both).unwrap().url, "o");
        let only = [remote("fork", "f")];
        assert_eq!(remote_for(&only).unwrap().url, "f");
    }

    #[test]
    fn none_or_several_without_origin_is_refused_with_the_contract_text() {
        let none = remote_for(&[]).unwrap_err();
        assert_eq!(none, RemoteRefusal::NoRemote);
        assert_eq!(
            none.message("app"),
            "app has no git remote; a Kubernetes daemon clones from it"
        );
        let many = remote_for(&[remote("a", "1"), remote("b", "2")]).unwrap_err();
        assert_eq!(many, RemoteRefusal::Ambiguous(vec!["a".into(), "b".into()]));
        assert_eq!(
            many.message("app"),
            "app has several remotes and none is origin: a, b"
        );
    }

    #[test]
    fn the_target_is_slug_and_hash_of_the_url_and_deterministic() {
        let url = "git@github.com:acme/My_Repo.git";
        let target = CloneTarget::for_url(url);
        assert_eq!(target, CloneTarget::for_url(url));
        let rest = target.dest.strip_prefix("/data/repos/my-repo-").unwrap();
        assert_eq!(rest.len(), 8, "{}", target.dest);
        assert!(rest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(
            target,
            CloneTarget::for_url("https://github.com/other/My_Repo.git")
        );
        assert!(CloneTarget::for_url("")
            .dest
            .starts_with("/data/repos/repo-"));
    }

    #[test]
    fn git_stderr_classifies_and_reads_as_the_contract_says() {
        let url = "https://h/r.git";
        let cases = [
            (
                "fatal: could not read Username for 'https://h': terminal prompts disabled",
                CloneFailure::AuthRequired { remote: url.into() },
                "cloning https://h/r.git needs a credential: allow Git credentials or an SSH key for Dev",
            ),
            (
                "git@h: Permission denied (publickey).\nfatal: Could not read from remote repository.",
                CloneFailure::AuthRequired { remote: url.into() },
                "cloning https://h/r.git needs a credential: allow Git credentials or an SSH key for Dev",
            ),
            (
                "error: unable to write file: No space left on device",
                CloneFailure::DiskFull,
                "the volume of Dev is full",
            ),
            (
                "fatal: unable to access 'https://h/r.git/': Could not resolve host: h",
                CloneFailure::Unreachable { remote: url.into() },
                "Dev could not reach https://h/r.git",
            ),
        ];
        for (stderr, failure, text) in cases {
            let got = CloneFailure::from_stderr(stderr, url);
            assert_eq!(got, failure, "{stderr}");
            assert_eq!(got.message("Dev"), text);
        }
        assert_eq!(
            CloneFailure::from_stderr("fatal: weird\n", url),
            CloneFailure::Other("fatal: weird".into())
        );
    }
}
