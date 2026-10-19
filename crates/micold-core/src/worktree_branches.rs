//! A worktree's branch set (feature 1003, research R2, contracts/pull-request-list.md §1): its
//! current branch plus every branch that was created in it, read from git's own reflogs so it
//! survives restarts with no stored state (FR-002).
//!
//! All of it is pure but [`read_branch_set`], which asks git through the [`Git`] seam. Nothing is
//! written; branch names come from git's own output.

use std::path::Path;

use crate::git::Git;

/// How far apart, in seconds, a branch's creation and the HEAD entry that moved onto it may be
/// (R2 rule 2): one git command writes both and can straddle a second boundary.
pub const MATCH_WINDOW_SECS: u64 = 2;

/// What git writes as the subject of a branch's first reflog entry.
const CREATED_FROM: &str = "branch: Created from ";
/// The subject of a HEAD entry that moved HEAD from one branch or commit to another.
const CHECKOUT: &str = "checkout: moving from ";
/// The subject of a HEAD entry that renamed the branch.
const RENAMED: &str = "Branch: renamed refs/heads/";
/// Where a local branch's ref lives.
const LOCAL: &str = "refs/heads/";

/// One reflog line as `git log -g --date=unix --format=%gD%x09%H%x09%gs` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflogEntry {
    /// The ref, without its `@{…}`: `HEAD` or `refs/heads/<name>`.
    pub refname: String,
    /// When the entry was written, Unix seconds.
    pub time: u64,
    /// The commit the ref pointed at after the entry.
    pub commit: String,
    /// What git wrote as the reason: `checkout: moving from <A> to <B>`, `branch: Created from …`.
    pub subject: String,
}

/// The entries of reflog text, in git's order (newest first per ref). A line that does not split
/// into three tab-separated fields, or whose `@{…}` is not a number, is skipped.
pub fn parse_reflog(text: &str) -> Vec<ReflogEntry> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<ReflogEntry> {
    let mut fields = line.splitn(3, '\t');
    let (selector, commit, subject) = (fields.next()?, fields.next()?, fields.next()?);
    let (refname, time) = selector.strip_suffix('}')?.rsplit_once("@{")?;
    Some(ReflogEntry {
        refname: refname.to_string(),
        time: time.parse().ok()?,
        commit: commit.to_string(),
        subject: subject.to_string(),
    })
}

/// Whether a name of a checkout or rename entry is a branch: not `HEAD`, and not a commit name (40
/// or 64 lowercase hex characters), which is what a checkout entry names when HEAD is detached.
fn is_branch(name: &str) -> bool {
    let commit = matches!(name.len(), 40 | 64)
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    !name.is_empty() && name != "HEAD" && !commit
}

/// `(<A>, <B>)` of `checkout: moving from <A> to <B>`.
fn checkout(subject: &str) -> Option<(&str, &str)> {
    subject.strip_prefix(CHECKOUT)?.split_once(" to ")
}

/// `(<X>, <Y>)` of `Branch: renamed refs/heads/<X> to refs/heads/<Y>`.
fn rename(subject: &str) -> Option<(&str, &str)> {
    let (from, to) = subject.strip_prefix(RENAMED)?.split_once(" to ")?;
    Some((from, to.strip_prefix(LOCAL)?))
}

/// Both names of a checkout or rename entry, in that order.
fn named(subject: &str) -> Option<(&str, &str)> {
    checkout(subject).or_else(|| rename(subject))
}

/// The branch names the worktree's HEAD reflog names (R2 rule 1): the current branch, then both
/// names of every checkout and rename entry, deduplicated in first-seen order. Commit names and
/// `HEAD` are not branches.
pub fn candidates(head_log: &[ReflogEntry], current: Option<&str>) -> Vec<String> {
    let names = head_log
        .iter()
        .filter_map(|entry| named(&entry.subject))
        .flat_map(|(a, b)| [a, b]);
    let mut out: Vec<String> = Vec::new();
    for name in current.into_iter().chain(names) {
        if is_branch(name) && !out.iter().any(|seen| seen == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// Whether a creation `subject` copies the remote branch of the same `name` (R2 rule 3):
/// `branch: Created from refs/remotes/<remote>/<name>` or `branch: Created from <remote>/<name>`.
pub fn copies_remote(subject: &str, name: &str) -> bool {
    let Some(source) = subject.strip_prefix(CREATED_FROM) else {
        return false;
    };
    let source = source.strip_prefix("refs/remotes/").unwrap_or(source);
    source
        .split_once('/')
        .is_some_and(|(remote, rest)| !remote.is_empty() && rest == name)
}

/// One HEAD entry with the branch (or commit) HEAD is on after it.
struct Position<'a> {
    time: u64,
    commit: &'a str,
    on: &'a str,
}

/// Where HEAD is after each entry (R2 rule 2), walked oldest first: a checkout moves it to `<B>`, a
/// rename to `<Y>`; entries before the first checkout are on that checkout's `<A>`, or on the
/// current branch when there is no checkout.
fn positions<'a>(head_log: &'a [ReflogEntry], current: Option<&'a str>) -> Vec<Position<'a>> {
    let start = head_log
        .iter()
        .rev()
        .find_map(|entry| checkout(&entry.subject).map(|(from, _)| from))
        .or(current)
        .unwrap_or("");
    let mut on = start;
    head_log
        .iter()
        .rev()
        .map(|entry| {
            if let Some((_, to)) = named(&entry.subject) {
                on = to;
            }
            Position {
                time: entry.time,
                commit: &entry.commit,
                on,
            }
        })
        .collect()
}

/// `name` and every name it was renamed from, as far back as the HEAD reflog's renames go.
fn former_names<'a>(name: &'a str, renames: &[(&'a str, &'a str)]) -> Vec<&'a str> {
    let mut names = vec![name];
    let mut i = 0;
    while let Some(&current) = names.get(i) {
        for &(from, to) in renames {
            if to == current && !names.contains(&from) {
                names.push(from);
            }
        }
        i += 1;
    }
    names
}

/// Whether `name` counts on its own record (R2 rules 2 to 4).
fn created_here(
    name: &str,
    ref_logs: &[ReflogEntry],
    positions: &[Position<'_>],
    renames: &[(&str, &str)],
) -> bool {
    let refname = format!("{LOCAL}{name}");
    // git prints each ref's entries newest first: the last one is the oldest still held.
    let Some(oldest) = ref_logs.iter().rev().find(|entry| entry.refname == refname) else {
        return true; // deleted: the record is gone (rule 4)
    };
    if !oldest.subject.starts_with(CREATED_FROM) {
        return true; // expired: the creation is gone (rule 4)
    }
    if copies_remote(&oldest.subject, name) {
        return false; // the remote's branch (rule 3)
    }
    let names = former_names(name, renames);
    positions.iter().any(|p| {
        p.commit == oldest.commit
            && p.time.abs_diff(oldest.time) <= MATCH_WINDOW_SECS
            && names.contains(&p.on)
    })
}

/// The worktree's branch set from its HEAD reflog and the reflogs of the candidates' refs (R2
/// rules 2 to 6), in [`candidates`] order. The default branch is not dropped here: the pull request
/// read names it (R3).
pub fn branch_set(
    head_log: &[ReflogEntry],
    ref_logs: &[ReflogEntry],
    current: Option<&str>,
) -> Vec<String> {
    let names = candidates(head_log, current);
    let positions = positions(head_log, current);
    let renames: Vec<(&str, &str)> = head_log.iter().filter_map(|e| rename(&e.subject)).collect();
    let mut kept: Vec<bool> = names
        .iter()
        .map(|name| {
            Some(name.as_str()) == current || created_here(name, ref_logs, &positions, &renames)
        })
        .collect();
    // Rule 5: a renamed branch counts under both names when either counts.
    let index = |name: &str| names.iter().position(|n| n == name);
    let mut changed = true;
    while changed {
        changed = false;
        for &(from, to) in &renames {
            if let (Some(a), Some(b)) = (index(from), index(to)) {
                if kept[a] != kept[b] {
                    kept[a] = true;
                    kept[b] = true;
                    changed = true;
                }
            }
        }
    }
    names
        .into_iter()
        .zip(kept)
        .filter_map(|(name, keep)| keep.then_some(name))
        .collect()
}

/// Read the branch set of the worktree at `dir`: one HEAD reflog run, then one run over the
/// candidates' refs. A failed HEAD run gives the current branch alone; a failed refs run treats
/// every candidate's record as gone (R2 rule 4).
pub fn read_branch_set<G: Git + ?Sized>(git: &G, dir: &Path, current: Option<&str>) -> Vec<String> {
    let Ok(text) = git.reflogs(dir, &["HEAD".to_string()]) else {
        return current.map(str::to_string).into_iter().collect();
    };
    let head_log = parse_reflog(&text);
    let refs: Vec<String> = candidates(&head_log, current)
        .iter()
        .map(|name| format!("{LOCAL}{name}"))
        .collect();
    let ref_logs = git
        .reflogs(dir, &refs)
        .map(|text| parse_reflog(&text))
        .unwrap_or_default();
    branch_set(&head_log, &ref_logs, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    const C0: &str = "1111111111111111111111111111111111111111";
    const C1: &str = "2222222222222222222222222222222222222222";
    const SHA256: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    /// A HEAD reflog entry.
    fn head(time: u64, commit: &str, subject: &str) -> ReflogEntry {
        entry("HEAD", time, commit, subject)
    }

    /// An entry of `refs/heads/<branch>`.
    fn branch(name: &str, time: u64, commit: &str, subject: &str) -> ReflogEntry {
        entry(&format!("refs/heads/{name}"), time, commit, subject)
    }

    fn entry(refname: &str, time: u64, commit: &str, subject: &str) -> ReflogEntry {
        ReflogEntry {
            refname: refname.to_string(),
            time,
            commit: commit.to_string(),
            subject: subject.to_string(),
        }
    }

    fn checkout(from: &str, to: &str) -> String {
        format!("checkout: moving from {from} to {to}")
    }

    const CREATED_HERE: &str = "branch: Created from HEAD";

    fn has(set: &[String], name: &str) -> bool {
        set.iter().any(|b| b == name)
    }

    /// `feat` created at `created` (commit C1), HEAD moved onto it at `moved`, then on to `other`.
    fn feat_moved_at(created: u64, moved: u64, creation: &str) -> Vec<String> {
        let head_log = [
            head(900, C1, &checkout("feat", "other")),
            head(moved, C1, &checkout("main", "feat")),
        ];
        let refs = [branch("feat", created, C1, creation)];
        branch_set(&head_log, &refs, Some("other"))
    }

    #[test]
    fn parse_reflog_reads_each_line_and_strips_the_selector() {
        let text = format!(
            "HEAD@{{100}}\t{C1}\t{}\nrefs/heads/feat/x@{{5}}\t{C0}\t\n",
            checkout("main", "feat/x")
        );
        assert_eq!(
            parse_reflog(&text),
            [
                head(100, C1, &checkout("main", "feat/x")),
                branch("feat/x", 5, C0, ""),
            ],
            "refname without its @{{…}}, time from it, an empty subject kept"
        );
    }

    #[test]
    fn parse_reflog_skips_a_line_that_is_not_three_fields_or_has_no_numeric_selector() {
        let text = format!(
            "garbage\nHEAD@{{x}}\t{C1}\ts\nHEAD\t{C1}\ts\nHEAD@{{7}}\t{C1}\nHEAD@{{8}}\t{C1}\tok\n\n"
        );
        assert_eq!(
            parse_reflog(&text),
            [head(8, C1, "ok")],
            "only the whole line is read"
        );
    }

    #[test]
    fn candidates_are_the_current_branch_then_every_checkout_and_rename_name_once() {
        let head_log = [
            head(6, C1, &checkout(C1, "feat")),
            head(5, C1, &checkout("feat", "HEAD")),
            head(4, C1, &checkout(SHA256, "feat")),
            head(3, C1, "Branch: renamed refs/heads/x to refs/heads/y"),
            head(2, C1, &checkout("main", "x")),
            head(1, C0, "commit: something"),
        ];
        assert_eq!(
            candidates(&head_log, Some("cur")),
            ["cur", "feat", "x", "y", "main"],
            "commit names and HEAD are not branches; each name once, first seen first"
        );
        assert_eq!(
            candidates(&head_log, None),
            ["feat", "x", "y", "main"],
            "a detached HEAD adds no current branch"
        );
    }

    #[test]
    fn copies_remote_matches_only_a_same_name_remote_source() {
        assert!(
            copies_remote("branch: Created from refs/remotes/origin/feat", "feat"),
            "the full remote ref of the same name"
        );
        assert!(
            copies_remote("branch: Created from origin/feat", "feat"),
            "the short remote name of the same name"
        );
        assert!(
            copies_remote("branch: Created from origin/feat/x", "feat/x"),
            "a branch name with a slash"
        );
        assert!(
            !copies_remote("branch: Created from origin/main", "feat"),
            "a remote branch of another name"
        );
        assert!(
            !copies_remote(CREATED_HERE, "feat"),
            "a branch created from HEAD"
        );
        assert!(
            !copies_remote("branch: Created from main", "main"),
            "a local source is not a remote"
        );
        assert!(
            !copies_remote("commit: origin/feat", "feat"),
            "only a branch creation counts"
        );
    }

    #[test]
    fn a_branch_created_here_is_kept() {
        assert!(has(&feat_moved_at(100, 100, CREATED_HERE), "feat"));
    }

    #[test]
    fn the_match_window_is_two_seconds_either_way() {
        let created = 100;
        for moved in [created - MATCH_WINDOW_SECS, created + MATCH_WINDOW_SECS] {
            assert!(
                has(&feat_moved_at(created, moved, CREATED_HERE), "feat"),
                "HEAD moved at {moved}, created at {created}: within the window"
            );
        }
        for moved in [
            created - MATCH_WINDOW_SECS - 1,
            created + MATCH_WINDOW_SECS + 1,
        ] {
            assert!(
                !has(&feat_moved_at(created, moved, CREATED_HERE), "feat"),
                "HEAD moved at {moved}, created at {created}: outside the window"
            );
        }
    }

    #[test]
    fn a_branch_created_elsewhere_and_checked_out_here_is_dropped() {
        assert!(!has(&feat_moved_at(100, 500, CREATED_HERE), "feat"));
    }

    #[test]
    fn a_match_at_another_commit_is_not_a_creation_here() {
        let head_log = [
            head(900, C1, &checkout("feat", "other")),
            head(100, C0, &checkout("main", "feat")),
        ];
        let refs = [branch("feat", 100, C1, CREATED_HERE)];
        assert!(!has(&branch_set(&head_log, &refs, Some("other")), "feat"));
    }

    #[test]
    fn the_worktree_add_start_is_on_the_first_checkouts_source() {
        // `git worktree add -b A`, then `git switch -c B`.
        let head_log = [
            head(200, C1, &checkout("A", "B")),
            head(100, C0, "reset: moving to HEAD"),
            head(100, C0, ""),
        ];
        let refs = [
            branch("A", 100, C0, CREATED_HERE),
            branch("B", 200, C1, CREATED_HERE),
        ];
        assert_eq!(branch_set(&head_log, &refs, Some("B")), ["B", "A"]);
    }

    #[test]
    fn a_same_name_remote_copy_is_dropped() {
        for source in [
            "branch: Created from refs/remotes/origin/feat",
            "branch: Created from origin/feat",
        ] {
            assert!(
                !has(&feat_moved_at(100, 100, source), "feat"),
                "{source}: the remote's branch, created elsewhere"
            );
        }
    }

    #[test]
    fn a_new_branch_from_another_remote_branch_is_kept() {
        // `git switch -c feat origin/main`.
        assert!(has(
            &feat_moved_at(100, 100, "branch: Created from origin/main"),
            "feat"
        ));
    }

    #[test]
    fn a_deleted_branch_is_kept() {
        let head_log = [
            head(900, C1, &checkout("feat", "other")),
            head(100, C1, &checkout("main", "feat")),
        ];
        let set = branch_set(&head_log, &[], Some("other"));
        assert!(has(&set, "feat"), "no ref log: the record is gone, kept");
    }

    #[test]
    fn a_branch_whose_creation_expired_is_kept() {
        let head_log = [
            head(900, C1, &checkout("feat", "other")),
            head(500, C1, &checkout("main", "feat")),
        ];
        let refs = [branch("feat", 400, C1, "commit: later work")];
        assert!(has(&branch_set(&head_log, &refs, Some("other")), "feat"));
    }

    #[test]
    fn a_renamed_branch_counts_under_both_names() {
        let head_log = [
            head(400, C1, &checkout("y", "other")),
            head(300, C1, "Branch: renamed refs/heads/x to refs/heads/y"),
            head(100, C1, &checkout("main", "x")),
        ];
        let refs = [
            branch("y", 300, C1, "Branch: renamed refs/heads/x to refs/heads/y"),
            branch("y", 100, C1, CREATED_HERE),
            branch("main", 10, C0, "branch: Created from HEAD"),
        ];
        let set = branch_set(&head_log, &refs, Some("other"));
        assert!(has(&set, "y"), "created here under its old name");
        assert!(has(&set, "x"), "and listed under that name too");
        assert!(!has(&set, "main"), "main was created elsewhere");
    }

    #[test]
    fn the_current_branch_always_counts() {
        let head_log = [head(500, C1, &checkout("main", "dev"))];
        let refs = [
            branch("dev", 100, C1, CREATED_HERE),
            branch("main", 10, C0, CREATED_HERE),
        ];
        assert_eq!(branch_set(&head_log, &refs, Some("dev")), ["dev"]);
    }

    #[test]
    fn a_detached_head_gives_only_the_created_branches() {
        let head_log = [
            head(900, C1, &checkout("feat", C1)),
            head(100, C1, &checkout("main", "feat")),
        ];
        let refs = [
            branch("feat", 100, C1, CREATED_HERE),
            branch("main", 10, C0, CREATED_HERE),
        ];
        assert_eq!(branch_set(&head_log, &refs, None), ["feat"]);
    }
}
