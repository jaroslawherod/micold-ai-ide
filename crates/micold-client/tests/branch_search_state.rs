//! Searching the branch list, as state transitions (feature 021, data-model §2).
//!
//! Everything the picker decides lives in the reducer, so all of it is reachable from here: what
//! the results are after a keystroke, where the keyboard highlight lands when the list changes
//! under it, what survives a query being cleared, and what a blocked branch is allowed to do. None
//! of that is glue, so none of it may live where only a person clicking around could find it out.

use micold_client::app::{BranchSource, Message, State, WorktreeForm};
use micold_core::naming::ConventionalType;
use micold_core::typeahead::Direction;
use micold_core::worktree::{BlockReason, BranchCandidate, BranchOrigin};

// --- fixtures -------------------------------------------------------------------------------

fn candidate(name: &str) -> BranchCandidate {
    BranchCandidate {
        name: name.to_string(),
        origin: BranchOrigin::Local,
        blocked_by: None,
    }
}

fn blocked(name: &str) -> BranchCandidate {
    BranchCandidate {
        name: name.to_string(),
        origin: BranchOrigin::Local,
        blocked_by: Some(BlockReason::CheckedOutInProjectRoot),
    }
}

/// The candidates every test below searches: two that share `log`, one that does not, and one
/// held elsewhere.
fn branches() -> Vec<BranchCandidate> {
    vec![
        candidate("feat/login"),
        candidate("chore/deps"),
        candidate("feat/logout"),
        blocked("main"),
    ]
}

/// An open form on the existing-branch source, with the candidates listed.
fn picker() -> State {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeTypeSelected(ConventionalType::Feat));
    state.update(Message::AddWorktreeNameChanged("login".to_string()));
    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));
    state.update(Message::AddWorktreeBranchesListed(branches()));
    state
}

fn form(state: &State) -> &WorktreeForm {
    state.worktree_form.as_ref().unwrap()
}

/// The branch names currently on offer, in the order they are offered.
fn results(state: &State) -> Vec<&str> {
    let f = form(state);
    f.branch_matches
        .iter()
        .map(|(i, _)| f.candidates[*i].name.as_str())
        .collect()
}

fn type_(state: &mut State, text: &str) {
    state.update(Message::AddWorktreeBranchQueryChanged(text.to_string()));
}

// --- T014: query, results, highlight, selection ----------------------------------------------

/// FR-002 — before anything is typed, the picker offers what it has always offered, in the order it
/// has always offered it. The whole feature is a narrowing of this baseline.
#[test]
fn with_no_query_every_candidate_is_offered_in_its_original_order() {
    let state = picker();
    assert_eq!(
        results(&state),
        vec!["feat/login", "chore/deps", "feat/logout", "main"]
    );
}

/// FR-005 — typing narrows, and the results always describe the text currently in the field.
#[test]
fn typing_narrows_the_results_to_what_matches() {
    let mut state = picker();
    type_(&mut state, "log");

    assert_eq!(results(&state), vec!["feat/login", "feat/logout"]);
    assert_eq!(form(&state).branch_query, "log");
}

/// FR-016 — clearing the field is one action and puts everything back.
#[test]
fn clearing_the_query_restores_the_full_list() {
    let mut state = picker();
    type_(&mut state, "log");
    type_(&mut state, "");

    assert_eq!(
        results(&state),
        vec!["feat/login", "chore/deps", "feat/logout", "main"]
    );
}

/// The results are derived, never edited in place: a later listing re-runs the current query
/// rather than leaving stale rows behind.
#[test]
fn a_new_listing_is_matched_against_the_current_query() {
    let mut state = picker();
    type_(&mut state, "log");

    state.update(Message::AddWorktreeBranchesListed(vec![
        candidate("feat/logging"),
        candidate("chore/deps"),
    ]));

    assert_eq!(results(&state), vec!["feat/logging"]);
}

/// Invariant 3 — the highlight may never point at a row that is no longer there. A keystroke that
/// shortens the list has to re-seat it, or the next Enter picks something the developer cannot see.
#[test]
fn the_highlight_never_dangles_when_the_results_shrink() {
    let mut state = picker();
    // The first press enters the list at row 0, so four presses reach the last of four rows —
    // `main`, which the query below filters away.
    for _ in 0..4 {
        state.update(Message::AddWorktreeBranchHighlightMoved(Direction::Next));
    }
    assert_eq!(form(&state).branch_highlight, Some(3), "on the last row");

    type_(&mut state, "log");

    let f = form(&state);
    assert_eq!(f.branch_matches.len(), 2);
    match f.branch_highlight {
        Some(i) => assert!(i < 2, "highlight {i} points past the {} rows left", f.branch_matches.len()),
        None => {}
    }
}

/// Moving saturates rather than wrapping — the reducer applies the rule `micold-core` decides.
#[test]
fn moving_the_highlight_stops_at_both_ends() {
    let mut state = picker();
    for _ in 0..10 {
        state.update(Message::AddWorktreeBranchHighlightMoved(Direction::Next));
    }
    assert_eq!(form(&state).branch_highlight, Some(3), "four rows, so the last is index 3");

    for _ in 0..10 {
        state.update(Message::AddWorktreeBranchHighlightMoved(Direction::Prev));
    }
    assert_eq!(form(&state).branch_highlight, Some(0));
}

/// FR-014 — a made choice survives every later keystroke, including one that hides it.
#[test]
fn the_selection_survives_the_query_changing_and_being_cleared() {
    let mut state = picker();
    type_(&mut state, "log");
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));
    assert_eq!(form(&state).selected_branch.as_ref().unwrap().name, "feat/login");

    type_(&mut state, "zzq");
    assert!(form(&state).branch_matches.is_empty(), "nothing matches now");
    assert_eq!(
        form(&state).selected_branch.as_ref().unwrap().name,
        "feat/login",
        "narrowing the list must not silently unmake a choice"
    );

    type_(&mut state, "");
    assert_eq!(form(&state).selected_branch.as_ref().unwrap().name, "feat/login");
}

/// FR-014a — the field holds the search text and nothing else. Picking a branch must not write its
/// name into the query, or clearing the field would have to mean two different things at once.
#[test]
fn picking_a_branch_leaves_the_search_text_alone() {
    let mut state = picker();
    type_(&mut state, "log");
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));

    assert_eq!(form(&state).branch_query, "log");
}

/// Leaving the picker takes its search state with it, so returning does not resume someone else's
/// half-finished search.
#[test]
fn switching_away_from_the_picker_resets_the_search() {
    let mut state = picker();
    type_(&mut state, "log");
    state.update(Message::AddWorktreeBranchHighlightMoved(Direction::Next));

    state.update(Message::AddWorktreeSourceChanged(BranchSource::New));

    let f = form(&state);
    assert!(f.branch_query.is_empty());
    assert!(f.branch_highlight.is_none());
    assert!(!f.branch_list_open);
}

// --- T015: focus opens the list ---------------------------------------------------------------

/// FR-001b — focusing shows what is on offer, before anything is typed. Without this the picker
/// opens as a bare field with no sign that it has anything in it.
#[test]
fn focusing_the_field_opens_the_list_without_filtering_anything() {
    let mut state = picker();
    assert!(!form(&state).branch_list_open, "closed until the field is focused");

    state.update(Message::AddWorktreeBranchFocused);

    let f = form(&state);
    assert!(f.branch_list_open);
    assert!(f.branch_query.is_empty(), "focusing is not typing");
    assert_eq!(f.branch_matches.len(), 4, "everything is still on offer");
    assert!(f.selected_branch.is_none());
}

/// Typing opens the list too — a developer who types into an unfocused-then-focused field should
/// never be left looking at results that are not shown.
#[test]
fn typing_opens_the_list() {
    let mut state = picker();
    type_(&mut state, "log");
    assert!(form(&state).branch_list_open);
}

/// FR-001b — dismissal closes the list and changes nothing else. Escape, a click outside and the
/// field losing focus are three triggers for this one effect.
#[test]
fn dismissing_closes_the_list_and_touches_nothing_else() {
    let mut state = picker();
    type_(&mut state, "log");
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));

    state.update(Message::AddWorktreeBranchDismissed);

    let f = form(&state);
    assert!(!f.branch_list_open);
    assert_eq!(f.branch_query, "log");
    assert_eq!(f.branch_matches.len(), 2);
    assert_eq!(f.selected_branch.as_ref().unwrap().name, "feat/login");
}

/// Picking closes the list — the developer has answered the question it was asking.
#[test]
fn picking_closes_the_list() {
    let mut state = picker();
    state.update(Message::AddWorktreeBranchFocused);
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));

    assert!(!form(&state).branch_list_open);
}

/// Invariant 6 — an open list with no matches is a real state, and the one that shows the no-match
/// message. Inferring "open" from "has rows" would make that message unreachable.
#[test]
fn a_query_matching_nothing_leaves_the_list_open_and_the_text_editable() {
    let mut state = picker();
    type_(&mut state, "zzq");

    let f = form(&state);
    assert!(f.branch_list_open, "the list stays open to say that nothing matched");
    assert!(f.branch_matches.is_empty());
    assert_eq!(f.branch_query, "zzq", "the text stays put so it can be corrected");
}

// --- T016: a blocked branch cannot be chosen --------------------------------------------------

/// FR-012 — searching never hides a branch. One that is checked out elsewhere still appears, with
/// whatever explains it.
#[test]
fn a_blocked_branch_is_still_listed_when_it_matches() {
    let mut state = picker();
    type_(&mut state, "main");

    assert_eq!(results(&state), vec!["main"]);
}

/// FR-012a — but it cannot be picked. The refusal used to happen at the point of creating, because
/// the old list widget could not disable a row; it now happens at the point of choosing.
#[test]
fn picking_a_blocked_branch_does_nothing_at_all() {
    let mut state = picker();
    state.update(Message::AddWorktreeBranchFocused);

    state.update(Message::AddWorktreeBranchSelected(blocked("main")));

    let f = form(&state);
    assert!(f.selected_branch.is_none(), "a blocked branch must not become the selection");
    assert!(f.branch_list_open, "and the list must not close on a press that did nothing");
}

/// An earlier, legitimate choice is not disturbed by pressing an unavailable row.
#[test]
fn a_blocked_press_does_not_disturb_an_existing_selection() {
    let mut state = picker();
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));

    state.update(Message::AddWorktreeBranchSelected(blocked("main")));

    assert_eq!(form(&state).selected_branch.as_ref().unwrap().name, "feat/login");
}

/// The guard at the point of action stays, unreachable through the picker but still the invariant's
/// last line of defence — and cheap enough that removing it would only trade a comparison for a
/// class of bug.
#[test]
fn the_submit_guard_still_refuses_a_blocked_selection() {
    let mut form = WorktreeForm {
        source: BranchSource::Existing,
        selected_branch: Some(blocked("main")),
        ..WorktreeForm::default()
    };
    assert!(!form.can_submit(), "the last line of defence must still hold");

    form.selected_branch = Some(candidate("feat/free"));
    assert!(form.can_submit());
}

// --- T017: the happy path is unchanged --------------------------------------------------------

/// FR-013 — picking an available branch from the search results does exactly what picking it from
/// the old list did. This is the assertion feature 016's rewritten tests would otherwise have taken
/// with them.
#[test]
fn picking_an_available_branch_from_the_results_behaves_as_it_always_did() {
    let mut state = picker();
    type_(&mut state, "log");

    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));

    let f = form(&state);
    assert_eq!(f.selected_branch.as_ref().unwrap().name, "feat/login");
    assert!(f.can_submit(), "an available branch is submittable");

    let derived = f.preview().expect("a selected branch previews");
    assert_eq!(derived.branch, "feat/login");
    assert_eq!(derived.dir_name, "feat-login");
}

/// Searching does not change which branch a selection derives its directory from — the preview is
/// read off the selection, not off the query.
#[test]
fn the_preview_follows_the_selection_not_the_search_text() {
    let mut state = picker();
    state.update(Message::AddWorktreeBranchSelected(candidate("release/v1.2")));
    type_(&mut state, "log");

    let derived = form(&state).preview().unwrap();
    assert_eq!(derived.branch, "release/v1.2");
    assert_eq!(derived.dir_name, "release-v1-2");
}

/// The way out of a no-match state is ordinary editing. The neighbouring test pins that the text
/// survives; this pins that shortening it brings the results back, which is what makes surviving
/// text useful rather than merely present.
#[test]
fn shortening_a_query_that_matched_nothing_brings_the_results_back() {
    let mut state = picker();
    type_(&mut state, "zzqxwv");
    assert!(form(&state).branch_matches.is_empty());

    type_(&mut state, "log");
    let f = form(&state);
    assert_eq!(f.branch_query, "log");
    assert_eq!(f.branch_matches.len(), 2, "both `log` branches are back");
    assert!(f.branch_list_open);
}

/// A no-match state does not disturb a selection already made. Typing nonsense is not a way to
/// unselect: only an explicit pick writes the selection (invariant 4).
#[test]
fn a_no_match_query_leaves_an_existing_selection_alone() {
    let mut state = picker();
    state.update(Message::AddWorktreeBranchSelected(candidate("feat/login")));
    type_(&mut state, "zzqxwv");

    let f = form(&state);
    assert!(f.branch_matches.is_empty());
    assert_eq!(f.selected_branch.as_ref().unwrap().name, "feat/login");
    assert!(f.can_submit(), "the form is still submittable on the branch already chosen");
}
