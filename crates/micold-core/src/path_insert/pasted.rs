//! Where a pasted clipboard image is saved, and what it is called (feature 487, FR-007, FR-008).
//!
//! Pure: it names places and files and writes nothing. A session's pasted images live in one
//! directory keyed by the session's id, so deleting the session can remove them and no two
//! sessions share a file (data-model: `PastedLayout`).

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use crate::session::SessionId;

/// The directory inside a worktree that holds pasted images.
pub const WORKTREE_DIR: &str = ".micold-pasted";

/// The directory under the app's data directory that holds pasted images of sessions that have no
/// writable worktree.
pub const TEMP_DIR: &str = "pasted";

/// One session's place for pasted images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastedLayout {
    dir: PathBuf,
    in_worktree: bool,
}

impl PastedLayout {
    /// Inside `worktree`: `<worktree>/.micold-pasted/<session-id>`.
    pub fn in_worktree(worktree: &Path, session: SessionId) -> Self {
        Self {
            dir: worktree.join(WORKTREE_DIR).join(session.to_string()),
            in_worktree: true,
        }
    }

    /// Outside any worktree: `<data_dir>/pasted/<session-id>`. For a session with no worktree, or
    /// one whose worktree cannot be written.
    pub fn in_data_dir(data_dir: &Path, session: SessionId) -> Self {
        Self {
            dir: data_dir.join(TEMP_DIR).join(session.to_string()),
            in_worktree: false,
        }
    }

    /// The session's directory for pasted images.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Whether the directory is inside a worktree, and so must be kept out of its version control
    /// status (FR-008).
    pub fn in_worktree_dir(&self) -> bool {
        self.in_worktree
    }

    /// A new file name in [`Self::dir`]: `<unix-nanos>-<seq>.png`. `seq` is a counter the caller
    /// bumps per paste, so two pastes in the same nanosecond still differ (US2.5).
    pub fn next_file(&self, now_nanos: u128, seq: u64) -> PathBuf {
        self.dir.join(format!("{now_nanos}-{seq}.png"))
    }

    /// The line that keeps pasted images out of a repository's status.
    pub fn exclude_line() -> &'static str {
        "/.micold-pasted/"
    }

    /// Remove this session's directory with everything in it. A directory that is not there is
    /// fine. Only a directory named for a session under a `pasted` root is ever removed (FR-013).
    pub fn remove(&self) -> io::Result<()> {
        if !is_pasted_root(self.dir.parent()) {
            return Ok(());
        }
        match std::fs::remove_dir_all(&self.dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// The per-session directories under `roots` that belong to no session in `live` (FR-014,
    /// SC-005). A root is a directory named [`WORKTREE_DIR`] or [`TEMP_DIR`]; any other path is
    /// ignored, as is an entry whose name is not a session id and a symbolic link.
    pub fn orphans(roots: &[PathBuf], live: &HashSet<SessionId>) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for root in roots {
            if !is_pasted_root(Some(root)) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if !kind.is_dir() {
                    continue;
                }
                let name = entry.file_name();
                let Some(id) = name.to_str().and_then(|n| n.parse::<uuid::Uuid>().ok()) else {
                    continue;
                };
                if !live.contains(&SessionId::from_uuid(id)) {
                    found.push(entry.path());
                }
            }
        }
        found.sort();
        found
    }
}

/// Every place pasted images may be kept: `.micold-pasted` in each of `worktrees`, and `pasted`
/// under `data_dir`. What [`PastedLayout::orphans`] searches.
pub fn pasted_roots(worktrees: &[PathBuf], data_dir: Option<&Path>) -> Vec<PathBuf> {
    worktrees
        .iter()
        .map(|w| w.join(WORKTREE_DIR))
        .chain(data_dir.map(|d| d.join(TEMP_DIR)))
        .collect()
}

/// Whether `dir` is a place pasted images live in, by its name.
fn is_pasted_root(dir: Option<&Path>) -> bool {
    dir.and_then(Path::file_name)
        .is_some_and(|n| n == WORKTREE_DIR || n == TEMP_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_worktree_session_pastes_inside_its_worktree() {
        let id = SessionId::new();
        let l = PastedLayout::in_worktree(Path::new("/r/wt"), id);
        assert_eq!(
            l.dir(),
            Path::new("/r/wt/.micold-pasted").join(id.to_string())
        );
        assert!(l.in_worktree_dir());
    }

    #[test]
    fn a_session_without_a_worktree_pastes_under_the_data_dir() {
        let id = SessionId::new();
        let l = PastedLayout::in_data_dir(Path::new("/d"), id);
        assert_eq!(l.dir(), Path::new("/d/pasted").join(id.to_string()));
        assert!(!l.in_worktree_dir());
    }

    #[test]
    fn two_pastes_in_the_same_nanosecond_get_different_names() {
        let l = PastedLayout::in_data_dir(Path::new("/d"), SessionId::new());
        assert_ne!(l.next_file(5, 1), l.next_file(5, 2));
        assert_eq!(l.next_file(5, 1).parent(), Some(l.dir()));
        assert!(l.next_file(5, 1).to_string_lossy().ends_with("5-1.png"));
    }

    #[test]
    fn the_exclude_line_names_the_worktree_dir() {
        assert_eq!(PastedLayout::exclude_line(), "/.micold-pasted/");
    }

    fn make(dir: &Path, name: &str) -> PathBuf {
        let d = dir.join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("1-1.png"), b"x").unwrap();
        d
    }

    #[test]
    fn orphans_are_the_dead_sessions_dirs_in_both_kinds_of_root() {
        let tmp = tempfile::tempdir().unwrap();
        let (live, dead, dead_wt) = (SessionId::new(), SessionId::new(), SessionId::new());
        let data = tmp.path().join("data").join(TEMP_DIR);
        let wt = tmp.path().join("wt").join(WORKTREE_DIR);
        make(&data, &live.to_string());
        let d1 = make(&data, &dead.to_string());
        let d2 = make(&wt, &dead_wt.to_string());
        let found = PastedLayout::orphans(&[data, wt], &HashSet::from([live]));
        let mut want = vec![d1, d2];
        want.sort();
        assert_eq!(found, want);
    }

    #[test]
    fn orphans_never_name_a_path_outside_a_pasted_root() {
        let tmp = tempfile::tempdir().unwrap();
        let other = tmp.path().join("projects");
        let stray = make(&other, &SessionId::new().to_string());
        let root = tmp.path().join(TEMP_DIR);
        make(&root, "notes");
        std::fs::write(root.join(SessionId::new().to_string()), b"a file").unwrap();
        assert!(PastedLayout::orphans(&[other, root.clone()], &HashSet::new()).is_empty());
        assert!(stray.exists() && root.join("notes").exists());
    }

    #[test]
    fn a_missing_root_has_no_orphans() {
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join(TEMP_DIR);
        assert!(PastedLayout::orphans(&[gone], &HashSet::new()).is_empty());
    }

    #[test]
    fn remove_deletes_the_session_dir_only() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (SessionId::new(), SessionId::new());
        let la = PastedLayout::in_data_dir(tmp.path(), a);
        let lb = PastedLayout::in_data_dir(tmp.path(), b);
        make(&tmp.path().join(TEMP_DIR), &a.to_string());
        let kept = make(&tmp.path().join(TEMP_DIR), &b.to_string());
        la.remove().unwrap();
        assert!(!la.dir().exists());
        assert!(kept.exists());
        la.remove().unwrap();
        drop(lb);
    }
}
