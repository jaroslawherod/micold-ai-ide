//! The Changes view's git reads (research R1): the commands, run through the helpers of
//! [`crate::git`], their output handed to the pure parsers of this module.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::base::{default_branch_from, Base, BaseUnavailable, DiffRange, ReviewScope, Toggles};
use super::changes::{
    assemble, classify, content_of, merge_untracked, parse_name_status, parse_numstat, ChangeKind,
    ChangeList, ChangedFile, Content, NameStatus, Origin, Untracked, VersionSizes,
};
use super::diff::{added_file, parse_unified, FileDiff, LoadedDiff, SideLines, Spans};
use super::marks::WholeChange;
use super::{limits, RelPath};
use crate::git::{git_failed, local_only, run_git, GitCli};
use crate::process::{no_window, run_bounded_with_input, RunOutcome};

/// Options every review read passes before the subcommand (R1).
const GLOBAL: [&str; 4] = [
    "-c",
    "core.quotepath=false",
    "--no-pager",
    // A path is a path, never a pattern: `a[1].rs` must not match `a1.rs` (review A M2 F1).
    "--literal-pathspecs",
];

/// `GLOBAL` without `--literal-pathspecs`: check-ignore takes paths, and refuses pathspec magic.
const CHECK_IGNORE_GLOBAL: [&str; 3] = ["-c", "core.quotepath=false", "--no-pager"];

/// The diff options every review read passes after `diff` (R1).
const DIFF: [&str; 5] = ["--no-ext-diff", "--no-textconv", "--no-color", "-z", "-M"];

/// Run a read-only git command in `dir`, kept to local objects, with `stdin` written to it.
fn read(dir: &Path, args: &[&str], stdin: Option<&str>) -> io::Result<String> {
    read_bytes(dir, args, stdin).map(|out| String::from_utf8_lossy(&out).into_owned())
}

/// [`read`], answering git's bytes as they are: a diff body is checked for UTF-8, not repaired.
pub(super) fn read_bytes(dir: &Path, args: &[&str], stdin: Option<&str>) -> io::Result<Vec<u8>> {
    let ran = run_read(dir, &GLOBAL, args, stdin, READ_TIMEOUT)?;
    if ran.success() {
        Ok(ran.stdout)
    } else {
        Err(git_failed(args, ran.stderr.as_bytes()))
    }
}

/// How long one review git command may run before it is killed and the read fails (#771): a git
/// that never answers (a hung filesystem, a repo under heavy concurrent git work) would otherwise
/// leave the Changes view on "loading" for good.
pub const READ_TIMEOUT: Duration = Duration::from_secs(20);

/// What a finished git run said.
struct Ran {
    /// The exit code, `-1` when a signal ended it.
    code: i32,
    stdout: Vec<u8>,
    stderr: String,
}

impl Ran {
    fn success(&self) -> bool {
        self.code == 0
    }
}

/// Run a read-only git command in `dir` with `global` options before it, answering its output
/// whatever its exit status, and killing it (with its children) after `timeout`.
fn run_read(
    dir: &Path,
    global: &[&str],
    args: &[&str],
    stdin: Option<&str>,
    timeout: Duration,
) -> io::Result<Ran> {
    let mut command = Command::new("git");
    local_only(&mut command);
    // A read never refreshes the index: that write is a change the view's own watch would see,
    // so each refresh would ask for another (review A M7 F3).
    command.env("GIT_OPTIONAL_LOCKS", "0");
    no_window(&mut command)
        .arg("-C")
        .arg(dir)
        .args(global)
        .args(args)
        .stdin(Stdio::null());
    match run_bounded_with_input(command, timeout, stdin.map(|text| text.as_bytes().to_vec())) {
        RunOutcome::Exited {
            code,
            stdout,
            stderr,
        } => Ok(Ran {
            code,
            stdout,
            stderr,
        }),
        RunOutcome::TimedOut { .. } => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "git {} timed out after {}s",
                args.join(" "),
                timeout.as_secs()
            ),
        )),
        RunOutcome::SpawnFailed(why) => Err(io::Error::other(why)),
    }
}

/// `git diff` with the R1 options over `revs` and one more `format` option.
fn diff(dir: &Path, format: &str, revs: &[&str]) -> io::Result<String> {
    let mut args = vec!["diff"];
    args.extend(DIFF);
    args.push(format);
    args.extend(revs);
    read(dir, &args, None)
}

/// The kinds and the counts of one range.
fn range_rows(
    dir: &Path,
    revs: &[&str],
) -> io::Result<(Vec<NameStatus>, Vec<super::changes::NumStat>)> {
    let names = parse_name_status(&diff(dir, "--raw", revs)?);
    let stats = parse_numstat(&diff(dir, "--numstat", revs)?);
    Ok((names, stats))
}

/// The paths one range changes.
fn range_paths(dir: &Path, revs: &[&str]) -> io::Result<BTreeSet<RelPath>> {
    Ok(parse_name_status(&diff(dir, "--raw", revs)?)
        .into_iter()
        .map(|name| name.path)
        .collect())
}

/// Untracked, not-ignored files, read from disk (FR-005). A nested repository or worktree that
/// git lists as `dir/` is not a file and is skipped.
fn untracked(dir: &Path) -> io::Result<Vec<Untracked>> {
    let mut files = Vec::new();
    for path in untracked_paths(dir)? {
        let full = dir.join(path.as_str());
        let Ok(meta) = std::fs::symlink_metadata(&full) else {
            continue;
        };
        let bytes = meta.len();
        let (content, lines) = if meta.is_file() && bytes <= limits::MAX_VERSION_BYTES {
            std::fs::read(&full).map_or((Content::Text, 0), |data| content_of(&data))
        } else {
            (Content::Text, 0)
        };
        files.push(Untracked {
            path,
            lines,
            content,
            bytes,
        });
    }
    Ok(files)
}

/// The paths of the untracked, not-ignored files; a nested repository or worktree (`dir/`) is
/// left out.
fn untracked_paths(dir: &Path) -> io::Result<Vec<RelPath>> {
    let raw = read(
        dir,
        &["ls-files", "-z", "--others", "--exclude-standard"],
        None,
    )?;
    Ok(raw
        .split('\0')
        .filter(|p| !p.is_empty() && !p.ends_with('/'))
        .map(RelPath::from_git)
        .collect())
}

/// The mode git gives a new file at `full` (feature 1271): `120000` for a symlink, `100755` for an
/// executable file, else `100644`; `None` for what is not a file or a symlink.
fn disk_mode(full: &Path) -> Option<&'static str> {
    let meta = std::fs::symlink_metadata(full).ok()?;
    if meta.file_type().is_symlink() {
        return Some("120000");
    }
    if !meta.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o100 != 0 {
            return Some("100755");
        }
    }
    Some("100644")
}

/// The blob ids of the working-tree files `paths` (research R2 step 3): one `git hash-object
/// --stdin-paths` for the files, which applies the clean filters as `git add` does, and the
/// symlinks hashed here over their target path, as git stores them (hash-object would follow the
/// link). A path holding a newline, a path that is no longer a file and every path of a failed
/// run get no id.
fn blob_ids(dir: &Path, paths: &[RelPath]) -> BTreeMap<RelPath, String> {
    let mut ids = BTreeMap::new();
    let mut files = Vec::new();
    for path in paths.iter().filter(|path| !path.as_str().contains('\n')) {
        let full = dir.join(path.as_str());
        match disk_mode(&full) {
            Some("120000") => {
                if let Ok(target) = worktree_bytes(&full) {
                    ids.insert(path.clone(), blob_id(&target));
                }
            }
            Some(_) => files.push(path),
            None => {}
        }
    }
    if files.is_empty() {
        return ids;
    }
    let input: String = files.iter().map(|path| format!("{path}\n")).collect();
    let Ok(out) = read(dir, &["hash-object", "--stdin-paths"], Some(&input)) else {
        return ids;
    };
    let hashed: Vec<&str> = out.lines().map(str::trim).collect();
    if hashed.len() == files.len() {
        for (path, id) in files.into_iter().zip(hashed) {
            ids.insert(path.clone(), id.to_owned());
        }
    }
    ids
}

/// Git's SHA-1 blob id of `bytes`.
fn blob_id(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()));
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The sizes of `specs` (`<rev>:<path>`) in one `git cat-file --batch-check`; `None` for one git
/// does not have.
pub(super) fn object_sizes(dir: &Path, specs: &[String]) -> Vec<Option<u64>> {
    if specs.is_empty() {
        return Vec::new();
    }
    let mut input = String::new();
    for spec in specs {
        // A newline in a path would split the request; ask for something that never exists.
        input.push_str(if spec.contains('\n') { ":" } else { spec });
        input.push('\n');
    }
    let Ok(out) = read(
        dir,
        &["cat-file", "--batch-check=%(objectsize)"],
        Some(&input),
    ) else {
        return vec![None; specs.len()];
    };
    let mut sizes: Vec<Option<u64>> = out.lines().map(|line| line.trim().parse().ok()).collect();
    sizes.resize(specs.len(), None);
    sizes
}

/// Mark the large rows (R8): the old version's size from git, the new one's from `HEAD` or the
/// working tree.
fn classify_all(dir: &Path, files: &mut [ChangedFile], old_rev: &str, new_rev: Option<&str>) {
    let old_specs: Vec<String> = files
        .iter()
        .map(|file| {
            let path = match &file.kind {
                ChangeKind::Renamed { from } => from,
                _ => &file.path,
            };
            format!("{old_rev}:{path}")
        })
        .collect();
    let old = object_sizes(dir, &old_specs);
    let new: Vec<Option<u64>> = match new_rev {
        Some(rev) => {
            let specs: Vec<String> = files
                .iter()
                .map(|file| format!("{rev}:{}", file.path))
                .collect();
            object_sizes(dir, &specs)
        }
        None => files
            .iter()
            .map(|file| {
                std::fs::metadata(dir.join(file.path.as_str()))
                    .ok()
                    .map(|m| m.len())
            })
            .collect(),
    };
    for ((file, old), new) in files.iter_mut().zip(old).zip(new) {
        if file.kind != ChangeKind::Untracked {
            classify(file, VersionSizes { old, new });
        }
    }
}

impl GitCli {
    /// Which of `paths` (files of the worktree at `dir`) git ignores, by one
    /// `git check-ignore -z --stdin` (R9). A tracked file is never ignored.
    pub fn ignored(&self, dir: &Path, paths: &[PathBuf]) -> io::Result<BTreeSet<PathBuf>> {
        // A path that is not UTF-8 cannot go through the text pipe: it counts as not ignored.
        let asked: Vec<&str> = paths.iter().filter_map(|path| path.to_str()).collect();
        if asked.is_empty() {
            return Ok(BTreeSet::new());
        }
        let mut input = asked.join("\0");
        input.push('\0');
        let args = ["check-ignore", "-z", "--stdin"];
        let ran = run_read(dir, &CHECK_IGNORE_GLOBAL, &args, Some(&input), READ_TIMEOUT)?;
        // Exit 1: none of them is ignored.
        match ran.code {
            0 => Ok(String::from_utf8_lossy(&ran.stdout)
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
                .collect()),
            1 => Ok(BTreeSet::new()),
            _ => Err(git_failed(&args, ran.stderr.as_bytes())),
        }
    }

    /// The branch checked out in the worktree at `dir` (feature 1149, G1): `None` when `HEAD` is
    /// detached or git cannot say. Read-only, bounded like every review read.
    pub fn checked_out_branch(&self, dir: &Path) -> Option<String> {
        let name = read(dir, &["symbolic-ref", "--short", "-q", "HEAD"], None).ok()?;
        let name = name.trim_end_matches('\n');
        (!name.is_empty()).then(|| name.to_owned())
    }

    /// Whether commit `sha` exists in the repository of `dir` (feature 1149, G1). A `sha` that is
    /// not 40 or 64 hex digits is false without running git; a git failure counts as absent.
    pub fn has_commit(&self, dir: &Path, sha: &str) -> bool {
        if !super::comment::is_commit_id(sha) {
            return false;
        }
        let object = format!("{sha}^{{commit}}");
        read(dir, &["cat-file", "-e", &object], None).is_ok()
    }

    /// The git metadata directories of the worktree at `dir`: its own git dir and the common dir
    /// (the same for the main worktree), absolute (R9).
    pub fn git_dirs(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        let out = read(
            dir,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-dir",
                "--git-common-dir",
            ],
            None,
        )?;
        let mut dirs: Vec<PathBuf> = out.lines().map(PathBuf::from).collect();
        dirs.dedup();
        Ok(dirs)
    }

    /// The reflog entries of `refs` in the worktree at `dir` (feature 1003, contracts/
    /// pull-request-list.md §1), one `%gD\t%H\t%gs` line each with the time as Unix seconds. A ref
    /// with no reflog is skipped, not an error. Read-only, bounded like every review read.
    pub fn reflogs(&self, dir: &Path, refs: &[String]) -> io::Result<String> {
        let mut args = vec![
            "log",
            "-g",
            "--ignore-missing",
            "--date=unix",
            "--format=%gD%x09%H%x09%gs",
        ];
        args.extend(refs.iter().map(String::as_str));
        args.push("--");
        read(dir, &args, None)
    }

    /// The base of the worktree at `dir` (R7): the merge-base of `HEAD` and the default branch.
    pub fn review_base(&self, dir: &Path) -> Base {
        let origin_head = read(
            dir,
            &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
            None,
        )
        .ok();
        let has = |branch: &str| {
            read(
                dir,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/{branch}"),
                ],
                None,
            )
            .is_ok()
        };
        let Some(branch) = default_branch_from(origin_head.as_deref(), has("main"), has("master"))
        else {
            return Base::Unavailable(BaseUnavailable::NoDefaultBranch);
        };
        match run_git(dir, &["merge-base", "HEAD", &branch]) {
            Ok(commit) if !commit.trim().is_empty() => Base::MergeBase {
                branch,
                commit: commit.trim().to_owned(),
            },
            _ => Base::Unavailable(BaseUnavailable::NoCommonHistory),
        }
    }

    /// The merge-base of `HEAD` and local branch `branch` (feature 483, R7): the base a run is
    /// counted against when its group started from `branch`.
    pub fn review_base_against(&self, dir: &Path, branch: &str) -> Base {
        let reference = format!("refs/heads/{branch}");
        if run_git(dir, &["rev-parse", "--verify", "--quiet", &reference]).is_err() {
            return Base::Unavailable(BaseUnavailable::NoDefaultBranch);
        }
        match run_git(dir, &["merge-base", "HEAD", &reference]) {
            Ok(commit) if !commit.trim().is_empty() => Base::MergeBase {
                branch: branch.to_owned(),
                commit: commit.trim().to_owned(),
            },
            _ => Base::Unavailable(BaseUnavailable::NoCommonHistory),
        }
    }

    /// The files changed in `dir` under `toggles` (R1, FR-002, FR-004, FR-005): one row per path,
    /// sorted, each classified against the R8 limits.
    pub fn change_list(
        &self,
        dir: &Path,
        scope: ReviewScope,
        toggles: Toggles,
    ) -> io::Result<ChangeList> {
        let base = match &scope {
            ReviewScope::Worktree {
                base: Base::MergeBase { commit, .. },
            } => Some(commit.clone()),
            _ => None,
        };
        let files = match (DiffRange::for_view(&scope, toggles), base.as_deref()) {
            (None, _) => Vec::new(),
            (Some(DiffRange::BaseToHead), Some(base)) => {
                let (names, stats) = range_rows(dir, &[base, "HEAD"])?;
                let mut files = assemble(
                    names,
                    stats,
                    &BTreeSet::new(),
                    &BTreeSet::new(),
                    Origin::Committed,
                );
                classify_all(dir, &mut files, base, Some("HEAD"));
                files
            }
            (Some(DiffRange::BaseToWorktree), Some(base)) => {
                let (names, stats) = range_rows(dir, &[base])?;
                let committed = range_paths(dir, &[base, "HEAD"])?;
                let uncommitted = range_paths(dir, &["HEAD"])?;
                let mut files = assemble(names, stats, &committed, &uncommitted, Origin::Both);
                classify_all(dir, &mut files, base, None);
                merge_untracked(files, untracked(dir)?)
            }
            _ => {
                let (names, stats) = range_rows(dir, &["HEAD"])?;
                let mut files = assemble(
                    names,
                    stats,
                    &BTreeSet::new(),
                    &BTreeSet::new(),
                    Origin::Uncommitted,
                );
                classify_all(dir, &mut files, "HEAD", None);
                merge_untracked(files, untracked(dir)?)
            }
        };
        Ok(ChangeList { files, scope })
    }

    /// The entry's whole working-tree change, whatever the toggles (feature 1271, research R2,
    /// contracts/marks-wire.md § G2): `git diff --raw --no-abbrev` over the merge-base (or `HEAD`
    /// for the Default entry and a worktree without a base), the origins from the committed and
    /// uncommitted path sets, and the blob ids git did not print from one `hash-object`. A failure
    /// of the diff or of a path set is the read's error; a file whose blob id could not be read
    /// has no id.
    pub fn whole_change(&self, dir: &Path, scope: &ReviewScope) -> io::Result<WholeChange> {
        let base = match scope {
            ReviewScope::Worktree {
                base: Base::MergeBase { commit, .. },
            } => Some(commit.as_str()),
            _ => None,
        };
        let names = parse_name_status(&diff(
            dir,
            "--raw",
            &["--no-abbrev", base.unwrap_or("HEAD")],
        )?);
        let (committed, uncommitted) = match base {
            Some(base) => (
                Some(range_paths(dir, &[base, "HEAD"])?),
                range_paths(dir, &["HEAD"])?,
            ),
            None => (None, BTreeSet::new()),
        };
        let untracked: Vec<(RelPath, String)> = untracked_paths(dir)?
            .into_iter()
            .filter_map(|path| {
                let mode = disk_mode(&dir.join(path.as_str()))?;
                Some((path, mode.to_owned()))
            })
            .collect();
        let unhashed: Vec<RelPath> = names
            .iter()
            .filter(|name| {
                let zeros = |text: &str| text.bytes().all(|b| b == b'0');
                // A submodule's new side (`160000`) is a commit, not a file to hash.
                zeros(&name.new_id) && !zeros(&name.new_mode) && name.new_mode != "160000"
            })
            .map(|name| name.path.clone())
            .chain(untracked.iter().map(|(path, _)| path.clone()))
            .collect();
        let hashed = blob_ids(dir, &unhashed);
        Ok(WholeChange::assemble(
            names,
            committed.as_ref(),
            &uncommitted,
            &hashed,
            untracked,
        ))
    }

    /// The diff of `path` (R1, R8): `git diff -M -U3` over the view's range, the old path too for a
    /// rename; an untracked file read from disk. `TooLarge` over the limits unless `force_large`.
    /// A text diff comes with both versions' lines; a side that does not exist is `None`.
    pub fn file_diff(
        &self,
        dir: &Path,
        scope: &ReviewScope,
        toggles: Toggles,
        path: &RelPath,
        from: Option<&RelPath>,
        force_large: bool,
    ) -> io::Result<LoadedDiff> {
        let base = match scope {
            ReviewScope::Worktree {
                base: Base::MergeBase { commit, .. },
            } => Some(commit.as_str()),
            _ => None,
        };
        // (old revision, new revision — `None` for the working tree)
        let (old_rev, new_rev) = match (DiffRange::for_view(scope, toggles), base) {
            (None, _) => {
                return Ok(LoadedDiff {
                    diff: FileDiff::Text(Vec::new()),
                    old: None,
                    new: None,
                    spans: Spans::default(),
                })
            }
            (Some(DiffRange::BaseToHead), Some(base)) => (base, Some("HEAD")),
            (Some(DiffRange::BaseToWorktree), Some(base)) => (base, None),
            _ => ("HEAD", None),
        };
        if new_rev.is_none() && is_untracked(dir, path)? {
            return untracked_diff(dir, path, force_large);
        }
        let old_path = from.unwrap_or(path);
        let mut revs = vec![old_rev];
        revs.extend(new_rev);
        let mut pathspec = vec!["--"];
        if from.is_some() {
            pathspec.push(old_path.as_str());
        }
        pathspec.push(path.as_str());

        let mut stat_args = revs.clone();
        stat_args.extend(&pathspec);
        let stats = parse_numstat(&diff(dir, "--numstat", &stat_args)?);
        let lines = stats.iter().find(|s| &s.path == path).map(|s| s.lines);
        if lines == Some(None) {
            return Ok(LoadedDiff {
                diff: FileDiff::Binary,
                old: None,
                new: None,
                spans: Spans::default(),
            });
        }
        if !force_large {
            let (added, removed) = lines.flatten().unwrap_or((0, 0));
            let old_size = object_sizes(dir, &[format!("{old_rev}:{old_path}")])[0];
            let new_size = match new_rev {
                Some(rev) => object_sizes(dir, &[format!("{rev}:{path}")])[0],
                None => std::fs::symlink_metadata(dir.join(path.as_str()))
                    .ok()
                    .map(|m| m.len()),
            };
            let too_big = |size: Option<u64>| size.is_some_and(|s| s > limits::MAX_VERSION_BYTES);
            if u64::from(added) + u64::from(removed) > u64::from(limits::MAX_CHANGED_LINES)
                || too_big(old_size)
                || too_big(new_size)
            {
                return Ok(LoadedDiff {
                    diff: FileDiff::TooLarge { added, removed },
                    old: None,
                    new: None,
                    spans: Spans::default(),
                });
            }
        }

        let mut args = vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "-M",
            "-U3",
        ];
        args.extend(&revs);
        args.extend(&pathspec);
        let diff = parse_unified(&read_bytes(dir, &args, None)?);
        if !matches!(diff, FileDiff::Text(_)) {
            return Ok(LoadedDiff {
                diff,
                old: None,
                new: None,
                spans: Spans::default(),
            });
        }
        let old = blob(dir, old_rev, old_path);
        let new = match new_rev {
            Some(rev) => blob(dir, rev, path),
            None => worktree_bytes(&dir.join(path.as_str())).ok(),
        };
        Ok(LoadedDiff {
            diff,
            old: old.as_deref().and_then(SideLines::from_bytes),
            new: new.as_deref().and_then(SideLines::from_bytes),
            spans: Spans::default(),
        })
    }
}

/// Whether `path` is untracked and not ignored in `dir` (FR-005).
fn is_untracked(dir: &Path, path: &RelPath) -> io::Result<bool> {
    let out = read(
        dir,
        &[
            "ls-files",
            "-z",
            "--others",
            "--exclude-standard",
            "--",
            path.as_str(),
        ],
        None,
    )?;
    Ok(out.split('\0').any(|p| p == path.as_str()))
}

/// An untracked file's diff, read from disk: every line added (R8: a NUL byte is binary).
fn untracked_diff(dir: &Path, path: &RelPath, force_large: bool) -> io::Result<LoadedDiff> {
    let full = dir.join(path.as_str());
    let size = std::fs::symlink_metadata(&full)?.len();
    if !force_large && size > limits::MAX_VERSION_BYTES {
        return Ok(LoadedDiff {
            diff: FileDiff::TooLarge {
                added: count_lines(&full)?,
                removed: 0,
            },
            old: None,
            new: None,
            spans: Spans::default(),
        });
    }
    let bytes = worktree_bytes(&full)?;
    let (_, lines) = content_of(&bytes);
    if !force_large && lines > limits::MAX_CHANGED_LINES {
        return Ok(LoadedDiff {
            diff: FileDiff::TooLarge {
                added: lines,
                removed: 0,
            },
            old: None,
            new: None,
            spans: Spans::default(),
        });
    }
    let diff = added_file(&bytes);
    let new = matches!(diff, FileDiff::Text(_))
        .then(|| SideLines::from_bytes(&bytes))
        .flatten();
    Ok(LoadedDiff {
        diff,
        old: None,
        new,
        spans: Spans::default(),
    })
}

/// The bytes of `<rev>:<path>`, or `None` when that revision has no such file.
fn blob(dir: &Path, rev: &str, path: &RelPath) -> Option<Vec<u8>> {
    if path.as_str().contains('\n') {
        return None;
    }
    read_bytes(
        dir,
        &["cat-file", "blob", &format!("{rev}:{}", path.as_str())],
        None,
    )
    .ok()
}

/// The bytes git diffs for a working-tree file: a symlink's target path, never what it points to
/// (review A M2 F3: the target may be outside the worktree).
pub(super) fn worktree_bytes(full: &Path) -> io::Result<Vec<u8>> {
    if std::fs::symlink_metadata(full)?.file_type().is_symlink() {
        return Ok(std::fs::read_link(full)?
            .into_os_string()
            .into_encoded_bytes());
    }
    std::fs::read(full)
}

/// The lines of a file too large to read whole, counted in chunks: newlines, plus a last line
/// without one (review A M2 F2).
fn count_lines(full: &Path) -> io::Result<u32> {
    use std::io::Read as _;
    let mut file = std::fs::File::open(full)?;
    let mut buf = vec![0u8; 64 * 1024];
    let (mut lines, mut last) = (0u32, b'\n');
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let newlines = buf[..n].iter().filter(|&&b| b == b'\n').count();
        lines = lines.saturating_add(u32::try_from(newlines).unwrap_or(u32::MAX));
        last = buf[n - 1];
    }
    Ok(if last == b'\n' {
        lines
    } else {
        lines.saturating_add(1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// #771: a git that never answers is killed at the deadline and the read fails with a timeout,
    /// instead of leaving the Changes view on "loading" for good.
    #[cfg(unix)]
    #[test]
    fn a_stalled_git_read_fails_at_the_deadline() {
        let dir = std::env::temp_dir();
        let started = Instant::now();
        let error = run_read(
            &dir,
            &["-c", "alias.stall=!sleep 30"],
            &["stall"],
            None,
            Duration::from_millis(300),
        )
        .err()
        .expect("a stalled git must not answer");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut, "{error}");
        assert!(error.to_string().contains("timed out"), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the child is killed at the deadline, not waited for"
        );
    }

    #[test]
    fn a_quick_git_read_answers_with_its_output_and_code() {
        let dir = std::env::temp_dir();
        let ran = run_read(&dir, &[], &["--version"], None, READ_TIMEOUT).expect("git runs");
        assert!(ran.success());
        assert!(String::from_utf8_lossy(&ran.stdout).starts_with("git version"));
    }
}
