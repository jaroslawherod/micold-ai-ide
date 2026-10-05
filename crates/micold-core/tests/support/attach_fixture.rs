//! Shared fixture for the attach tests (feature 582, T002).
//!
//! A scratch git repository with real worktrees under `.claude/worktrees/`, one worktree whose
//! directory has been deleted behind git's back (prunable), a sibling repository whose path
//! differs only by a suffix (`/a/proj` vs `/a/proj-x`), and a fake provider store under a scratch
//! home seeded for both repositories. Included with
//! `#[path = "support/attach_fixture.rs"] mod attach_fixture;`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use uuid::Uuid;

/// Run `git -C dir args`, panicking with git's stderr on failure.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository with an initial commit.
pub fn init_repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "t@t.test"]);
    git(dir, &["config", "user.name", "T"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "root"]);
}

/// The directory name the Claude Code store uses for `cwd`: every non-alphanumeric char is `-`.
pub fn encoded(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The scratch world.
pub struct AttachFixture {
    /// Keeps the scratch directory alive.
    pub tmp: TempDir,
    /// The project repository (`<tmp>/proj`).
    pub repo: PathBuf,
    /// A sibling repository whose path starts with the project's (`<tmp>/proj-x`).
    pub sibling: PathBuf,
    /// The scratch home holding `.claude/projects/`.
    pub home: PathBuf,
}

impl AttachFixture {
    /// A repository with one valid worktree per name in `worktrees`, plus a sibling repository.
    pub fn new(worktrees: &[&str]) -> Self {
        let tmp = TempDir::new().unwrap();
        // On Windows `canonicalize` yields a `\\?\` verbatim path, which git cannot create
        // worktrees under ("could not create leading directories"); strip the prefix.
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();
        let base = canonical
            .to_str()
            .and_then(|p| p.strip_prefix(r"\\?\"))
            .map_or(canonical.clone(), PathBuf::from);
        let repo = base.join("proj");
        let sibling = base.join("proj-x");
        let home = base.join("home");
        init_repo(&repo);
        init_repo(&sibling);
        std::fs::create_dir_all(home.join(".claude/projects")).unwrap();
        let fx = Self {
            tmp,
            repo,
            sibling,
            home,
        };
        for name in worktrees {
            fx.add_worktree(name);
        }
        fx
    }

    /// The path of the worktree directory `dir_name`.
    pub fn worktree_path(&self, dir_name: &str) -> PathBuf {
        self.repo.join(".claude/worktrees").join(dir_name)
    }

    /// Create a real worktree on branch `worktree-<dir_name>`, made behind the app's back.
    pub fn add_worktree(&self, dir_name: &str) -> PathBuf {
        let target = self.worktree_path(dir_name);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        git(
            &self.repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                &format!("worktree-{dir_name}"),
                target.to_str().unwrap(),
            ],
        );
        target
    }

    /// A worktree git still lists but whose directory is gone (prunable).
    pub fn add_missing_worktree(&self, dir_name: &str) -> PathBuf {
        let target = self.add_worktree(dir_name);
        std::fs::remove_dir_all(&target).unwrap();
        target
    }

    /// Write one Claude Code transcript for `cwd` and return its id.
    pub fn seed_session(&self, cwd: &Path) -> Uuid {
        let id = Uuid::new_v4();
        let dir = self.home.join(".claude/projects").join(encoded(cwd));
        std::fs::create_dir_all(&dir).unwrap();
        let line = format!(
            "{{\"type\":\"user\",\"cwd\":{},\"sessionId\":\"{id}\",\"message\":{{\"role\":\"user\",\"content\":\"hello\"}}}}\n",
            serde_json::to_string(&cwd.to_string_lossy()).unwrap()
        );
        std::fs::write(dir.join(format!("{id}.jsonl")), line).unwrap();
        id
    }
}
