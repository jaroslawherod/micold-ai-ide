//! Turning dropped files into text for a terminal's input line (feature 487).
//!
//! Pure: a list of host paths and the terminal's shell go in, and the quoted text to type comes
//! out, with the paths that could not be written for that shell set aside and named. Nothing here
//! sends anything; the client types the text without a newline, so nothing runs until Enter
//! (FR-003).

pub mod pasted;
pub mod quote;

use std::path::{Path, PathBuf};

use crate::sandbox::MountSet;

pub use pasted::{PastedLayout, pasted_roots};
pub use quote::{quote, Unrepresentable};

/// The shell that will read the inserted text. Each quotes differently (research R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellKind {
    /// `sh`, `dash`, `ksh` and anything unknown on Unix: single quotes only.
    Posix,
    /// bash and zsh: as [`ShellKind::Posix`], plus `$'…'` for names that are not UTF-8.
    Bash,
    /// fish: single quotes with `\\` and `\'` escaped.
    Fish,
    /// PowerShell: single quotes, doubled inside (curly quotes too).
    PowerShell,
    /// `cmd.exe`: double quotes, with no way to write `"` or `%` inside.
    Cmd,
}

/// Where the text will be typed (data-model: `InsertTarget`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertTarget<'a> {
    /// A terminal running on this machine: the paths are used as they are.
    Host,
    /// A terminal inside the sandbox container (FR-010, FR-011): each path becomes the path the
    /// container sees, and a path the container cannot see is refused. The shell is the
    /// container's, so [`ShellKind::Bash`] whatever the host runs.
    Sandbox(
        /// What the sandbox shares.
        &'a MountSet,
        /// The container paths of the projects the running container really mounts. A project in
        /// the set but not here was registered after the container started.
        &'a [String],
    ),
}

/// What a sandboxed session shares, gathered by the shell for the reducer (feature 487, FR-010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxShare {
    /// The mounts the sandbox was planned with.
    pub mounts: MountSet,
    /// The container paths of the projects the running container mounts.
    pub mounted: Vec<String>,
}

impl SandboxShare {
    /// The target for [`plan_insertion`].
    pub fn target(&self) -> InsertTarget<'_> {
        InsertTarget::Sandbox(&self.mounts, &self.mounted)
    }
}

/// One path that will be inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertedPath {
    /// The path as it was dropped.
    pub original: PathBuf,
    /// The path the terminal will see.
    pub shown: PathBuf,
    /// `shown`, quoted for the shell.
    pub text: String,
}

/// Why a path was left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The shell has no way to write this name as one literal argument.
    Unrepresentable {
        /// The file.
        path: PathBuf,
        /// The shell that cannot write it.
        shell: ShellKind,
    },
    /// The sandbox shares no registered project that holds this file (FR-011).
    OutsideProjects {
        /// The file.
        path: PathBuf,
    },
    /// The file is in a project the running sandbox was started without.
    NotMounted {
        /// The file.
        path: PathBuf,
    },
}

impl Refusal {
    /// A sentence for the user that names the file and the reason (FR-011).
    pub fn message(&self) -> String {
        match self {
            Refusal::Unrepresentable { path, shell } => format!(
                "Could not insert {}: {} cannot take a name with these characters.",
                file_name(path),
                shell_name(*shell)
            ),
            Refusal::OutsideProjects { path } => format!(
                "Could not insert {}: it is outside the registered projects, so the sandbox \
                 cannot see it.",
                file_name(path)
            ),
            Refusal::NotMounted { path } => format!(
                "Could not insert {}: its project was added after the sandbox started, so the \
                 sandbox cannot see it yet. Restart the sandbox to share it.",
                file_name(path)
            ),
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn shell_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Posix => "this shell",
        ShellKind::Bash => "bash",
        ShellKind::Fish => "fish",
        ShellKind::PowerShell => "PowerShell",
        ShellKind::Cmd => "cmd",
    }
}

/// What a drop will type, and what it leaves out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InsertionPlan {
    /// The paths to type, in the order they were given.
    pub accepted: Vec<InsertedPath>,
    /// The paths left out, each with its reason.
    pub refused: Vec<Refusal>,
}

impl InsertionPlan {
    /// The accepted paths joined by single spaces, or `None` when nothing is accepted. No leading
    /// or trailing space and no line break: what the user already typed is untouched (FR-004).
    pub fn text(&self) -> Option<String> {
        if self.accepted.is_empty() {
            return None;
        }
        Some(
            self.accepted
                .iter()
                .map(|p| p.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

/// Plan the insertion of `paths` for a terminal whose shell is `shell`.
///
/// A directory is inserted like a file and a missing file is still inserted: the terminal's
/// program decides what to make of either.
pub fn plan_insertion(paths: &[PathBuf], shell: ShellKind, target: InsertTarget) -> InsertionPlan {
    let shell = match target {
        InsertTarget::Host => shell,
        InsertTarget::Sandbox(..) => ShellKind::Bash,
    };
    let mut plan = InsertionPlan::default();
    for path in paths {
        let shown = match target {
            InsertTarget::Host => path.clone(),
            InsertTarget::Sandbox(mounts, mounted) => match container_path(mounts, mounted, path) {
                Ok(shown) => shown,
                Err(refusal) => {
                    plan.refused.push(refusal);
                    continue;
                }
            },
        };
        match quote(shell, shown.as_os_str()) {
            Ok(text) => plan.accepted.push(InsertedPath {
                original: path.clone(),
                shown,
                text,
            }),
            Err(Unrepresentable) => plan.refused.push(Refusal::Unrepresentable {
                path: path.clone(),
                shell,
            }),
        }
    }
    plan
}

/// The path the container sees for `path`, or why it sees none.
///
/// The path is judged after symlinks resolve, so a link inside a project that points out of it is
/// refused. A file that does not exist yet is judged by its parent directory. The sandbox state
/// directory is shared too, but only its `pasted` folder is offered: the rest holds the token.
fn container_path(mounts: &MountSet, mounted: &[String], path: &Path) -> Result<PathBuf, Refusal> {
    let outside = || Refusal::OutsideProjects {
        path: path.to_path_buf(),
    };
    let real = resolve(path).ok_or_else(outside)?;

    let pasted = mounts.state.host.join(pasted::TEMP_DIR);
    let pasted = resolve(&pasted).unwrap_or(pasted);
    if let Ok(rest) = real.strip_prefix(&pasted) {
        // Strings all the way: `Path::join` would write `\` on a Windows host.
        let base = join_container(&mounts.state.container, Path::new(pasted::TEMP_DIR));
        return Ok(join_container(&base, rest));
    }

    // The most specific project wins when one sits inside another.
    let (project, rest) = mounts
        .projects
        .iter()
        .filter_map(|p| {
            let root = resolve(&p.host).unwrap_or_else(|| p.host.clone());
            let rest = real.strip_prefix(&root).ok()?.to_path_buf();
            Some((p, rest, root.components().count()))
        })
        .max_by_key(|(_, _, depth)| *depth)
        .map(|(p, rest, _)| (p, rest))
        .ok_or_else(outside)?;
    let container = project.container.to_string_lossy();
    if !mounted.iter().any(|m| *m == container) {
        return Err(Refusal::NotMounted {
            path: path.to_path_buf(),
        });
    }
    Ok(join_container(&project.container, &rest))
}

/// `path` with symlinks resolved; a missing file is its resolved parent plus its name.
fn resolve(path: &Path) -> Option<PathBuf> {
    if let Ok(real) = path.canonicalize() {
        return Some(real);
    }
    let name = path.file_name()?;
    Some(path.parent()?.canonicalize().ok()?.join(name))
}

/// `base` followed by `rest`, written with `/` as the container expects whatever the host is.
fn join_container(base: &Path, rest: &Path) -> PathBuf {
    let mut out = base.to_string_lossy().into_owned();
    for part in rest.components() {
        if let std::path::Component::Normal(name) = part {
            if !out.ends_with('/') {
                out.push('/');
            }
            out.push_str(&name.to_string_lossy());
        }
    }
    PathBuf::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(p: &[&str]) -> Vec<PathBuf> {
        p.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn accepted_paths_keep_input_order_and_join_with_single_spaces() {
        let plan = plan_insertion(
            &paths(&["/b/two.png", "/a/one file.png", "/c"]),
            ShellKind::Posix,
            InsertTarget::Host,
        );
        assert_eq!(
            plan.text().as_deref(),
            Some("'/b/two.png' '/a/one file.png' '/c'")
        );
        assert!(plan.refused.is_empty());
    }

    #[test]
    fn nothing_accepted_means_no_text() {
        let plan = plan_insertion(&[], ShellKind::Posix, InsertTarget::Host);
        assert_eq!(plan.text(), None);
        let plan = plan_insertion(&paths(&["/a\"b"]), ShellKind::Cmd, InsertTarget::Host);
        assert_eq!(plan.text(), None);
        assert_eq!(plan.refused.len(), 1);
    }

    #[test]
    fn a_directory_and_a_missing_file_are_inserted_like_any_file() {
        let dir = std::env::temp_dir();
        let missing = dir.join("487-no-such-file.png");
        let plan = plan_insertion(
            &[dir.clone(), missing.clone()],
            ShellKind::Posix,
            InsertTarget::Host,
        );
        assert_eq!(plan.accepted.len(), 2);
        assert_eq!(plan.accepted[1].original, missing);
    }

    #[test]
    fn an_unrepresentable_name_is_refused_with_the_file_named() {
        let plan = plan_insertion(
            &paths(&["/ok/a.png", "/x/50%.png"]),
            ShellKind::Cmd,
            InsertTarget::Host,
        );
        assert_eq!(plan.accepted.len(), 1);
        let msg = plan.refused[0].message();
        assert!(msg.contains("50%.png"), "{msg}");
        assert!(msg.contains("cmd"), "{msg}");
    }

    mod sandbox {
        use super::*;
        use crate::sandbox::{
            HomeMount, MountSet, ProjectMount, SecretMount, StateMount, STATE_CONTAINER_DIR,
        };

        struct Fixture {
            _tmp: tempfile::TempDir,
            root: PathBuf,
            mounts: MountSet,
        }

        fn fixture() -> Fixture {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().canonicalize().unwrap();
            for dir in ["proj", "late", "state/pasted", "home", "elsewhere"] {
                std::fs::create_dir_all(root.join(dir)).unwrap();
            }
            let mounts = MountSet {
                projects: vec![
                    ProjectMount::project_for(root.join("proj"), false),
                    ProjectMount::project_for(root.join("late"), false),
                ],
                state: StateMount {
                    host: root.join("state"),
                    container: PathBuf::from(STATE_CONTAINER_DIR),
                },
                home: HomeMount {
                    host: root.join("state/home"),
                    container: root.join("home"),
                },
                secret: SecretMount {
                    host: root.join("state/token"),
                    container: PathBuf::from("/run/token"),
                },
                credentials: Vec::new(),
            };
            Fixture {
                _tmp: tmp,
                root,
                mounts,
            }
        }

        fn mounted(f: &Fixture) -> Vec<String> {
            vec![f.root.join("proj").to_string_lossy().into_owned()]
        }

        // Unix only: a Windows host's own paths are not container paths (the next test).
        #[cfg(unix)]
        #[test]
        fn a_project_file_is_inserted_as_its_container_path() {
            let f = fixture();
            let file = f.root.join("proj/a b.png");
            std::fs::write(&file, b"x").unwrap();
            let m = mounted(&f);
            let plan = plan_insertion(
                std::slice::from_ref(&file),
                ShellKind::Fish,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(plan.accepted[0].shown, file);
            // The container's shell is bash whatever the host runs: single quotes, not fish's.
            assert_eq!(
                plan.text().unwrap(),
                format!("'{}'", file.to_string_lossy())
            );
        }

        #[test]
        fn a_windows_mapping_writes_container_separators() {
            let mut f = fixture();
            f.mounts.projects[0].container = PathBuf::from("/mnt/host/c/p");
            let file = f.root.join("proj/sub/x.png");
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, b"x").unwrap();
            let m = vec!["/mnt/host/c/p".to_string()];
            let plan = plan_insertion(
                &[file],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(
                plan.accepted[0].shown,
                PathBuf::from("/mnt/host/c/p/sub/x.png")
            );
        }

        #[test]
        fn a_file_outside_every_project_is_refused_and_named() {
            let f = fixture();
            let file = f.root.join("elsewhere/secret.txt");
            std::fs::write(&file, b"x").unwrap();
            let m = mounted(&f);
            let plan = plan_insertion(
                &[file],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(plan.text(), None);
            assert!(matches!(plan.refused[0], Refusal::OutsideProjects { .. }));
            let msg = plan.refused[0].message();
            assert!(
                msg.contains("secret.txt") && msg.contains("outside"),
                "{msg}"
            );
        }

        #[test]
        fn a_project_the_container_started_without_is_not_mounted() {
            let f = fixture();
            let file = f.root.join("late/a.png");
            std::fs::write(&file, b"x").unwrap();
            let m = mounted(&f);
            let plan = plan_insertion(
                &[file],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert!(matches!(plan.refused[0], Refusal::NotMounted { .. }));
            assert!(plan.refused[0].message().contains("a.png"));
        }

        #[cfg(unix)]
        #[test]
        fn a_symlink_that_leaves_the_project_is_refused() {
            let f = fixture();
            let target = f.root.join("elsewhere/real.txt");
            std::fs::write(&target, b"x").unwrap();
            let link = f.root.join("proj/link.txt");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            let m = mounted(&f);
            let plan = plan_insertion(
                &[link],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert!(matches!(plan.refused[0], Refusal::OutsideProjects { .. }));
        }

        // Unix only, like the first test: it expects the host path back as the container path.
        #[cfg(unix)]
        #[test]
        fn a_missing_file_is_judged_by_its_parent() {
            let f = fixture();
            let m = mounted(&f);
            let inside = f.root.join("proj").join("not-yet.png");
            let outside = f.root.join("elsewhere/not-yet.png");
            let plan = plan_insertion(
                &[inside.clone(), outside, f.root.join("nodir/x.png")],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(plan.accepted.len(), 1);
            assert_eq!(plan.accepted[0].shown, inside);
            assert_eq!(plan.refused.len(), 2);
        }

        #[test]
        fn a_missing_file_under_a_mapped_project_gets_its_container_path() {
            let mut f = fixture();
            f.mounts.projects[0].container = PathBuf::from("/mnt/host/c/p");
            let m = vec!["/mnt/host/c/p".to_string()];
            let plan = plan_insertion(
                &[
                    f.root.join("proj").join("not-yet.png"),
                    f.root.join("nodir").join("x.png"),
                ],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(plan.accepted.len(), 1);
            assert_eq!(
                plan.accepted[0].shown,
                PathBuf::from("/mnt/host/c/p/not-yet.png")
            );
            assert_eq!(plan.refused.len(), 1);
        }

        #[test]
        fn only_the_pasted_folder_of_the_state_directory_is_offered() {
            let f = fixture();
            let m = mounted(&f);
            let image = f.root.join("state/pasted/abc/1-0.png");
            let token = f.root.join("state/token");
            std::fs::create_dir_all(image.parent().unwrap()).unwrap();
            std::fs::write(&image, b"x").unwrap();
            std::fs::write(&token, b"x").unwrap();
            let plan = plan_insertion(
                &[image, token],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(
                plan.accepted[0].shown,
                PathBuf::from(format!("{STATE_CONTAINER_DIR}/pasted/abc/1-0.png"))
            );
            assert_eq!(plan.refused.len(), 1);
        }

        #[test]
        fn a_mixed_drop_inserts_the_visible_ones_and_names_the_rest() {
            let f = fixture();
            let m = mounted(&f);
            let a = f.root.join("proj/a.png");
            let b = f.root.join("proj/b.png");
            let c = f.root.join("elsewhere/c.png");
            for p in [&a, &b, &c] {
                std::fs::write(p, b"x").unwrap();
            }
            let plan = plan_insertion(
                &[a, c, b],
                ShellKind::Posix,
                InsertTarget::Sandbox(&f.mounts, &m),
            );
            assert_eq!(plan.accepted.len(), 2);
            assert_eq!(plan.refused.len(), 1);
            assert!(plan.refused[0].message().contains("c.png"));
        }
    }
}
