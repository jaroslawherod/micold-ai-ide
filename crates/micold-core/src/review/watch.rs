//! What a Changes view's file watch reacts to (feature 482, research R9): which changed paths
//! matter ([`relevant_paths`]) and when a burst of them is over ([`Debouncer`]). Pure: the
//! client's subscription owns the watcher and calls these.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long the watch waits without a new event before it releases a batch (R9).
pub const QUIET: Duration = Duration::from_millis(300);

/// The paths of `paths` that can change what the view shows, in their order.
///
/// `entry_root` is the entry's directory; `git_dirs` its git metadata directories (its own git
/// dir and the common dir). Under a git dir only `HEAD`, `index`, `packed-refs` and `refs/` count
/// (stage, commit, branch moves), never a `.lock` file; under the root every other path counts
/// except those under `.claude/worktrees/` (another entry's files, for the Default entry).
pub fn relevant_paths(
    entry_root: &Path,
    git_dirs: &[PathBuf],
    paths: Vec<PathBuf>,
) -> Vec<PathBuf> {
    let worktrees = entry_root.join(".claude").join("worktrees");
    paths
        .into_iter()
        .filter(|path| {
            // The innermost git dir decides: a worktree's own git dir sits inside the common one.
            let git_dir = git_dirs
                .iter()
                .filter(|dir| path.starts_with(dir))
                .max_by_key(|dir| dir.components().count());
            match git_dir {
                Some(dir) => path.strip_prefix(dir).is_ok_and(is_git_state),
                None => {
                    path.starts_with(entry_root)
                        && path != entry_root
                        && !path.starts_with(&worktrees)
                }
            }
        })
        .collect()
}

/// A path inside a git dir that records what is staged, committed or checked out.
fn is_git_state(relative: &Path) -> bool {
    if relative.extension().is_some_and(|ext| ext == "lock") {
        return false;
    }
    relative == Path::new("HEAD")
        || relative == Path::new("index")
        || relative == Path::new("packed-refs")
        || relative.starts_with("refs")
}

/// Collects a burst of changed paths and releases them once no event came for [`QUIET`].
#[derive(Debug, Default)]
pub struct Debouncer {
    pending: BTreeSet<PathBuf>,
    last: Option<Instant>,
}

impl Debouncer {
    /// Records `paths` changed at `now`.
    pub fn push(&mut self, paths: Vec<PathBuf>, now: Instant) {
        if paths.is_empty() {
            return;
        }
        self.pending.extend(paths);
        self.last = Some(now);
    }

    /// The batch, each path once, when [`QUIET`] has passed since the last event; else `None`.
    pub fn ready(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        let deadline = self.deadline()?;
        if now < deadline {
            return None;
        }
        self.last = None;
        Some(std::mem::take(&mut self.pending).into_iter().collect())
    }

    /// When the pending batch will be ready, if there is one.
    pub fn deadline(&self) -> Option<Instant> {
        self.last.map(|last| last + QUIET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/repo";

    fn git_dirs() -> Vec<PathBuf> {
        vec![PathBuf::from("/repo/.git")]
    }

    fn kept(paths: &[&str]) -> Vec<PathBuf> {
        relevant_paths(
            Path::new(ROOT),
            &git_dirs(),
            paths.iter().map(PathBuf::from).collect(),
        )
    }

    fn paths(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn git_objects_logs_and_lock_files_are_dropped() {
        assert_eq!(
            kept(&[
                "/repo/.git/objects/ab/cdef",
                "/repo/.git/logs/HEAD",
                "/repo/.git/index.lock",
                "/repo/.git/refs/heads/main.lock",
            ]),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn head_index_refs_and_packed_refs_are_kept() {
        let changed = [
            "/repo/.git/HEAD",
            "/repo/.git/index",
            "/repo/.git/refs/heads/main",
            "/repo/.git/packed-refs",
        ];
        assert_eq!(kept(&changed), paths(&changed));
    }

    #[test]
    fn worktree_files_are_kept_even_a_lock_file_of_the_project() {
        let changed = ["/repo/src/main.rs", "/repo/Cargo.lock", "/repo/new.txt"];
        assert_eq!(kept(&changed), paths(&changed));
    }

    #[test]
    fn paths_outside_the_entry_and_its_git_dirs_are_dropped() {
        assert_eq!(kept(&["/elsewhere/a.rs", "/repo"]), Vec::<PathBuf>::new());
    }

    #[test]
    fn the_default_entry_ignores_its_worktrees() {
        assert_eq!(
            kept(&[
                "/repo/.claude/worktrees/feat/src/a.rs",
                "/repo/.claude/settings.json"
            ]),
            paths(&["/repo/.claude/settings.json"])
        );
    }

    #[test]
    fn a_worktree_entry_watches_its_own_git_dir_and_the_common_refs() {
        let root = Path::new("/repo/.claude/worktrees/feat");
        let dirs = vec![
            PathBuf::from("/repo/.git/worktrees/feat"),
            PathBuf::from("/repo/.git"),
        ];
        let changed = paths(&[
            "/repo/.claude/worktrees/feat/src/a.rs",
            "/repo/.git/worktrees/feat/HEAD",
            "/repo/.git/worktrees/feat/index",
            "/repo/.git/refs/heads/feat",
            "/repo/.git/worktrees/feat/logs/HEAD",
            "/repo/.git/worktrees/other/index",
        ]);
        assert_eq!(
            relevant_paths(root, &dirs, changed.clone()),
            changed[..4].to_vec()
        );
    }

    #[test]
    fn the_debouncer_waits_for_quiet() {
        let start = Instant::now();
        let mut debouncer = Debouncer::default();
        debouncer.push(paths(&["/repo/a"]), start);
        assert_eq!(debouncer.ready(start + Duration::from_millis(299)), None);
        assert_eq!(
            debouncer.deadline(),
            Some(start + QUIET),
            "ready at the quiet mark"
        );
        assert_eq!(
            debouncer.ready(start + QUIET),
            Some(paths(&["/repo/a"])),
            "released after 300 ms without an event"
        );
        assert_eq!(debouncer.ready(start + QUIET * 2), None, "released once");
        assert_eq!(debouncer.deadline(), None);
    }

    #[test]
    fn a_new_event_restarts_the_wait_and_repeated_paths_merge() {
        let start = Instant::now();
        let mut debouncer = Debouncer::default();
        debouncer.push(paths(&["/repo/b", "/repo/a"]), start);
        let later = start + Duration::from_millis(200);
        debouncer.push(paths(&["/repo/a"]), later);
        assert_eq!(
            debouncer.ready(start + QUIET),
            None,
            "the second event restarted the wait"
        );
        assert_eq!(
            debouncer.ready(later + QUIET),
            Some(paths(&["/repo/a", "/repo/b"])),
            "each path once"
        );
    }

    #[test]
    fn an_empty_push_is_not_an_event() {
        let start = Instant::now();
        let mut debouncer = Debouncer::default();
        debouncer.push(Vec::new(), start);
        assert_eq!(debouncer.deadline(), None);
        assert_eq!(debouncer.ready(start + QUIET), None);
    }
}
