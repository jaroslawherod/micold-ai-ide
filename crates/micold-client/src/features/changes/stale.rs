//! The Review tab's hold (feature 1275, US1, FR-005–FR-014, data-model §10–§13): a background
//! read that differs from the list on screen is held, not shown; the header counts what changed
//! since the user looked, those rows carry a dot, and the notice applies. The comparison itself
//! is `micold_core::review::stale`; this module only places it in the view's state.

use std::io;

use micold_core::review::changes::{ChangeKind, ChangeList, ChangedFile};
use micold_core::review::pr_diff::{PrDiff, PrFile};
use micold_core::review::stale::{observe, Held, Listing, Observed};
use micold_core::review::RelPath;

use super::{
    close_new, judge_anchors, pr_diff_ready, pr_file_diff, reanchor_effect, request_diff,
    reset_diff_pane, shown_list, Effect, Load, OpenView, PrDiffLoad, PullRequestView, Source,
    State,
};

/// The selected file's row as the diff read found it, read in the same job (research R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRow {
    /// Not read: the diff is not of the working tree, or the row's read failed (its error is not
    /// shown: the diff still shows).
    NotRead,
    /// The read lists no row for the file: it has no changes any more.
    Unlisted,
    /// The file's current row.
    Row(ChangedFile),
}

impl DiffRow {
    /// The row of `path` in a pathspec-limited read (`GitCli::change_list_of` of the path and a
    /// rename's old path): a failed read is [`DiffRow::NotRead`]; a rename is found by either path.
    pub fn from_read(read: io::Result<ChangeList>, path: &RelPath) -> Self {
        let Ok(list) = read else {
            return Self::NotRead;
        };
        list.files
            .into_iter()
            .find(|row| {
                &row.path == path
                    || matches!(&row.kind, ChangeKind::Renamed { from } if from == path)
            })
            .map_or(Self::Unlisted, Self::Row)
    }
}

/// A pull request's newer diff held back, and the head it was read at: what comments record
/// once it is shown (research R6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldPr {
    /// The latest diff and the files it changes.
    pub held: Held<PrDiff>,
    /// The head the latest diff was read at.
    pub head: String,
}

/// The header's notice: *1 file changed since you looked* or *n files changed since you looked*;
/// `None` while nothing is held for the shown source (FR-007).
pub fn stale_notice(view: &OpenView) -> Option<String> {
    let count = match &view.source {
        Source::Changes => view.held.as_ref()?.count(),
        Source::PullRequest(pr) => pr.held.as_ref()?.held.count(),
    };
    Some(match count {
        1 => "1 file changed since you looked".to_owned(),
        n => format!("{n} files changed since you looked"),
    })
}

/// Whether `path`'s row carries the *changed since you looked* dot (FR-008).
pub fn changed_since_you_looked(view: &OpenView, path: &RelPath) -> bool {
    match &view.source {
        Source::Changes => view.held.as_ref().is_some_and(|held| held.contains(path)),
        Source::PullRequest(pr) => pr.held.as_ref().is_some_and(|h| h.held.contains(path)),
    }
}

/// Whether the user selected the selected file while it was changed since they looked and the
/// latest read no longer lists it: the diff area says it has no changes any more (US1 AS11). A
/// file that drops out of a background read while open keeps its diff (H1, FR-014).
pub fn no_longer_changed(view: &OpenView) -> bool {
    view.chose_gone && selection_gone(view)
}

/// The user just selected the selected file: remember whether it was gone then.
pub(super) fn note_selection(view: &mut OpenView) {
    view.chose_gone = selection_gone(view);
}

/// The head the shown patch of `path` was read at: the head of the read it was adopted from, or
/// the pull request's.
pub fn head_of<'a>(pr: &'a PullRequestView, path: &RelPath) -> &'a str {
    pr.adopted.get(path).unwrap_or(&pr.head)
}

/// Whether the selected file is in the changed-since set and the latest read does not list it.
fn selection_gone(view: &OpenView) -> bool {
    fn gone<L: Listing>(held: Option<&Held<L>>, selected: Option<&RelPath>) -> bool {
        match (held, selected) {
            (Some(held), Some(path)) => held.contains(path) && !held.latest_lists(path),
            _ => false,
        }
    }
    match &view.source {
        Source::Changes => gone(view.held.as_ref(), view.selected.as_ref()),
        Source::PullRequest(pr) => gone(pr.held.as_ref().map(|h| &h.held), pr.selected.as_ref()),
    }
}

/// The working-tree list on screen when it has rows: what a background read is held against.
/// Empty, unread or failed, a change applies at once (FR-012).
pub(super) fn list_on_screen(view: &OpenView) -> Option<&ChangeList> {
    shown_list(view).filter(|list| !list.files.is_empty())
}

/// Take the list on screen to hold a background read against; `None` (the list left as it is)
/// when the read is the user's (`apply`) or no rows are on screen.
pub(super) fn take_list_on_screen(view: &mut OpenView, apply: bool) -> Option<ChangeList> {
    if apply || list_on_screen(view).is_none() {
        return None;
    }
    match std::mem::take(&mut view.list) {
        Load::Ready(list)
        | Load::Loading {
            last: Some(list), ..
        } => Some(list),
        _ => None,
    }
}

/// The list on screen, to change in place.
fn shown_list_mut(list: &mut Load<ChangeList>) -> Option<&mut ChangeList> {
    match list {
        Load::Ready(list)
        | Load::Loading {
            last: Some(list), ..
        } => Some(list),
        _ => None,
    }
}

/// A background read answered while `shown` is on screen (research R3, R4): a read that changes
/// nothing replaces it silently; one that differs is held and `shown` stays; a failed one keeps
/// the list and the hold and shows no error.
pub(super) fn hold_read(view: &mut OpenView, shown: ChangeList, read: Result<ChangeList, String>) {
    let Ok(latest) = read else {
        view.list = Load::Ready(shown);
        return;
    };
    let observed = match view.held.take() {
        Some(held) => held.observe(&shown, latest),
        None => observe(&shown, latest),
    };
    view.list = Load::Ready(match observed {
        Observed::Same(latest) => latest,
        Observed::Held(held) => {
            view.held = Some(held);
            shown
        }
    });
    if !selection_gone(view) {
        view.chose_gone = false;
    }
}

/// The selection no longer listed loses its diff and a new comment on it (as a refresh does).
pub(super) fn drop_unlisted_selection(view: &mut OpenView) {
    let listed = match (&view.selected, shown_list(view)) {
        (Some(selected), Some(list)) => list.files.iter().any(|f| &f.path == selected),
        _ => true,
    };
    if !listed {
        view.selected = None;
        view.diff = Load::Idle;
        close_new(view);
    }
}

/// The notice was pressed (FR-007, FR-010): the held read is shown, the selected file kept while
/// listed and its diff read again with the old one kept meanwhile; a pull request's latest diff
/// replaces the shown one. Anchors are judged again.
pub(super) fn applied(state: &mut State) -> Effect {
    let Some(view) = state.tabs.shown_view_mut() else {
        return Effect::None;
    };
    if let Source::PullRequest(pr) = &mut view.source {
        let Some(HeldPr { held, head }) = pr.held.take() else {
            return Effect::None;
        };
        let mut shown = PrDiff::default();
        held.apply(&mut shown);
        show_pr_diff(view, shown, head);
        let moves = judge_anchors(state);
        return reanchor_effect(state, moves);
    }
    if !apply_tree_hold(view) {
        return Effect::None;
    }
    judge_anchors(state);
    request_diff(state, true)
}

/// Show the working tree's held read, if any; whether there was one.
pub(super) fn apply_tree_hold(view: &mut OpenView) -> bool {
    let Some(held) = view.held.take() else {
        return false;
    };
    view.chose_gone = false;
    if let Some(list) = shown_list_mut(&mut view.list) {
        held.apply(list);
    }
    drop_unlisted_selection(view);
    true
}

/// The working-tree row the diff read found for the selected file (FR-009, research R5). Under a
/// hold the shown and the latest list both take it, so the file leaves the set; a read that no
/// longer lists it keeps it in the set and the diff area says so. With nothing held a found row
/// is written into the list on screen.
pub(super) fn adopt_row(view: &mut OpenView, row: DiffRow) {
    if view.source != Source::Changes {
        return;
    }
    let Some(path) = view.selected.clone() else {
        return;
    };
    let Some(shown) = shown_list_mut(&mut view.list) else {
        return;
    };
    let files = match row {
        DiffRow::NotRead => return,
        DiffRow::Unlisted if view.held.is_none() => return,
        DiffRow::Unlisted => Vec::new(),
        DiffRow::Row(row) => vec![row],
    };
    let fresh = ChangeList {
        files,
        scope: shown.scope.clone(),
    };
    match view.held.take() {
        // A read the user's selection started: one that no longer lists the file says so.
        Some(held) => {
            view.held = held.adopt(shown, &path, &fresh);
            note_selection(view);
        }
        None => {
            shown.put(&path, &fresh);
            if let Some(ChangeKind::Renamed { from }) = fresh.files.first().map(|r| &r.kind) {
                shown.put(from, &fresh);
            }
        }
    }
}

/// The shown pull request's diff, as the hold compares it.
fn pr_diff_of(pr: &PullRequestView) -> Option<PrDiff> {
    let PrDiffLoad::Ready { files, patches, .. } = &pr.diff else {
        return None;
    };
    Some(PrDiff {
        files: files
            .iter()
            .map(|file| PrFile {
                file: file.clone(),
                patch: patches.get(&file.path).cloned().unwrap_or_default(),
            })
            .collect(),
    })
}

/// A head refresh of the shown pull request answered with `diff` (research R6). Returns the diff
/// to show now with its head: no rows on screen, or nothing differs; `None` when it is held.
pub(super) fn hold_pr_refresh(
    pr: &mut PullRequestView,
    diff: PrDiff,
    head: String,
) -> Option<(PrDiff, String)> {
    let Some(shown) = pr_diff_of(pr).filter(|shown| !shown.files.is_empty()) else {
        pr.held = None;
        return Some((diff, head));
    };
    let observed = match pr.held.take() {
        Some(HeldPr { held, .. }) => held.observe(&shown, diff),
        None => observe(&shown, diff),
    };
    match observed {
        Observed::Same(latest) => Some((latest, head)),
        Observed::Held(held) => {
            pr.held = Some(HeldPr { held, head });
            None
        }
    }
}

/// Show `diff`, read at `head`, as the shown pull request's: the selected file is kept while
/// listed (FR-010), else the diff pane is reset as a refresh does.
pub(super) fn show_pr_diff(view: &mut OpenView, diff: PrDiff, head: String) {
    let Source::PullRequest(pr) = &mut view.source else {
        return;
    };
    pr.diff = pr_diff_ready(diff);
    pr.head = head;
    pr.adopted.clear();
    let dropped = pr
        .selected
        .as_ref()
        .is_some_and(|path| !super::pr_lists(pr, path));
    if dropped {
        pr.selected = None;
    }
    pr.file = pr_file_diff(pr);
    if dropped {
        reset_diff_pane(view);
    }
}

/// Selecting `path` of a held pull request (FR-009): a file in the set shows its latest patch
/// and row and leaves the set; one the latest diff no longer lists stays in it.
pub(super) fn adopt_pr_selection(pr: &mut PullRequestView, path: &RelPath) {
    if !pr.held.as_ref().is_some_and(|h| h.held.contains(path)) {
        return;
    }
    let (Some(mut shown), Some(HeldPr { held, head })) = (pr_diff_of(pr), pr.held.take()) else {
        return;
    };
    // The whole latest diff, so a file new to the shown one takes its place in GitHub's order.
    let latest = held.latest().clone();
    let listed = latest.files.iter().any(|f| &f.file.path == path);
    pr.held = match held.adopt(&mut shown, path, &latest) {
        Some(held) => {
            if listed {
                pr.adopted.insert(path.clone(), head.clone());
            }
            Some(HeldPr { held, head })
        }
        None => {
            // Nothing left held: what is shown is the latest diff, read at its head.
            pr.head = head;
            pr.adopted.clear();
            None
        }
    };
    pr.diff = pr_diff_ready(shown);
}
