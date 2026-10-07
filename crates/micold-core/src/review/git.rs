//! The Changes view's git reads (research R1): the commands, run through the helpers of
//! [`crate::git`], their output handed to the pure parsers of this module.

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use super::base::{default_branch_from, Base, BaseUnavailable, DiffRange, ReviewScope, Toggles};
use super::changes::{
    assemble, classify, content_of, merge_untracked, parse_name_status, parse_numstat, ChangeKind,
    ChangeList, ChangedFile, Content, NameStatus, Origin, Untracked, VersionSizes,
};
use super::diff::{added_file, parse_unified, FileDiff, LoadedDiff, SideLines, Spans};
use super::{limits, RelPath};
use crate::git::{local_only, run_git, GitCli};
use crate::process::no_window;

/// Options every review read passes before the subcommand (R1).
const GLOBAL: [&str; 4] = [
    "-c",
    "core.quotepath=false",
    "--no-pager",
    // A path is a path, never a pattern: `a[1].rs` must not match `a1.rs` (review A M2 F1).
    "--literal-pathspecs",
];

/// The diff options every review read passes after `diff` (R1).
const DIFF: [&str; 5] = ["--no-ext-diff", "--no-textconv", "--no-color", "-z", "-M"];

/// Run a read-only git command in `dir`, kept to local objects, with `stdin` written to it.
fn read(dir: &Path, args: &[&str], stdin: Option<&str>) -> io::Result<String> {
    read_bytes(dir, args, stdin).map(|out| String::from_utf8_lossy(&out).into_owned())
}

/// [`read`], answering git's bytes as they are: a diff body is checked for UTF-8, not repaired.
fn read_bytes(dir: &Path, args: &[&str], stdin: Option<&str>) -> io::Result<Vec<u8>> {
    let output = run_read(dir, &GLOBAL, args, stdin)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(failed(args, &output))
    }
}

/// The error of a git run that did not succeed, with what it said.
fn failed(args: &[&str], output: &Output) -> io::Error {
    io::Error::other(format!(
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// Run a read-only git command in `dir` with `global` options before it, answering its output
/// whatever its exit status.
fn run_read(dir: &Path, global: &[&str], args: &[&str], stdin: Option<&str>) -> io::Result<Output> {
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
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    // Write the input on its own thread: git answers as it reads, and a reply that filled its
    // pipe while we were still writing would stop both sides.
    let writer = match (stdin, child.stdin.take()) {
        (Some(text), Some(mut pipe)) => {
            let text = text.to_owned();
            Some(std::thread::spawn(move || pipe.write_all(text.as_bytes())))
        }
        _ => None,
    };
    let output = child.wait_with_output()?;
    if let Some(writer) = writer {
        writer
            .join()
            .map_err(|_| io::Error::other("the git input writer panicked"))??;
    }
    Ok(output)
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
    let raw = read(
        dir,
        &["ls-files", "-z", "--others", "--exclude-standard"],
        None,
    )?;
    let mut files = Vec::new();
    for path in raw
        .split('\0')
        .filter(|p| !p.is_empty() && !p.ends_with('/'))
    {
        let full = dir.join(path);
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
            path: RelPath::from_git(path),
            lines,
            content,
            bytes,
        });
    }
    Ok(files)
}

/// The sizes of `specs` (`<rev>:<path>`) in one `git cat-file --batch-check`; `None` for one git
/// does not have.
fn object_sizes(dir: &Path, specs: &[String]) -> Vec<Option<u64>> {
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
        // Not `--literal-pathspecs`: check-ignore takes paths, and refuses pathspec magic.
        let output = run_read(dir, &GLOBAL[..3], &args, Some(&input))?;
        // Exit 1: none of them is ignored.
        match output.status.code() {
            Some(0) => Ok(String::from_utf8_lossy(&output.stdout)
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
                .collect()),
            Some(1) => Ok(BTreeSet::new()),
            _ => Err(failed(&args, &output)),
        }
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
fn worktree_bytes(full: &Path) -> io::Result<Vec<u8>> {
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
