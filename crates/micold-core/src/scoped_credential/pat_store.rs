//! The pasted PATs on the user's device (#1002, FR-005, contracts/pat-source.md §3, research R5):
//! `<root>/github-pat/<owner>/<repo>`, owner-only, two lines `expires_at=<unix>` and `token=<token>`.
//! The root is [`crate::paths::secrets_dir`], a sibling of the data directory that no container
//! mounts. A token is read back only as a [`SecretToken`].

use std::io;
use std::path::{Path, PathBuf};

use super::{SecretToken, UnixTime};
use crate::github::GithubRepo;
use crate::owner_only;

/// The directory under the root that holds the PATs.
const DIR: &str = "github-pat";

/// One stored PAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPat {
    /// The token.
    pub token: SecretToken,
    /// When GitHub said it expires, at the last check.
    pub expires_at: UnixTime,
}

impl StoredPat {
    fn to_text(&self) -> String {
        format!(
            "expires_at={}\ntoken={}\n",
            self.expires_at,
            self.token.expose()
        )
    }

    fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let expires_at = lines.next()?.strip_prefix("expires_at=")?.parse().ok()?;
        let token = lines.next()?.strip_prefix("token=")?;
        (!token.is_empty() && lines.next().is_none()).then(|| Self {
            token: SecretToken::new(token),
            expires_at,
        })
    }
}

/// The PATs under one root.
#[derive(Debug, Clone)]
pub struct PatStore {
    root: PathBuf,
}

impl PatStore {
    /// A store under `root` (nothing is created until the first [`PatStore::put`]).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The store under the device's [`crate::paths::secrets_dir`], when it has one.
    pub fn on_device() -> Option<Self> {
        crate::paths::secrets_dir().map(Self::new)
    }

    fn owner_dir(&self, repo: &GithubRepo) -> PathBuf {
        self.root.join(DIR).join(repo.owner().to_ascii_lowercase())
    }

    fn file_name(repo: &GithubRepo) -> String {
        repo.name().to_ascii_lowercase()
    }

    /// Store `pat` for `repo`, owner-only, replacing an earlier one.
    pub fn put(&self, repo: &GithubRepo, pat: &StoredPat) -> io::Result<()> {
        owner_only::write(
            &self.owner_dir(repo),
            &Self::file_name(repo),
            pat.to_text().as_bytes(),
        )
        .map(|_| ())
    }

    /// The PAT of `repo`; `None` when there is none or its file is unreadable.
    pub fn get(&self, repo: &GithubRepo) -> Option<StoredPat> {
        let path = self.owner_dir(repo).join(Self::file_name(repo));
        StoredPat::from_text(&std::fs::read_to_string(path).ok()?)
    }

    /// Remove the PAT of `repo`; removing one that is not there is not an error.
    pub fn remove(&self, repo: &GithubRepo) -> io::Result<()> {
        match std::fs::remove_file(self.owner_dir(repo).join(Self::file_name(repo))) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Every stored PAT with its repository, sorted by repository. A file that is not a PAT, or
    /// whose name is no repository, is skipped.
    pub fn list(&self) -> Vec<(GithubRepo, StoredPat)> {
        let mut out = vec![];
        for owner in subdirs(&self.root.join(DIR)) {
            for file in files(&owner) {
                let (Some(o), Some(n)) = (file_name(&owner), file_name(&file)) else {
                    continue;
                };
                let Some(repo) =
                    GithubRepo::from_remote_url(&format!("https://github.com/{o}/{n}"))
                else {
                    continue;
                };
                if let Some(pat) = std::fs::read_to_string(&file)
                    .ok()
                    .and_then(|t| StoredPat::from_text(&t))
                {
                    out.push((repo, pat));
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// The root, for a test.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(str::to_string)
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    entries(dir).into_iter().filter(|p| p.is_dir()).collect()
}

fn files(dir: &Path) -> Vec<PathBuf> {
    entries(dir)
        .into_iter()
        .filter(|p| p.is_file() && !file_name(p).is_some_and(|n| n.starts_with('.')))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(url: &str) -> GithubRepo {
        GithubRepo::from_remote_url(url).unwrap()
    }

    fn pat(token: &str, expires_at: UnixTime) -> StoredPat {
        StoredPat {
            token: SecretToken::new(token),
            expires_at,
        }
    }

    #[test]
    fn a_put_pat_is_read_back_and_replaced_by_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        let r = repo("https://github.com/Acme/App");
        assert_eq!(store.get(&r), None);
        store.put(&r, &pat("github_pat_one", 10)).unwrap();
        assert_eq!(store.get(&r), Some(pat("github_pat_one", 10)));
        store.put(&r, &pat("github_pat_two", 20)).unwrap();
        assert_eq!(
            store.get(&repo("https://github.com/acme/app")),
            Some(pat("github_pat_two", 20))
        );
        let text = std::fs::read_to_string(dir.path().join("github-pat/acme/app")).unwrap();
        assert_eq!(text, "expires_at=20\ntoken=github_pat_two\n");
    }

    #[cfg(unix)]
    #[test]
    fn the_file_and_its_directories_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path().join("secrets"));
        store
            .put(&repo("https://github.com/o/r"), &pat("github_pat_x", 1))
            .unwrap();
        let mode = |p: PathBuf| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(store.root().join("github-pat/o/r")), 0o600);
        assert_eq!(mode(store.root().join("github-pat/o")), 0o700);
    }

    #[test]
    fn remove_is_idempotent_and_list_shows_what_is_left() {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        let (a, b) = (
            repo("https://github.com/o/a"),
            repo("https://github.com/p/b"),
        );
        store.put(&b, &pat("github_pat_b", 2)).unwrap();
        store.put(&a, &pat("github_pat_a", 1)).unwrap();
        assert_eq!(
            store.list(),
            vec![
                (a.clone(), pat("github_pat_a", 1)),
                (b.clone(), pat("github_pat_b", 2))
            ]
        );
        store.remove(&a).unwrap();
        store.remove(&a).unwrap();
        assert_eq!(store.list(), vec![(b, pat("github_pat_b", 2))]);
        assert_eq!(PatStore::new(dir.path().join("none")).list(), vec![]);
    }

    #[test]
    fn a_name_that_climbs_out_is_no_repository_and_a_junk_file_is_skipped() {
        for url in [
            "https://github.com/o/..",
            "https://github.com/../r",
            "https://github.com/o/a%2f..%2fb",
        ] {
            assert!(GithubRepo::from_remote_url(url).is_none(), "{url}");
        }
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        std::fs::create_dir_all(dir.path().join("github-pat/o")).unwrap();
        std::fs::write(dir.path().join("github-pat/o/junk"), "not a pat").unwrap();
        assert_eq!(store.list(), vec![]);
        assert_eq!(store.get(&repo("https://github.com/o/junk")), None);
    }

    #[test]
    fn the_stored_text_never_shows_the_token_in_debug() {
        let shown = format!("{:?}", pat("github_pat_secret", 5));
        assert!(!shown.contains("github_pat_secret"), "{shown}");
    }
}
