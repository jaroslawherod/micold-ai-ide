//! Path resolution and containment for the worktree guard (research R4, FR-012, FR-014).
//!
//! A path becomes a [`ResolvedPath`]: lexical `.`/`..` removal, then the deepest existing ancestor
//! canonicalised (symlinks), then drive letter, separators and (on a case-insensitive volume) case
//! normalised. Containment is a **component-wise** prefix test, never a string one, so `/a/wt` does
//! not contain `/a/wt2`. Separator handling is the same on every platform, so one logical layout
//! gives one decision on Linux, macOS and Windows (FR-014); what differs per platform is only what
//! [`PathFacts`] reports about the file system.

use std::path::{Path, PathBuf};

/// What the guard needs to know about the file system, injected so tests can stand in for any
/// platform.
pub trait PathFacts {
    /// `path` with the deepest ancestor that exists canonicalised (symlinks followed) and the
    /// not-yet-existing rest appended unchanged.
    fn canonicalize_existing_prefix(&self, path: &Path) -> PathBuf;

    /// Whether the volume holding `path` ignores case.
    fn case_insensitive(&self, path: &Path) -> bool;
}

/// [`PathFacts`] over the real file system. Case-insensitivity is the platform default (Windows and
/// macOS volumes), not a probe of the volume.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealPathFacts;

impl PathFacts for RealPathFacts {
    fn canonicalize_existing_prefix(&self, path: &Path) -> PathBuf {
        let mut rest = Vec::new();
        let mut current = path;
        loop {
            if let Ok(mut canonical) = std::fs::canonicalize(current) {
                canonical.extend(rest.iter().rev());
                return canonical;
            }
            match (current.parent(), current.file_name()) {
                (Some(parent), Some(name)) => {
                    rest.push(name.to_owned());
                    current = parent;
                }
                _ => return path.to_path_buf(),
            }
        }
    }

    fn case_insensitive(&self, _path: &Path) -> bool {
        cfg!(any(windows, target_os = "macos"))
    }
}

/// A path in the one form containment is tested on: its root marker (`/` or an upper-case drive
/// like `C:`) then its components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPath(Vec<String>);

impl ResolvedPath {
    /// Whether `other` is this path or lies under it, component by component.
    pub fn contains(&self, other: &Self) -> bool {
        other.0.starts_with(&self.0)
    }

    /// The final component (the directory name), if the path has more than a root.
    pub fn last(&self) -> Option<&str> {
        self.0.last().map(String::as_str).filter(|c| !is_root(c))
    }

    /// Number of components, the root marker included. A deeper match is a more specific one.
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// This path with `name` (which may hold `/`) appended.
    pub fn join(&self, name: &str) -> Self {
        let mut parts = self.0.clone();
        parts.extend(
            name.split('/')
                .filter(|c| !c.is_empty() && *c != ".")
                .map(str::to_owned),
        );
        Self(parts)
    }
}

fn is_root(component: &str) -> bool {
    component == "/" || component.ends_with(':')
}

/// Resolve `path` (see the module docs). Pass an absolute path: a relative one has no root and only
/// matches another relative one.
pub fn resolve(facts: &dyn PathFacts, path: &Path) -> ResolvedPath {
    let lexical = lexical(&path.to_string_lossy());
    let canonical = facts.canonicalize_existing_prefix(Path::new(&joined(&lexical)));
    let fold = facts.case_insensitive(path);
    let mut parts = self::lexical(&canonical.to_string_lossy());
    if fold {
        for part in &mut parts {
            *part = part.to_lowercase();
        }
    }
    ResolvedPath(parts)
}

/// Whether `path` names a location from the root, in Unix or Windows spelling.
pub fn has_root(path: &Path) -> bool {
    lexical(&path.to_string_lossy())
        .first()
        .is_some_and(|c| is_root(c))
}

/// Split into `[root?, components…]`, removing `.` and resolving `..` without touching the disk.
fn lexical(raw: &str) -> Vec<String> {
    let text = raw.replace('\\', "/");
    let text = match text.strip_prefix("//?/") {
        Some(rest) if has_drive(rest) => rest.to_owned(),
        _ => text,
    };
    let mut parts = Vec::new();
    let rest = if has_drive(&text) {
        parts.push(text[..2].to_uppercase());
        &text[2..]
    } else if text.starts_with('/') {
        parts.push("/".to_owned());
        &text[..]
    } else {
        &text[..]
    };
    let floor = parts.len();
    for component in rest.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if parts.len() > floor {
                    parts.pop();
                }
            }
            other => parts.push(other.to_owned()),
        }
    }
    parts
}

fn has_drive(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn joined(parts: &[String]) -> String {
    match parts.first() {
        Some(root) if root == "/" => format!("/{}", parts[1..].join("/")),
        Some(root) if root.ends_with(':') => format!("{root}/{}", parts[1..].join("/")),
        _ => parts.join("/"),
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;

    /// A file system that is made of symlink prefixes and one case rule.
    #[derive(Default)]
    pub struct FakeFacts {
        pub links: Vec<(&'static str, &'static str)>,
        pub insensitive: bool,
    }

    impl PathFacts for FakeFacts {
        fn canonicalize_existing_prefix(&self, path: &Path) -> PathBuf {
            let text = path.to_string_lossy().into_owned();
            for (from, to) in &self.links {
                if text == *from || text.starts_with(&format!("{from}/")) {
                    return PathBuf::from(format!("{to}{}", &text[from.len()..]));
                }
            }
            path.to_path_buf()
        }

        fn case_insensitive(&self, _path: &Path) -> bool {
            self.insensitive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeFacts;
    use super::*;

    fn inside(facts: &FakeFacts, base: &str, target: &str) -> bool {
        resolve(facts, Path::new(base)).contains(&resolve(facts, Path::new(target)))
    }

    #[test]
    fn dot_and_dotdot_are_removed_lexically() {
        let f = FakeFacts::default();
        assert!(inside(&f, "/a/wt", "/a/wt/./src/../lib.rs"));
        assert!(!inside(&f, "/a/wt", "/a/wt/../other/x"));
        assert_eq!(
            resolve(&f, Path::new("/a/../../x")),
            resolve(&f, Path::new("/x"))
        );
    }

    #[test]
    fn a_symlinked_ancestor_resolves_to_its_target() {
        let f = FakeFacts {
            links: vec![("/home/me/proj", "/mnt/data/proj")],
            ..Default::default()
        };
        assert!(inside(
            &f,
            "/mnt/data/proj/.claude/worktrees/a",
            "/home/me/proj/.claude/worktrees/a/src/x.rs"
        ));
    }

    #[test]
    fn case_is_folded_only_on_a_case_insensitive_volume() {
        let insensitive = FakeFacts {
            insensitive: true,
            ..Default::default()
        };
        assert!(inside(&insensitive, "/Users/Me/Wt", "/users/me/wt/File.rs"));
        assert!(!inside(
            &FakeFacts::default(),
            "/Users/Me/Wt",
            "/users/me/wt/File.rs"
        ));
    }

    #[test]
    fn drive_letters_and_separators_are_normalised() {
        let f = FakeFacts {
            insensitive: true,
            ..Default::default()
        };
        assert!(inside(&f, "C:\\Repo\\wt", "c:/repo/wt/src\\main.rs"));
        assert!(inside(&f, "C:\\Repo\\wt", "\\\\?\\C:\\Repo\\wt\\a"));
        assert!(!inside(&f, "C:\\Repo\\wt", "D:\\Repo\\wt\\a"));
    }

    #[test]
    fn the_prefix_is_component_wise_not_a_string_prefix() {
        let f = FakeFacts::default();
        assert!(!inside(&f, "/a/wt", "/a/wt2/x"));
        assert!(inside(&f, "/a/wt", "/a/wt"));
    }

    #[test]
    fn a_deeper_match_has_a_larger_depth() {
        let f = FakeFacts::default();
        let root = resolve(&f, Path::new("/a"));
        let wt = resolve(&f, Path::new("/a/.claude/worktrees/x"));
        assert!(root.contains(&wt) && wt.depth() > root.depth());
        assert_eq!(wt.last(), Some("x"));
        assert_eq!(resolve(&f, Path::new("/")).last(), None);
        assert!(has_root(Path::new("C:\\x")) && has_root(Path::new("/x")));
        assert!(!has_root(Path::new("src/x")));
    }

    #[test]
    fn the_real_facts_resolve_a_path_that_does_not_exist_yet() {
        let dir = tempfile::tempdir().unwrap();
        let base = resolve(&RealPathFacts, dir.path());
        let new = resolve(&RealPathFacts, &dir.path().join("not/yet/there.rs"));
        assert!(base.contains(&new));
    }

    #[cfg(unix)]
    #[test]
    fn the_real_facts_follow_a_symlinked_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, dir.path().join("link")).unwrap();
        let base = resolve(&RealPathFacts, &real);
        let via = resolve(&RealPathFacts, &dir.path().join("link/new.rs"));
        assert!(base.contains(&via));
    }
}
