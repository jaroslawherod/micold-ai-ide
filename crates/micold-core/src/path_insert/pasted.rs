//! Where a pasted clipboard image is saved, and what it is called (feature 487, FR-007, FR-008).
//!
//! Pure: it names places and files and writes nothing. A session's pasted images live in one
//! directory keyed by the session's id, so deleting the session can remove them and no two
//! sessions share a file (data-model: `PastedLayout`).

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
}
