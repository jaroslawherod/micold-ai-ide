//! The Review tab's keyboard (#1274, plan D4–D5): who has the keyboard ([`review_holder`]), the
//! decision of a press ([`key_pressed`]) and what each review key does to the shown view
//! ([`review_key`], contract K7). The keys and the rules they follow are core's
//! (`micold_core::review::{keys, navigate}`); this module only feeds them the view's state.

use micold_core::navigation::Dir;
use micold_core::quick_switch::Candidate;
use micold_core::review::comment::{CommentId, ReviewComment};
use micold_core::review::diff::header_rows;
use micold_core::review::drawer::{DrawerEvent, DrawerTab};
use micold_core::review::jump::{review_candidates, ReviewEntry, ReviewJump};
use micold_core::review::keys::{
    key_hint, resolve, sheet_rows, Holder, Press, ReviewAction, ReviewKey, SINGLE_KEYS_OFF_NOTE,
};
use micold_core::review::marks::ReviewState;
use micold_core::review::navigate::{
    comment_move, comment_order, comment_target, has_hunks, next_unreviewed, resolve_edge,
    step_comment, step_file, step_hunk, CommentFact, CommentMove, CommentOpen, HunkMove, NavFile,
    SelectedDiff, Show, Unreviewed, ALL_FILES_REVIEWED, PICK_OR_HUNK,
};
use micold_core::review::tree;
use micold_core::review::RelPath;
use micold_core::settings::DiffLayout;
use micold_core::typeahead::Query;

use super::{
    change_leaves, diff_body, file_reviews, in_scope, is_outdated, jump_offset, jump_target,
    list_body, mark_toggled, pick_allowed, review_of, reviewed_paths, selected_file, sheet_on_top,
    shown_hunks, shown_range, update, view_scope, DiffBody, Effect, Jump, ListBody, Load, Msg,
    OpenView, Pick, Source, State,
};
use crate::features::window::FieldId;

/// Who has the keyboard for the review keys (#1274 research R7, FR-002, SC-002): `Review` while
/// the shown entry's Review tab is on screen and nothing else holds the keyboard; `Composer` while
/// its comment box has it; `Sheet` while the shortcut sheet is on top; `Other` for a focused
/// field, Settings, the Overview, the quick switcher or any other registered surface on top.
pub fn review_holder(app: &crate::app::State) -> Holder {
    if app.review_shown().is_none()
        || app.settings.settings_draft.is_some()
        || app.overview.open
        || app.terminal_focused()
    {
        return Holder::Other;
    }
    if crate::overlay::registry::topmost(app).is_some() {
        return if sheet_on_top(app) {
            Holder::Sheet
        } else {
            Holder::Other
        };
    }
    match app.live_focused_field() {
        None => Holder::Review,
        Some(FieldId::ReviewComposer) => Holder::Composer,
        Some(_) => Holder::Other,
    }
}

/// A press the review's listener heard (contract K6), decided at the press: by who has the keyboard
/// now and the window's single-key switch (FR-022). `None` leaves it to whatever has the keyboard.
pub fn key_pressed(app: &crate::app::State, press: &Press) -> Option<Msg> {
    resolve(press, review_holder(app), app.settings.review_single_keys).map(Msg::ReviewKey)
}

/// `first`, then `then` unless it is nothing.
pub(super) fn with(first: Effect, then: Effect) -> Effect {
    match (first, then) {
        (first, Effect::None) => first,
        (Effect::None, then) => then,
        (Effect::Batch(mut list), then) => {
            list.push(then);
            Effect::Batch(list)
        }
        (first, then) => Effect::Batch(vec![first, then]),
    }
}

/// What a review key does to the shown view (contract K7).
pub(super) fn review_key(state: &mut State, action: ReviewAction) -> Effect {
    if state.tabs.shown_view().is_none() {
        return Effect::None;
    }
    match action {
        ReviewAction::NextFile => step_files(state, Dir::Next),
        ReviewAction::PrevFile => step_files(state, Dir::Previous),
        ReviewAction::NextHunk => step_hunks(state, Dir::Next),
        ReviewAction::PrevHunk => step_hunks(state, Dir::Previous),
        ReviewAction::ToggleReviewed => mark_toggled(state),
        ReviewAction::NextUnreviewed => to_unreviewed(state),
        ReviewAction::NextComment => step_comments(state, Dir::Next),
        ReviewAction::PrevComment => step_comments(state, Dir::Previous),
        ReviewAction::Comment => comment(state),
        ReviewAction::ToggleSheet => {
            if let Some(view) = state.tabs.shown_view_mut() {
                view.sheet_open = !view.sheet_open;
            }
            Effect::None
        }
        // FR-010: exactly **Send review (n)**, which does nothing while it is unavailable.
        ReviewAction::Send => update(state, Msg::SendPressed),
    }
}

/// The shown source's files in the tree's order (its sort and filter, folds ignored: FR-003), with
/// whether each has hunks `n`/`p` stop on.
fn nav_order(state: &State, view: &OpenView) -> Vec<NavFile> {
    let ListBody::Files(files) = list_body(view) else {
        return Vec::new();
    };
    let leaves = change_leaves(files);
    let reviewed = reviewed_paths(state, view);
    let is_reviewed = |path: &str| reviewed.contains(path);
    let query = Query::new(&view.tree.filter);
    let large = match &view.source {
        Source::Changes => &view.shown_large,
        Source::PullRequest(pr) => &pr.shown_large,
    };
    tree::file_order(&leaves, &view.tree, &query, &is_reviewed)
        .into_iter()
        .filter_map(|leaf| files.get(leaf))
        .map(|file| NavFile {
            path: file.path.clone(),
            has_hunks: has_hunks(file, large.contains(&file.path)),
        })
        .collect()
}

/// `j` / `k` (FR-003).
fn step_files(state: &mut State, dir: Dir) -> Effect {
    let Some(view) = state.tabs.shown_view() else {
        return Effect::None;
    };
    let order = nav_order(state, view);
    match step_file(&order, selected_file(view), dir) {
        Some(path) => select(state, path),
        None => Effect::None,
    }
}

/// Select `path` as a tree click does (the one select path, `Msg::FileSelected`: its row revealed,
/// the current hunk cleared), show its diff from the top and bring its row into view (FR-003).
fn select(state: &mut State, path: RelPath) -> Effect {
    let selected = update(state, Msg::FileSelected(path.clone()));
    let Some(view) = state.tabs.shown_view_mut() else {
        return selected;
    };
    // Shown from the top: no hunk is current, even when the file was selected already.
    view.diff_offset = 0;
    view.current_hunk = None;
    let mut effect = with(selected, Effect::ScrollDiff { offset: 0 });
    if let Some(offset) = row_into_view(view, &path) {
        view.list_offset = offset;
        effect = with(effect, Effect::ScrollList { offset });
    }
    effect
}

/// The tree offset that brings `path`'s row into view, `None` when it is in view already.
fn row_into_view(view: &OpenView, path: &RelPath) -> Option<u32> {
    let files = match list_body(view) {
        ListBody::Files(files) => files,
        _ => return None,
    };
    let index = view.change_rows.iter().position(|row| {
        matches!(row.kind, tree::RowKind::File { leaf } if files.get(leaf).is_some_and(|f| &f.path == path))
    })?;
    let height = crate::ui::changes::ROW_HEIGHT;
    let top = (index as f32 * height) as u32;
    let bottom = ((index + 1) as f32 * height) as u32;
    let (from, viewport) = (view.list_offset, view.list_viewport);
    if viewport == 0 || top < from {
        (top != from).then_some(top)
    } else if bottom > from + viewport {
        Some(bottom - viewport)
    } else {
        None
    }
}

/// The selected file's diff as `step_hunk` reads it.
fn selected_diff(view: &OpenView) -> SelectedDiff {
    match (&view.source, &view.diff) {
        (Source::PullRequest(_), _) | (Source::Changes, Load::Ready(_)) => {
            SelectedDiff::Hunks(shown_hunks(view).len())
        }
        (Source::Changes, Load::Failed(_)) => SelectedDiff::Failed,
        (Source::Changes, Load::Idle | Load::Loading { .. }) => SelectedDiff::Loading,
    }
}

/// `n` / `p` (FR-004).
fn step_hunks(state: &mut State, dir: Dir) -> Effect {
    let Some(view) = state.tabs.shown_view() else {
        return Effect::None;
    };
    let order = nav_order(state, view);
    let step = step_hunk(
        &order,
        selected_file(view),
        selected_diff(view),
        view.current_hunk,
        dir,
    );
    match step {
        HunkMove::Current(index) => {
            current(state, index).map_or(Effect::None, |offset| Effect::ScrollDiff { offset })
        }
        HunkMove::File { path, edge } => {
            let selected = select(state, path.clone());
            if let Some(view) = state.tabs.shown_view_mut() {
                view.pending_edge = Some((path, edge));
            }
            // A pull request's file is parsed at once: its edge is current now.
            match resolve_pending_edge(state) {
                Some(offset) => with(selected, Effect::ScrollDiff { offset }),
                None => selected,
            }
        }
        HunkMove::Nothing => Effect::None,
    }
}

/// Make hunk `index` of the selected file current and return the offset that puts its header at
/// the top of the diff (FR-004), with `jump_offset`'s arithmetic: rows above it, and the measured
/// cards and composers hung above its first line.
fn current(state: &mut State, index: usize) -> Option<u32> {
    let layout = state.layout;
    let view = state.tabs.shown_view_mut()?;
    let row = *header_rows(&view.shaped, layout).get(index)?;
    let hunk = shown_hunks(view).get(index)?;
    let (old, new) = (hunk.old_start, hunk.new_start);
    let hung: u32 = view
        .slot_heights
        .iter()
        .filter(|((side, line), _)| match side {
            micold_core::review::Side::Old => *line < old,
            micold_core::review::Side::New => *line < new,
        })
        .map(|(_, height)| *height)
        .sum();
    let offset = (row as f32 * crate::ui::material::DIFF_ROW_HEIGHT) as u32 + hung;
    view.current_hunk = Some(index);
    view.diff_offset = offset;
    Some(offset)
}

/// The pending edge of the selected file, once its diff is shown: its hunk is made current and the
/// diff scrolled to it (research R5). A file whose diff has no hunk drops the edge.
pub(super) fn resolve_pending_edge(state: &mut State) -> Option<u32> {
    let view = state.tabs.shown_view_mut()?;
    let (path, edge) = view.pending_edge.clone()?;
    if selected_file(view) != Some(&path) {
        return None;
    }
    let SelectedDiff::Hunks(count) = selected_diff(view) else {
        return None;
    };
    view.pending_edge = None;
    current(state, resolve_edge(edge, count)?)
}

/// What `]` and `[` know of the shown scope's comments (FR-006): each at its shown first line, in
/// the diff when a press on its drawer row would jump to it there and it is not outdated.
fn comment_facts(state: &State, view: &OpenView) -> Vec<CommentFact> {
    let Some(review) = review_of(state, view) else {
        return Vec::new();
    };
    in_scope(review, view_scope(view))
        .map(|c| CommentFact {
            id: c.id,
            path: c.path.clone(),
            line: shown_range(view, c).start(),
            created: c.created,
            in_diff: matches!(jump_target(state, c.id), Some(Jump::Shown { .. }))
                && !is_outdated(view, c),
        })
        .collect()
}

/// `]` / `[` (FR-006): the next or previous comment in comment order, its file selected first when
/// it is another; a card in the diff is reached by the drawer row's jump, any other comment by the
/// drawer opened on it.
fn step_comments(state: &mut State, dir: Dir) -> Effect {
    reach_comment(state, |ordered, order, selected, last| {
        step_comment(ordered, order, selected, last, dir)
    })
}

/// Reach the comment `pick` chooses from the shown scope's comments in comment order, the shown
/// files' order, the selected file and the last comment reached, as `]` does (FR-006).
fn reach_comment(
    state: &mut State,
    pick: impl FnOnce(
        &[CommentFact],
        &[NavFile],
        Option<&RelPath>,
        Option<CommentId>,
    ) -> Option<CommentMove>,
) -> Effect {
    let Some(view) = state.tabs.shown_view() else {
        return Effect::None;
    };
    let order = nav_order(state, view);
    let ordered = comment_order(&comment_facts(state, view), &order);
    let Some(step) = pick(&ordered, &order, selected_file(view), view.last_comment) else {
        return Effect::None;
    };
    let tab = review_of(state, view)
        .and_then(|r| r.comments.iter().find(|c| c.id == step.id))
        .map(|c| DrawerTab::of(c.state.status()));
    let selected = match step.select {
        Some(path) => select(state, path),
        None => Effect::None,
    };
    let Some(view) = state.tabs.shown_view_mut() else {
        return selected;
    };
    view.last_comment = Some(step.id);
    view.drawer.hidden = None;
    view.drawer.focus = Some(step.id);
    match step.show {
        Show::Card => {
            view.drawer.to_show = true;
            match jump_offset(state) {
                Some(offset) => with(selected, Effect::ScrollDiff { offset }),
                None => selected,
            }
        }
        Show::Drawer => {
            view.drawer.to_show = false;
            if !view.drawer.state.open {
                view.drawer.state.apply(DrawerEvent::Toggle);
            }
            if let Some(tab) = tab {
                view.drawer.state.apply(DrawerEvent::SelectTab(tab));
            }
            selected
        }
    }
}

/// What a chosen review entry does to the shown view (contract J5): nothing for a file or comment
/// the view no longer lists.
pub(super) fn jumped(state: &mut State, jump: ReviewJump) -> Effect {
    match jump {
        ReviewJump::File(path) => {
            let Some(view) = state.tabs.shown_view() else {
                return Effect::None;
            };
            if nav_order(state, view).iter().any(|file| file.path == path) {
                select(state, path)
            } else {
                Effect::None
            }
        }
        ReviewJump::Comment(id) => reach_comment(state, |ordered, order, selected, _| {
            comment_move(ordered, order, selected, id)
        }),
    }
}

/// The jump-to field's review entries for `app`'s current entry (#1274 FR-016, contract J1, J4):
/// the files of its Review tab, shown or hidden, in the order `j` steps through them, then the
/// comments of its shown scope; nothing when the current entry has no Review tab (FR-019).
pub fn jump_entries(app: &crate::app::State) -> Vec<(Candidate, ReviewEntry)> {
    let Some(entry) = app.shown_location() else {
        return Vec::new();
    };
    // No active project reads as the empty path, as `review_shown` does.
    let project = app.workspace.active.clone().unwrap_or_default();
    let Some(tab) = app.changes.tabs.get(&(project, entry)) else {
        return Vec::new();
    };
    let (state, view) = (&app.changes, &tab.view);
    let files: Vec<RelPath> = nav_order(state, view)
        .into_iter()
        .map(|file| file.path)
        .collect();
    let reviews = file_reviews(state, view, true);
    let comments: Vec<ReviewComment> = review_of(state, view)
        // At their shown first line, as `]` orders them (FR-006).
        .map(|review| {
            in_scope(review, view_scope(view))
                .map(|comment| ReviewComment {
                    range: shown_range(view, comment),
                    ..comment.clone()
                })
                .collect()
        })
        .unwrap_or_default();
    review_candidates(
        &files,
        |path| {
            reviews
                .get(path)
                .is_some_and(|review| review.state == ReviewState::Reviewed)
        },
        &comments,
    )
}

/// `c` (FR-009): focus the open composer, else open **Add comment**'s on the picked lines, else
/// pick the current hunk's changed lines and open it there, else say what to do.
fn comment(state: &mut State) -> Effect {
    let Some(view) = state.tabs.shown_view() else {
        return Effect::None;
    };
    let current = view.current_hunk.and_then(|i| shown_hunks(view).get(i));
    let target = comment_target(view.composer.is_some(), view.pick.is_some(), current);
    let target = match target {
        CommentOpen::Hunk { .. } if !pick_allowed(view) => CommentOpen::Refused,
        target => target,
    };
    let Some(view) = state.tabs.shown_view_mut() else {
        return Effect::None;
    };
    match target {
        CommentOpen::FocusComposer => Effect::FocusComposer,
        CommentOpen::Picked => with(update(state, Msg::AddComment), Effect::FocusComposer),
        CommentOpen::Hunk { side, range } => {
            view.pick = Some(Pick {
                side,
                anchor: range.start(),
                head: range.end(),
            });
            with(update(state, Msg::AddComment), Effect::FocusComposer)
        }
        CommentOpen::Refused => {
            view.notice = Some(PICK_OR_HUNK);
            Effect::None
        }
    }
}

/// `g` (FR-008): the next file not marked reviewed, read now, so a mark made in another window
/// counts (US2 s5).
fn to_unreviewed(state: &mut State) -> Effect {
    let Some(view) = state.tabs.shown_view() else {
        return Effect::None;
    };
    let order = nav_order(state, view);
    let reviews = file_reviews(state, view, true);
    let is_reviewed = |path: &RelPath| {
        reviews
            .get(path)
            .is_some_and(|review| review.state == ReviewState::Reviewed)
    };
    match next_unreviewed(&order, selected_file(view), is_reviewed) {
        Unreviewed::Select(path) => select(state, path),
        Unreviewed::AllReviewed => {
            if let Some(view) = state.tabs.shown_view_mut() {
                view.notice = Some(ALL_FILES_REVIEWED);
            }
            Effect::None
        }
        Unreviewed::Nothing => Effect::None,
    }
}

/// The press the comment composer's text area offers the review (contract K6, US4 s4): the send
/// chord sends the review as **Send review (n)** does; every other press stays the text area's.
pub fn composer_press(press: &Press, single_keys: bool) -> Option<Msg> {
    match resolve(press, Holder::Composer, single_keys)? {
        ReviewAction::Send => Some(Msg::SendPressed),
        _ => None,
    }
}

/// What the shortcut sheet lists (FR-012, FR-022, contract K4a): each row's key in the running
/// platform's form with its text, in table order, and the note shown while the single keys are off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetModel {
    /// `(key, what it does)`, in [`micold_core::review::keys::REVIEW_KEYS`]' order, then Back and
    /// Forward.
    pub rows: Vec<(String, &'static str)>,
    /// [`micold_core::review::keys::SINGLE_KEYS_OFF_NOTE`] while the single-key switch is off.
    pub note: Option<&'static str>,
}

/// The shortcut sheet's content while it is open, for the window's single-key switch.
pub fn shortcut_sheet(app: &crate::app::State) -> Option<SheetModel> {
    app.changes
        .tabs
        .shown_view()
        .filter(|view| view.sheet_open)?;
    Some(sheet_model(app.settings.review_single_keys))
}

/// The sheet's content with the single-key switch at `single_keys` (contract K4a).
pub fn sheet_model(single_keys: bool) -> SheetModel {
    SheetModel {
        rows: sheet_rows(single_keys)
            .map(|row| (platform_label(row), row.sheet))
            .chain(history_rows())
            .collect(),
        note: (!single_keys).then_some(SINGLE_KEYS_OFF_NOTE),
    }
}

/// Back and Forward, which end the sheet whether or not the single keys are on (FR-014, U6).
fn history_rows() -> [(String, &'static str); 2] {
    use micold_core::history::Dir;
    [
        (
            crate::keymap::history_key_label(Dir::Back),
            "Back (also the mouse back button)",
        ),
        (
            crate::keymap::history_key_label(Dir::Forward),
            "Forward (also the mouse forward button)",
        ),
    ]
}

/// `row`'s key in the running platform's form (contract K4).
fn platform_label(row: &ReviewKey) -> String {
    let (other, mac) = row.labels();
    if cfg!(target_os = "macos") {
        mac
    } else {
        other
    }
}

/// The tooltip of **Send review (n)** (FR-013), naming the send chord.
pub const SEND_TOOLTIP: &str = "Send the draft comments";

/// The tooltip of the previous-hunk control in the diff header (FR-013).
pub const PREVIOUS_HUNK: &str = "Previous change";
/// The tooltip of the next-hunk control in the diff header (FR-013).
pub const NEXT_HUNK: &str = "Next change";

/// `text` naming `action`'s key in the platform's form (FR-013, contract K4a): nothing added while
/// the single-key switch is off, so a tooltip never names a key that does nothing.
pub fn with_key_hint(text: &str, action: ReviewAction, single_keys: bool) -> String {
    match key_hint(action, single_keys) {
        Some(row) => format!("{text} ({})", platform_label(row)),
        None => text.to_owned(),
    }
}

/// The diff row of the current hunk's header in `layout`, which the diff marks (FR-005).
pub fn current_header_row(view: &OpenView, layout: DiffLayout) -> Option<usize> {
    let index = view.current_hunk?;
    header_rows(&view.shaped, layout).get(index).copied()
}

/// Whether the diff header shows the previous / next hunk controls: a diff with hunks is shown.
pub fn hunk_controls(view: &OpenView) -> bool {
    matches!(diff_body(view), DiffBody::Diff(_)) && !shown_hunks(view).is_empty()
}
