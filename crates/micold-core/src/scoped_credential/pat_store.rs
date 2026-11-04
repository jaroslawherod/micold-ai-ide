//! The pasted PATs on the user's device (#1002, #1529, FR-005, FR-007, contracts/pat-source.md §3,
//! research R5): `<root>/github-pat/<name>`, owner-only, the lines `expires_at=<unix>`, an optional
//! `repos=<pattern>,<pattern>` and `token=<token>`. A token is stored by the name the user gave it
//! and may reach any number of repositories; `repos` narrows it (`owner/repo`, `owner/*`; absent =
//! every repository the token reaches). The order the tokens were added in, or moved to, is kept in
//! `github-pat/.order` and decides which of two equally specific tokens a repository uses.
//! The root is [`crate::paths::secrets_dir`], a sibling of the data directory that no container
//! mounts. A token is read back only as a [`SecretToken`].
//!
//! Before #1529 a token was stored per repository, `github-pat/<owner>/<repo>`;
//! [`PatStore::migrate`] turns each into a token named `<owner>-<repo>` with `repos = ["owner/repo"]`.

use std::io;
use std::path::{Path, PathBuf};

use super::{SecretToken, UnixTime};
use crate::daemon_config::RepoPattern;
use crate::github::GithubRepo;
use crate::owner_only;

/// The directory under the root that holds the PATs.
const DIR: &str = "github-pat";

/// The file in [`DIR`] that holds the tokens' order, one name a line.
const ORDER: &str = ".order";

/// The longest token name.
pub const MAX_NAME_LEN: usize = 64;

/// Whether `name` is usable as a token's name (and so a file name): 1 to [`MAX_NAME_LEN`] letters,
/// digits, `-`, `_` or `.`, not starting with `.`.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// One stored PAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPat {
    /// The token.
    pub token: SecretToken,
    /// When GitHub said it expires, at the last check.
    pub expires_at: UnixTime,
    /// The repositories it is narrowed to; empty = every repository the token reaches.
    pub repos: Vec<RepoPattern>,
}

impl StoredPat {
    fn to_text(&self) -> String {
        let repos = if self.repos.is_empty() {
            String::new()
        } else {
            let list: Vec<String> = self.repos.iter().map(ToString::to_string).collect();
            format!("repos={}\n", list.join(","))
        };
        format!(
            "expires_at={}\n{repos}token={}\n",
            self.expires_at,
            self.token.expose()
        )
    }

    fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let expires_at = lines.next()?.strip_prefix("expires_at=")?.parse().ok()?;
        let mut line = lines.next()?;
        let mut repos = vec![];
        if let Some(list) = line.strip_prefix("repos=") {
            for pattern in list.split(',') {
                repos.push(RepoPattern::parse(pattern)?);
            }
            line = lines.next()?;
        }
        let token = line.strip_prefix("token=")?;
        (!token.is_empty() && lines.next().is_none()).then(|| Self {
            token: SecretToken::new(token),
            expires_at,
            repos,
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

    /// The store under the device's [`crate::paths::secrets_dir`], when it has one. Tokens an
    /// earlier version stored per repository are migrated first ([`PatStore::migrate`]).
    pub fn on_device() -> Option<Self> {
        let store = crate::paths::secrets_dir().map(Self::new)?;
        let _ = store.migrate();
        Some(store)
    }

    fn dir(&self) -> PathBuf {
        self.root.join(DIR)
    }

    /// The file the token `name` is (or would be) kept in, which is what `[[github.tokens]]`
    /// names as `token_file`; `None` for a name that is no token name.
    pub fn token_file(&self, name: &str) -> Option<PathBuf> {
        valid_name(name).then(|| self.dir().join(name.to_ascii_lowercase()))
    }

    fn order(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir().join(ORDER))
            .map(|t| t.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// Keep `names` as the tokens' order.
    fn write_order(&self, names: &[String]) -> io::Result<()> {
        let text: String = names.iter().map(|n| format!("{n}\n")).collect();
        owner_only::write(&self.dir(), ORDER, text.as_bytes()).map(|_| ())
    }

    /// Store `pat` as the token `name`, owner-only, replacing an earlier one of that name (which
    /// keeps its place); a new name goes last.
    pub fn put(&self, name: &str, pat: &StoredPat) -> io::Result<()> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "not a valid token name");
        let path = self.token_file(name).ok_or_else(invalid)?;
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(invalid)?;
        owner_only::write(&self.dir(), file, pat.to_text().as_bytes())?;
        let mut order = self.order();
        if !order.iter().any(|n| n == file) {
            order.push(file.to_string());
            self.write_order(&order)?;
        }
        Ok(())
    }

    /// The token `name`; `None` when there is none or its file is unreadable.
    pub fn get(&self, name: &str) -> Option<StoredPat> {
        StoredPat::from_text(&std::fs::read_to_string(self.token_file(name)?).ok()?)
    }

    /// Remove the token `name`; removing one that is not there is not an error.
    pub fn remove(&self, name: &str) -> io::Result<()> {
        let Some(path) = self.token_file(name) else {
            return Ok(());
        };
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        let name = name.to_ascii_lowercase();
        let order: Vec<String> = self.order().into_iter().filter(|n| *n != name).collect();
        if self.dir().join(ORDER).exists() {
            self.write_order(&order)?;
        }
        Ok(())
    }

    /// Keep the tokens in `names`' order; a name that is not stored is ignored, and a stored token
    /// not named goes after the others.
    pub fn set_order(&self, names: &[String]) -> io::Result<()> {
        let stored: Vec<String> = self.list().into_iter().map(|(n, _)| n).collect();
        let mut order: Vec<String> = names
            .iter()
            .map(|n| n.to_ascii_lowercase())
            .filter(|n| stored.contains(n))
            .collect();
        order.dedup();
        for name in stored {
            if !order.contains(&name) {
                order.push(name);
            }
        }
        self.write_order(&order)
    }

    /// Every stored token with its name, in order: the order file's, then any token it does not
    /// name by name. A file that is not a PAT is skipped.
    pub fn list(&self) -> Vec<(String, StoredPat)> {
        let mut found: Vec<(String, StoredPat)> = files(&self.dir())
            .into_iter()
            .filter_map(|file| {
                let name = file_name(&file).filter(|n| valid_name(n))?;
                let pat = StoredPat::from_text(&std::fs::read_to_string(&file).ok()?)?;
                Some((name, pat))
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        let order = self.order();
        let rank = |name: &str| order.iter().position(|n| n == name).unwrap_or(usize::MAX);
        found.sort_by_key(|(name, _)| rank(name));
        found
    }

    /// Turn each token an earlier version stored per repository, `github-pat/<owner>/<repo>`, into
    /// the token `<owner>-<repo>` narrowed to `owner/repo`, and delete the old file. Behaviour does
    /// not change: that repository uses that token. A name already taken gets `-2`, `-3`, ...
    /// Returns how many moved.
    pub fn migrate(&self) -> io::Result<usize> {
        let mut moved = 0;
        for owner_dir in subdirs(&self.dir()) {
            for file in files(&owner_dir) {
                let (Some(o), Some(n)) = (file_name(&owner_dir), file_name(&file)) else {
                    continue;
                };
                let (Some(repo), Some(mut pat)) = (
                    GithubRepo::from_remote_url(&format!("https://github.com/{o}/{n}")),
                    std::fs::read_to_string(&file)
                        .ok()
                        .and_then(|t| StoredPat::from_text(&t)),
                ) else {
                    continue;
                };
                let repo = repo.lowercased();
                pat.repos = vec![RepoPattern::Exact(
                    repo.owner().to_string(),
                    repo.name().to_string(),
                )];
                let base = format!("{}-{}", repo.owner(), repo.name());
                let mut name = base.clone();
                let mut suffix = 2;
                while self.token_file(&name).is_none_or(|p| p.exists()) {
                    name = format!("{base}-{suffix}");
                    suffix += 1;
                }
                self.put(&name, &pat)?;
                std::fs::remove_file(&file)?;
                moved += 1;
            }
            // Only an emptied directory goes; `remove_dir` refuses one that still holds a file.
            let _ = std::fs::remove_dir(&owner_dir);
        }
        Ok(moved)
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

    fn pat(token: &str, expires_at: UnixTime) -> StoredPat {
        StoredPat {
            token: SecretToken::new(token),
            expires_at,
            repos: vec![],
        }
    }

    fn legacy(root: &Path, owner: &str, repo: &str, token: &str, expires_at: UnixTime) {
        let dir = root.join("github-pat").join(owner);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(repo),
            format!("expires_at={expires_at}\ntoken={token}\n"),
        )
        .unwrap();
    }

    #[test]
    fn a_token_is_stored_by_name_and_replaced_by_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        assert_eq!(store.get("work"), None);
        store.put("work", &pat("github_pat_one", 10)).unwrap();
        assert_eq!(store.get("work"), Some(pat("github_pat_one", 10)));
        store.put("Work", &pat("github_pat_two", 20)).unwrap();
        assert_eq!(store.get("work"), Some(pat("github_pat_two", 20)));
        let text = std::fs::read_to_string(dir.path().join("github-pat/work")).unwrap();
        assert_eq!(text, "expires_at=20\ntoken=github_pat_two\n");
    }

    #[test]
    fn the_repository_filter_is_kept_with_the_token() {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        let mut filtered = pat("github_pat_x", 5);
        filtered.repos = vec![
            RepoPattern::parse("Acme/App").unwrap(),
            RepoPattern::parse("acme/*").unwrap(),
        ];
        store.put("acme", &filtered).unwrap();
        let text = std::fs::read_to_string(dir.path().join("github-pat/acme")).unwrap();
        assert_eq!(
            text,
            "expires_at=5\nrepos=acme/app,acme/*\ntoken=github_pat_x\n"
        );
        assert_eq!(store.get("acme"), Some(filtered));
    }

    #[cfg(unix)]
    #[test]
    fn the_file_and_its_directory_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path().join("secrets"));
        store.put("x", &pat("github_pat_x", 1)).unwrap();
        let mode = |p: PathBuf| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(store.root().join("github-pat/x")), 0o600);
        assert_eq!(mode(store.root().join("github-pat")), 0o700);
    }

    #[test]
    fn tokens_list_in_the_order_they_were_added_and_moved_to() {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        for (name, n) in [("b", 2), ("a", 1), ("c", 3)] {
            store
                .put(name, &pat(&format!("github_pat_{name}"), n))
                .unwrap();
        }
        let names = |s: &PatStore| s.list().into_iter().map(|(n, _)| n).collect::<Vec<_>>();
        assert_eq!(names(&store), ["b", "a", "c"]);
        store.put("b", &pat("github_pat_b2", 9)).unwrap();
        assert_eq!(
            names(&store),
            ["b", "a", "c"],
            "a replaced token keeps its place"
        );
        store.set_order(&["c".into(), "b".into()]).unwrap();
        assert_eq!(names(&store), ["c", "b", "a"]);
        store.remove("c").unwrap();
        store.remove("c").unwrap();
        assert_eq!(names(&store), ["b", "a"]);
        assert_eq!(PatStore::new(dir.path().join("none")).list(), vec![]);
    }

    #[test]
    fn a_name_that_climbs_out_is_refused_and_a_junk_file_is_skipped() {
        for bad in ["", ".", "..", "../x", "a/b", ".order", "a b"] {
            assert!(!valid_name(bad), "{bad}");
        }
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        assert!(store.put("../x", &pat("github_pat_x", 1)).is_err());
        std::fs::create_dir_all(dir.path().join("github-pat")).unwrap();
        std::fs::write(dir.path().join("github-pat/junk"), "not a pat").unwrap();
        assert_eq!(store.list(), vec![]);
        assert_eq!(store.get("junk"), None);
    }

    #[test]
    fn each_per_repository_token_migrates_to_a_token_filtered_to_that_repository() {
        let dir = tempfile::tempdir().unwrap();
        legacy(dir.path(), "acme", "app", "github_pat_a", 10);
        legacy(dir.path(), "acme", "web", "github_pat_w", 20);
        let store = PatStore::new(dir.path());
        assert_eq!(store.migrate().unwrap(), 2);
        let exact = |o: &str, r: &str| vec![RepoPattern::Exact(o.into(), r.into())];
        assert_eq!(
            store.get("acme-app"),
            Some(StoredPat {
                repos: exact("acme", "app"),
                ..pat("github_pat_a", 10)
            })
        );
        assert_eq!(
            store.get("acme-web"),
            Some(StoredPat {
                repos: exact("acme", "web"),
                ..pat("github_pat_w", 20)
            })
        );
        assert!(
            !dir.path().join("github-pat/acme").exists(),
            "old files are gone"
        );
        assert_eq!(store.migrate().unwrap(), 0, "nothing is migrated twice");
    }

    #[test]
    fn a_migrated_name_never_replaces_a_token_that_has_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        store.put("acme-app", &pat("github_pat_mine", 1)).unwrap();
        legacy(dir.path(), "acme", "app", "github_pat_old", 2);
        assert_eq!(store.migrate().unwrap(), 1);
        assert_eq!(store.get("acme-app"), Some(pat("github_pat_mine", 1)));
        assert_eq!(store.get("acme-app-2").unwrap().expires_at, 2);
    }

    #[test]
    fn the_stored_text_never_shows_the_token_in_debug() {
        let shown = format!("{:?}", pat("github_pat_secret", 5));
        assert!(!shown.contains("github_pat_secret"), "{shown}");
    }
}
