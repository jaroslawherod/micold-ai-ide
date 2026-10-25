//! Review comments held by the service (feature 482, contracts/review-wire.md W1–W5, W11, W12).
//!
//! One [`EntryReview`] per entry, keyed by project and then by worktree directory name (`""` is
//! the Default entry, as on the wire). A project's comments and reviewed marks (feature 1271) are
//! read from its review file the first time anything asks for them, and every accepted edit is
//! written back through the catalog before memory changes (W5), so a failed write leaves them as
//! they were.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use micold_core::mcp::errors::{ErrorCategory, OpError};
use micold_core::protocol::messages::{DaemonMsg, ErrorKind, ReviewEditOp};
use micold_core::review::comment::{
    fresh_id, CommentId, CommentRef, Draft, EntryReview, Outcome, PrAnchor, Reporter,
    ReviewComment, ReviewError, Scope, SendSnapshot, Status,
};
use micold_core::review::marks::EntryMarks;
use micold_core::review::prompt::{EntryKind, PrContext};
use micold_core::review::store::ReviewFile;
use micold_core::review::{LineRange, RelPath};

use crate::catalog::Catalog;

/// Why an edit was refused, as the wire answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The `OperationError` kind.
    pub kind: ErrorKind,
    /// What to tell the user.
    pub message: String,
}

impl Refusal {
    /// A refusal of `kind` saying `message`.
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<ReviewError> for Refusal {
    fn from(err: ReviewError) -> Self {
        let kind = match err {
            ReviewError::Invalid(_) => ErrorKind::InvalidInput,
            ReviewError::NotFound => ErrorKind::NotFound,
            ReviewError::Refused | ReviewError::Draft => ErrorKind::Refused,
            ReviewError::InSend | ReviewError::Busy => ErrorKind::Busy,
            ReviewError::NothingPending | ReviewError::RefTaken => ErrorKind::InvalidInput,
        };
        Self::new(kind, err.to_string())
    }
}

/// Every project's review comments and reviewed marks the service has read so far.
#[derive(Debug, Default)]
pub struct Reviews {
    projects: HashMap<PathBuf, HashMap<String, EntryReview>>,
}

impl Reviews {
    /// `project`'s entries, comments and marks, read from its review file on first use. An entry
    /// with marks and no comments is an entry too (S4).
    fn project(&mut self, catalog: &Catalog, project: &Path) -> &mut HashMap<String, EntryReview> {
        self.projects
            .entry(project.to_path_buf())
            .or_insert_with(|| {
                let ReviewFile {
                    mut entries,
                    mut marks,
                } = catalog.load_reviews(project);
                for dir in marks.keys() {
                    entries.entry(dir.clone()).or_default();
                }
                entries
                    .into_iter()
                    .map(|(dir, comments)| {
                        let marks = EntryMarks::from_vec(marks.remove(&dir).unwrap_or_default());
                        (dir, EntryReview::from_parts(comments, marks))
                    })
                    .collect()
            })
    }

    /// One `ReviewChanged` per entry of `project` that has comments or marks (RM3), for a client
    /// that has just attached to it.
    pub fn pushes_on_attach(&mut self, catalog: &Catalog, project: &Path) -> Vec<DaemonMsg> {
        let mut entries: Vec<_> = self
            .project(catalog, project)
            .iter()
            .filter(|(_, review)| !review.is_empty())
            .map(|(dir, review)| changed(project, dir, review))
            .collect();
        // A stable order, so a window applies them the same way every time.
        entries.sort_by(|a, b| entry_dir(a).cmp(entry_dir(b)));
        entries
    }

    /// Apply `op` to entry `dir` of `project` (W1, W3, RM1), write the project's review file
    /// (W5), and return the `ReviewChanged` to push (`None` when the edit changed nothing: no
    /// write, no push, W15, W19, RM2). The caller has checked the entry exists (W2). On any
    /// refusal, memory and file are unchanged.
    pub fn apply_edit(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        op: ReviewEditOp,
        now: u64,
    ) -> Result<Option<DaemonMsg>, Refusal> {
        let entries = self.project(catalog, project);
        let mut edited = entries.get(dir).cloned().unwrap_or_default();
        match op {
            ReviewEditOp::Add {
                path,
                side,
                start,
                end,
                quote,
                text,
                pr,
            } => {
                // W18: a pull request anchor is checked, and the Default entry has no pull requests.
                let pr = match pr {
                    None => None,
                    Some(_) if dir.is_empty() => {
                        return Err(Refusal::new(
                            ErrorKind::InvalidInput,
                            "the project root has no pull request to comment on".to_owned(),
                        ))
                    }
                    Some(pr) => Some(PrAnchor::checked(
                        pr.number,
                        &pr.head_branch,
                        &pr.head_commit,
                    )?),
                };
                let draft = Draft {
                    pr,
                    path: checked_path(&path)?,
                    side,
                    range: LineRange::new(start, end).ok_or_else(|| {
                        Refusal::new(
                            ErrorKind::InvalidInput,
                            format!("invalid line range {start}..={end}: lines start at 1 and start <= end"),
                        )
                    })?,
                    quote,
                    text,
                };
                let id = fresh_id(&edited, CommentId::new);
                edited.add(draft, id, now)?;
            }
            ReviewEditOp::SetText { id, text } => edited.set_text(id, &text)?,
            ReviewEditOp::Delete { id } => edited.delete(id)?,
            // W19: only the scope's comments, and no write or push when nothing changed.
            ReviewEditOp::ClearSent { scope } => {
                edited.clear_sent(scope);
                if edited == entries.get(dir).cloned().unwrap_or_default() {
                    return Ok(None);
                }
            }
            ReviewEditOp::DiscardPending { scope } => {
                edited.discard_pending(scope);
                if edited == entries.get(dir).cloned().unwrap_or_default() {
                    return Ok(None);
                }
            }
            ReviewEditOp::Mark { id, outcome, note } => edited.mark(id, outcome, &note, now)?,
            // W15: compare-and-set, so a repeat from another window changes nothing.
            ReviewEditOp::Reanchor { moves } => {
                if edited.reanchor(&moves) == 0 {
                    return Ok(None);
                }
            }
            // RM1: a mark of the working tree, or of a pull request of a worktree; RM2: a repeat
            // changes nothing. A send freezes comments, not marks.
            ReviewEditOp::SetReviewed {
                scope,
                path,
                change,
            } => {
                match scope {
                    Scope::PullRequest(0) => {
                        return Err(Refusal::new(
                            ErrorKind::InvalidInput,
                            "a pull request's number starts at 1",
                        ))
                    }
                    Scope::PullRequest(_) if dir.is_empty() => {
                        return Err(Refusal::new(
                            ErrorKind::InvalidInput,
                            "the project root has no pull request to mark files of",
                        ))
                    }
                    Scope::WorkingTree | Scope::PullRequest(_) => {}
                }
                if !edited.set_reviewed(scope, checked_path(&path)?, change) {
                    return Ok(None);
                }
            }
        }

        catalog.save_reviews(project, &file_with(entries, dir, &edited)).map_err(|err| {
            tracing::warn!(project = %project.display(), entry = dir, %err, "review not written");
            Refusal::new(
                ErrorKind::IoFailed,
                format!("the review could not be saved: {err}"),
            )
        })?;

        let msg = changed(project, dir, &edited);
        tracing::info!(
            project = %project.display(),
            entry = dir,
            comments = edited.comments().len(),
            marks = edited.marks().to_vec().len(),
            "review changed"
        );
        if edited.is_empty() {
            entries.remove(dir);
        } else {
            entries.insert(dir.to_owned(), edited);
        }
        Ok(Some(msg))
    }

    /// Entry `dir`'s Draft comments of `scope`, for re-anchoring or reading before a send (W16,
    /// W20).
    pub fn drafts(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        scope: Scope,
    ) -> Vec<ReviewComment> {
        self.project(catalog, project)
            .get(dir)
            .map(|review| {
                review
                    .comments()
                    .iter()
                    .filter(|comment| {
                        comment.state.status() == Status::Draft && comment.scope() == scope
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Open a send of entry `dir`'s Draft comments of `scope` (W6, W20): the snapshot to deliver
    /// and the `ReviewChanged` (with `sending`) to push. `InvalidInput` with no Draft, `Busy`
    /// while another send of the entry is open. Nothing is written: `sending` is not stored.
    pub fn begin_send(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        kind: EntryKind,
        scope: (Scope, Option<&PrContext>),
        outdated: &[CommentId],
    ) -> Result<(SendSnapshot, DaemonMsg), Refusal> {
        let review = self
            .project(catalog, project)
            .get_mut(dir)
            .ok_or(ReviewError::NothingPending)?;
        let snapshot = review.begin_send(kind, scope.0, scope.1, outdated)?;
        let msg = changed(project, dir, review);
        tracing::info!(
            project = %project.display(),
            entry = dir,
            comments = snapshot.ids.len(),
            pr = scope.1.map(|pr| pr.number),
            "review send opened"
        );
        Ok((snapshot, msg))
    }

    /// Close entry `dir`'s send as delivered (W8): the snapshot's comments become `Sent { at }`
    /// and the project's review file is written. The prompt was typed, so a failed write is
    /// logged and memory keeps them sent: marking them pending again would send them twice
    /// (FR-018).
    pub fn finish_send(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        snapshot: &SendSnapshot,
        at: u64,
    ) -> DaemonMsg {
        let entries = self.project(catalog, project);
        let Some(review) = entries.get_mut(dir) else {
            return changed(project, dir, &EntryReview::default());
        };
        review.finish_send(snapshot, at);
        let msg = changed(project, dir, review);
        let file = file_with(entries, dir, &entries[dir]);
        if let Err(err) = catalog.save_reviews(project, &file) {
            tracing::warn!(project = %project.display(), entry = dir, %err, "sent review comments not written");
        }
        msg
    }

    /// Report comment `reference` of entry `dir` of `project` as `outcome` with `reply` (feature
    /// 946, contracts/agent-tool.md checks 5–8): `dir` is the caller's own entry (`""` for the
    /// Default entry). A reference of another entry of the project is refused, one of no entry is
    /// not found. The review file is written before memory changes; returns the reported comment,
    /// its status before, and the `ReviewChanged` to push (W17).
    #[allow(clippy::too_many_arguments)] // the report's own fields, as the tool gives them
    pub fn report(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        reference: CommentRef,
        outcome: Outcome,
        reply: &str,
        by: Reporter,
        at: u64,
    ) -> Result<(ReviewComment, Status, DaemonMsg), OpError> {
        let entries = self.project(catalog, project);
        let holds = |review: &EntryReview| {
            review
                .comments()
                .iter()
                .any(|comment| comment.reference() == reference)
        };
        let Some(mut reported) = entries.get(dir).filter(|review| holds(review)).cloned() else {
            return Err(if entries.values().any(holds) {
                OpError::new(
                    ErrorCategory::RefusedByPolicy,
                    "that comment belongs to another worktree; a session reports only on its own entry's comments",
                )
            } else {
                OpError::not_found("no such comment; it may have been cleared or discarded")
            });
        };
        let previous =
            reported
                .report(reference, outcome, reply, by, at)
                .map_err(|err| match err {
                    ReviewError::Invalid(why) => OpError::invalid_input(why),
                    ReviewError::InSend => OpError::new(
                        ErrorCategory::Conflict,
                        "that comment is being sent and has not been delivered yet",
                    ),
                    ReviewError::Draft => OpError::new(
                        ErrorCategory::Conflict,
                        "that comment is still a draft; it has not been sent to you",
                    ),
                    _ => {
                        OpError::not_found("no such comment; it may have been cleared or discarded")
                    }
                })?;
        catalog
            .save_reviews(project, &file_with(entries, dir, &reported))
            .map_err(|err| {
                tracing::warn!(project = %project.display(), entry = dir, %err, "review report not written");
                OpError::service_error(format!("the report could not be saved: {err}"))
            })?;
        let comment = reported
            .comments()
            .iter()
            .find(|comment| comment.reference() == reference)
            .cloned()
            .expect("the reported comment is in its entry");
        let msg = changed(project, dir, &reported);
        tracing::info!(project = %project.display(), entry = dir, comment = %reference, "review comment reported");
        entries.insert(dir.to_owned(), reported);
        Ok((comment, previous, msg))
    }

    /// Forget worktree `dir` of `project` once it is deleted (W11, RM5): its comments and marks
    /// leave memory and the review file, and the empty `ReviewChanged` to push is returned. A
    /// failed write is logged; memory forgets them regardless, since the worktree is gone.
    pub fn forget_worktree(&mut self, catalog: &Catalog, project: &Path, dir: &str) -> DaemonMsg {
        let entries = self.project(catalog, project);
        if entries.remove(dir).is_some() {
            save_logged(
                catalog,
                project,
                entries,
                "deleted worktree's review comments not removed",
            );
            tracing::info!(project = %project.display(), entry = dir, "review comments of a deleted worktree removed");
        }
        changed(project, dir, &EntryReview::default())
    }

    /// Forget the comments of every worktree of `project` that is gone (Edge Case "removed
    /// worktree": removed outside the app): `gone` says whether a worktree directory name no
    /// longer exists. The Default entry is never pruned. Returns the empty `ReviewChanged` of each
    /// one pruned.
    pub fn prune_worktrees(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        gone: impl Fn(&str) -> bool,
    ) -> Vec<DaemonMsg> {
        let entries = self.project(catalog, project);
        let mut pruned: Vec<String> = entries
            .keys()
            .filter(|dir| !dir.is_empty() && gone(dir))
            .cloned()
            .collect();
        if pruned.is_empty() {
            return Vec::new();
        }
        pruned.sort();
        for dir in &pruned {
            entries.remove(dir);
        }
        save_logged(
            catalog,
            project,
            entries,
            "removed worktrees' review comments not pruned",
        );
        tracing::info!(project = %project.display(), entries = ?pruned, "review comments of removed worktrees pruned");
        pruned
            .iter()
            .map(|dir| changed(project, dir, &EntryReview::default()))
            .collect()
    }

    /// Forget a project removed from the catalog: its comments and marks leave memory (the
    /// catalog deletes its review file, W11, RM5).
    pub fn forget_project(&mut self, project: &Path) {
        self.projects.remove(project);
    }

    /// Close entry `dir`'s send undelivered (W9): every comment stays as it was.
    pub fn abort_send(&mut self, catalog: &Catalog, project: &Path, dir: &str) -> DaemonMsg {
        let entries = self.project(catalog, project);
        match entries.get_mut(dir) {
            Some(review) => {
                review.abort_send();
                changed(project, dir, review)
            }
            None => changed(project, dir, &EntryReview::default()),
        }
    }
}

/// Write `entries` as `project`'s review file, logging a failure as `what`.
fn save_logged(
    catalog: &Catalog,
    project: &Path,
    entries: &HashMap<String, EntryReview>,
    what: &str,
) {
    let file = file_of(entries.iter().map(|(dir, review)| (dir.as_str(), review)));
    if let Err(err) = catalog.save_reviews(project, &file) {
        tracing::warn!(project = %project.display(), %err, "{what}");
    }
}

/// The review file of a project whose entry `dir` holds `review` and every other entry what
/// `entries` holds.
fn file_with(
    entries: &HashMap<String, EntryReview>,
    dir: &str,
    review: &EntryReview,
) -> ReviewFile {
    file_of(
        entries
            .iter()
            .filter(|(other, _)| other.as_str() != dir)
            .map(|(other, review)| (other.as_str(), review))
            .chain([(dir, review)]),
    )
}

/// The review file holding `entries`' comments and marks.
fn file_of<'a>(entries: impl Iterator<Item = (&'a str, &'a EntryReview)>) -> ReviewFile {
    let mut file = ReviewFile::default();
    for (dir, review) in entries {
        file.entries
            .insert(dir.to_owned(), review.comments().to_vec());
        file.marks.insert(dir.to_owned(), review.marks().to_vec());
    }
    file
}

/// W1: a path relative to the entry root, `/`-separated.
fn checked_path(path: &str) -> Result<RelPath, Refusal> {
    if path.contains('\\') {
        return Err(Refusal::new(
            ErrorKind::InvalidInput,
            "a comment's path is `/`-separated and has no `\\`",
        ));
    }
    RelPath::from_native(path).ok_or_else(|| {
        Refusal::new(
            ErrorKind::InvalidInput,
            "a comment's path is relative to the entry and not empty",
        )
    })
}

fn changed(project: &Path, dir: &str, review: &EntryReview) -> DaemonMsg {
    DaemonMsg::ReviewChanged {
        project: project.to_path_buf(),
        worktree_dir: dir.to_owned(),
        comments: review.comments().to_vec(),
        sending: review.sending(),
        marks: review.marks().to_vec(),
    }
}

fn entry_dir(msg: &DaemonMsg) -> &str {
    match msg {
        DaemonMsg::ReviewChanged { worktree_dir, .. } => worktree_dir,
        _ => "",
    }
}

/// What re-anchoring an entry's Drafts against its working tree found (W16).
#[derive(Debug, Default)]
pub struct Relocated {
    /// The Drafts whose quoted code moved.
    pub moves: Vec<micold_core::review::comment::Move>,
    /// The current-side Drafts whose code is gone or ambiguous: the prompt notes them outdated.
    pub outdated: Vec<CommentId>,
}

/// Locate each current-side Draft of `drafts` in its file under `root` (W16). A file that is
/// missing or not text holds nothing: its Drafts are outdated. Reads files, so it runs on the
/// blocking pool, outside the state lock.
pub fn relocate(root: &Path, drafts: &[ReviewComment]) -> Relocated {
    use micold_core::review::anchor::{locate, Located};
    use micold_core::review::comment::Move;
    use micold_core::review::diff::SideLines;
    use micold_core::review::Side;

    let mut found = Relocated::default();
    let mut files: HashMap<&str, Option<SideLines>> = HashMap::new();
    // A pull request comment is placed against its pull request's diff, never the working tree.
    for comment in drafts
        .iter()
        .filter(|c| c.side == Side::New && c.pr.is_none())
    {
        let lines = files.entry(comment.path.as_str()).or_insert_with(|| {
            std::fs::read(root.join(comment.path.as_str()))
                .ok()
                .and_then(|bytes| SideLines::from_bytes(&bytes))
        });
        let located = lines.as_ref().map_or(Located::Gone, |lines| {
            locate(&comment.quote, comment.range, lines)
        });
        match located {
            Located::Same(_) => {}
            Located::Moved(to) => {
                found.moves.extend(Move::new(comment.id, comment.range, to));
            }
            Located::Gone | Located::Ambiguous => found.outdated.push(comment.id),
        }
    }
    found
}
