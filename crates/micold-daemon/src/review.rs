//! Review comments held by the service (feature 482, contracts/review-wire.md W1–W3, W5, W12).
//!
//! One [`EntryReview`] per entry, keyed by project and then by worktree directory name (`""` is
//! the Default entry, as on the wire). A project's comments are read from its review file the
//! first time anything asks for them, and every accepted edit is written back through the catalog
//! before memory changes (W5), so a failed write leaves the comments as they were.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use micold_core::protocol::messages::{DaemonMsg, ErrorKind, ReviewEditOp};
use micold_core::review::comment::{CommentId, Draft, EntryReview, ReviewError, SendSnapshot};
use micold_core::review::prompt::EntryKind;
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
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
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
            ReviewError::Refused => ErrorKind::Refused,
            ReviewError::InSend | ReviewError::Busy => ErrorKind::Busy,
            ReviewError::NothingPending => ErrorKind::InvalidInput,
        };
        Self::new(kind, err.to_string())
    }
}

/// Every project's review comments the service has read so far.
#[derive(Debug, Default)]
pub struct Reviews {
    projects: HashMap<PathBuf, HashMap<String, EntryReview>>,
}

impl Reviews {
    /// `project`'s entries, read from its review file on first use.
    fn project(&mut self, catalog: &Catalog, project: &Path) -> &mut HashMap<String, EntryReview> {
        self.projects
            .entry(project.to_path_buf())
            .or_insert_with(|| {
                catalog
                    .load_reviews(project)
                    .entries
                    .into_iter()
                    .map(|(dir, comments)| (dir, EntryReview::from_comments(comments)))
                    .collect()
            })
    }

    /// One `ReviewChanged` per entry of `project` that has comments, for a client that has just
    /// attached to it.
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

    /// Apply `op` to entry `dir` of `project` (W1, W3), write the project's review file (W5), and
    /// return the `ReviewChanged` to push. The caller has checked the entry exists (W2). On any
    /// refusal, memory and file are unchanged.
    pub fn apply_edit(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        op: ReviewEditOp,
        now: u64,
    ) -> Result<DaemonMsg, Refusal> {
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
            } => {
                let draft = Draft {
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
                edited.add(draft, CommentId::new(), now)?;
            }
            ReviewEditOp::SetText { id, text } => edited.set_text(id, &text)?,
            ReviewEditOp::Delete { id } => edited.delete(id)?,
            ReviewEditOp::ClearSent | ReviewEditOp::DiscardPending => {
                return Err(Refusal::new(
                    ErrorKind::Refused,
                    "clearing comments is not available in this build",
                ));
            }
        }

        catalog.save_reviews(project, &file_with(entries, dir, &edited)).map_err(|err| {
            tracing::warn!(project = %project.display(), entry = dir, %err, "review comments not written");
            Refusal::new(
                ErrorKind::IoFailed,
                format!("the comments could not be saved: {err}"),
            )
        })?;

        let msg = changed(project, dir, &edited);
        tracing::info!(
            project = %project.display(),
            entry = dir,
            comments = edited.comments().len(),
            "review comments changed"
        );
        if edited.is_empty() {
            entries.remove(dir);
        } else {
            entries.insert(dir.to_owned(), edited);
        }
        Ok(msg)
    }

    /// Open a send of entry `dir`'s pending comments (W6): the snapshot to deliver and the
    /// `ReviewChanged` (with `sending`) to push. `InvalidInput` with nothing pending, `Busy` while
    /// another send of the entry is open. Nothing is written: `sending` is not stored.
    pub fn begin_send(
        &mut self,
        catalog: &Catalog,
        project: &Path,
        dir: &str,
        kind: EntryKind,
        outdated: &[CommentId],
    ) -> Result<(SendSnapshot, DaemonMsg), Refusal> {
        let review = self
            .project(catalog, project)
            .get_mut(dir)
            .ok_or(ReviewError::NothingPending)?;
        let snapshot = review.begin_send(kind, outdated)?;
        let msg = changed(project, dir, review);
        tracing::info!(
            project = %project.display(),
            entry = dir,
            comments = snapshot.ids.len(),
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

/// The review file of a project whose entry `dir` holds `review` and every other entry what
/// `entries` holds.
fn file_with(
    entries: &HashMap<String, EntryReview>,
    dir: &str,
    review: &EntryReview,
) -> ReviewFile {
    let mut file = ReviewFile {
        entries: entries
            .iter()
            .filter(|(other, _)| other.as_str() != dir)
            .map(|(other, review)| (other.clone(), review.comments().to_vec()))
            .collect(),
    };
    file.entries
        .insert(dir.to_owned(), review.comments().to_vec());
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
    }
}

fn entry_dir(msg: &DaemonMsg) -> &str {
    match msg {
        DaemonMsg::ReviewChanged { worktree_dir, .. } => worktree_dir,
        _ => "",
    }
}
