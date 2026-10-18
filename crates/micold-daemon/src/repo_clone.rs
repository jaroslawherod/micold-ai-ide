//! `ClientMsg::RepoClone` (feature 686, FR-007): make sure a clone of a remote is in place on the
//! daemon's volume, atomically. A new clone is made in `.partial-<name>-<rand>` beside its
//! destination and renamed on success, so a failure leaves nothing half-cloned; an existing one is
//! fetched. Two requests for one destination are serialised, so the second finds the first's clone
//! and fetches (FR-023). Everything here is **blocking**: call it from `spawn_blocking`.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use micold_core::protocol::messages::CloneFailureWire;

/// The environment variable naming the directory every clone must lie under.
pub const ROOT_ENV: &str = "MICOLD_REPOS_ROOT";

/// The configured clone root, when the environment names one.
pub fn root_from_env() -> Option<PathBuf> {
    std::env::var_os(ROOT_ENV)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn lock_for(dest: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut all = LOCKS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Arc::clone(all.entry(dest.to_path_buf()).or_default())
}

fn refuse(text: impl Into<String>) -> CloneFailureWire {
    CloneFailureWire::Other(text.into())
}

/// `dest` as a path strictly inside `root`, with no `..` in it.
fn checked_dest(root: Option<&Path>, dest: &str) -> Result<PathBuf, CloneFailureWire> {
    let Some(root) = root else {
        return Err(refuse(format!(
            "this daemon keeps no clones: {ROOT_ENV} is not set"
        )));
    };
    let dest = PathBuf::from(dest);
    let plain = dest.components().all(|c| {
        matches!(
            c,
            Component::RootDir | Component::Normal(_) | Component::Prefix(_)
        )
    });
    if !dest.is_absolute() || !plain || dest == root || !dest.starts_with(root) {
        return Err(refuse(format!(
            "{} is outside {}",
            dest.display(),
            root.display()
        )));
    }
    Ok(dest)
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<(), CloneFailureWire> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    micold_core::process::no_window(&mut cmd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C");
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        cmd.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }
    let out = cmd
        .output()
        .map_err(|e| refuse(format!("could not run git: {e}")))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(CloneFailureWire::classify(&String::from_utf8_lossy(
            &out.stderr,
        )))
    }
}

/// Clone `url` to `dest`, or fetch when `dest` already holds a clone. `root` is the daemon's
/// [`ROOT_ENV`]; a `dest` outside it is refused before git runs.
pub fn ensure_clone(root: Option<&Path>, url: &str, dest: &str) -> Result<(), CloneFailureWire> {
    let dest = checked_dest(root, dest)?;
    let lock = lock_for(&dest);
    let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if dest.exists() {
        if !dest.join(".git").exists() {
            return Err(refuse(format!(
                "{} exists but is not a git repository",
                dest.display()
            )));
        }
        return git(Some(&dest), &["fetch", "--all", "--prune"]);
    }
    let Some(parent) = dest.parent() else {
        return Err(refuse("the destination has no parent directory"));
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| refuse(format!("could not create {}: {e}", parent.display())))?;
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let partial = parent.join(format!(".partial-{name}-{}-{nonce:x}", std::process::id()));
    let cloned = git(None, &["clone", "--", url, &partial.to_string_lossy()]);
    settle(cloned, &partial, &dest)
}

/// Move a finished clone into place, or remove the partial directory: a failure leaves nothing
/// half-cloned.
fn settle(
    cloned: Result<(), CloneFailureWire>,
    partial: &Path,
    dest: &Path,
) -> Result<(), CloneFailureWire> {
    let outcome = cloned.and_then(|()| {
        std::fs::rename(partial, dest)
            .map_err(|e| refuse(format!("could not move the clone into place: {e}")))
    });
    if outcome.is_err() {
        let _ = std::fs::remove_dir_all(partial);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }

    /// A local bare repository with one commit; returns its path.
    fn bare(base: &Path) -> PathBuf {
        let work = base.join("work");
        std::fs::create_dir_all(&work).unwrap();
        run(&work, &["init", "-q", "-b", "main"]);
        std::fs::write(work.join("a.txt"), "a").unwrap();
        run(&work, &["add", "."]);
        run(&work, &["commit", "-q", "-m", "one"]);
        let bare = base.join("remote.git");
        let ok = Command::new("git")
            .args(["clone", "-q", "--bare"])
            .arg(&work)
            .arg(&bare)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        bare
    }

    fn entries(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_new_clone_arrives_whole_and_leaves_no_partial_directory() {
        let base = tempfile::tempdir().unwrap();
        let remote = bare(base.path());
        let root = base.path().join("repos");
        let dest = root.join("app-1");
        ensure_clone(
            Some(&root),
            &remote.to_string_lossy(),
            &dest.to_string_lossy(),
        )
        .unwrap();
        assert!(dest.join("a.txt").is_file());
        assert_eq!(entries(&root), vec!["app-1".to_string()]);
    }

    #[test]
    fn an_existing_destination_is_fetched() {
        let base = tempfile::tempdir().unwrap();
        let remote = bare(base.path());
        let root = base.path().join("repos");
        let dest = root.join("app-1");
        let (url, dest_s) = (remote.to_string_lossy(), dest.to_string_lossy());
        ensure_clone(Some(&root), &url, &dest_s).unwrap();
        // A new commit reaches the remote through a second checkout.
        let other = base.path().join("other");
        let ok = Command::new("git")
            .args(["clone", "-q"])
            .arg(&remote)
            .arg(&other)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        std::fs::write(other.join("b.txt"), "b").unwrap();
        run(&other, &["add", "."]);
        run(&other, &["commit", "-q", "-m", "two"]);
        run(&other, &["push", "-q", "origin", "HEAD"]);
        ensure_clone(Some(&root), &url, &dest_s).unwrap();
        let log = Command::new("git")
            .arg("-C")
            .arg(&dest)
            .args(["log", "--oneline", "origin/main"])
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&log.stdout).lines().count(), 2);
    }

    #[test]
    fn two_requests_for_one_destination_are_serialised() {
        let base = tempfile::tempdir().unwrap();
        let remote = bare(base.path());
        let root = base.path().join("repos");
        let (url, dest) = (
            remote.to_string_lossy().into_owned(),
            root.join("app-1").to_string_lossy().into_owned(),
        );
        let results: Vec<_> = (0..2)
            .map(|_| {
                let (root, url, dest) = (root.clone(), url.clone(), dest.clone());
                std::thread::spawn(move || ensure_clone(Some(&root), &url, &dest))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        assert_eq!(results, vec![Ok(()), Ok(())]);
        assert_eq!(entries(&root), vec!["app-1".to_string()]);
    }

    #[test]
    fn a_failed_clone_leaves_no_directory() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("repos");
        let missing = base.path().join("nowhere.git");
        let dest = root.join("app-1");
        let err = ensure_clone(
            Some(&root),
            &missing.to_string_lossy(),
            &dest.to_string_lossy(),
        )
        .unwrap_err();
        assert!(matches!(err, CloneFailureWire::Other(_)), "{err:?}");
        assert!(entries(&root).is_empty(), "{:?}", entries(&root));
    }

    #[test]
    fn a_destination_outside_the_root_is_refused_before_git_runs() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("repos");
        for dest in [
            "/etc/x",
            root.to_str().unwrap(),
            &format!("{}/../escape", root.display()),
            "relative/x",
        ] {
            let err = ensure_clone(Some(&root), "u", dest).unwrap_err();
            assert!(
                matches!(&err, CloneFailureWire::Other(t) if t.contains("is outside")),
                "{dest}: {err:?}"
            );
        }
        assert!(matches!(
            ensure_clone(None, "u", "/data/repos/x"),
            Err(CloneFailureWire::Other(t)) if t.contains(ROOT_ENV)
        ));
        assert!(!root.exists());
    }

    #[test]
    fn a_failed_clone_removes_its_partial_directory_and_a_good_one_moves_into_place() {
        let base = tempfile::tempdir().unwrap();
        let (partial, dest) = (base.path().join(".partial-x"), base.path().join("x"));
        std::fs::create_dir_all(&partial).unwrap();
        std::fs::write(partial.join("f"), "f").unwrap();
        let failed = settle(Err(refuse("boom")), &partial, &dest);
        assert!(failed.is_err());
        assert!(!partial.exists() && !dest.exists());
        std::fs::create_dir_all(&partial).unwrap();
        std::fs::write(partial.join("f"), "f").unwrap();
        settle(Ok(()), &partial, &dest).unwrap();
        assert!(!partial.exists() && dest.join("f").is_file());
    }
}
