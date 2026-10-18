//! Leaving a required field marks it, on the draft that owns it and nowhere else (feature 937,
//! FR-006, research R3).
//!
//! Driven through the root, which is the one place a blur reaches a draft's marks (`app::field_blurred`). What it holds to:
//! a fresh form shows nothing, a blur marks only the field left, a blur of a field whose draft is
//! not open changes nothing, and one window's form never marks another's (Principle II).

use micold_client::app::{Message, State};
use micold_client::features::window::FieldId;
use micold_client::features::window::Msg as WindowMsg;
use micold_client::features::worktree_form::Msg as FormMsg;
use micold_core::required::FieldRequirement::{self, Missing, Required};

/// The user leaves `id`, as the view reports it.
mod required {
    use super::*;
    pub fn blurred(state: &mut State, id: FieldId) {
        state.update(Message::Window(WindowMsg::FieldFocusChanged(id, false)));
    }
}

fn opened() -> State {
    let mut state = State::default();
    state.update(Message::WorktreeForm(FormMsg::Opened));
    state
}

fn reads(state: &State, field: FieldId) -> FieldRequirement {
    let form = state.worktree_form.form.as_ref().expect("the form is open");
    form.required.state_in(&form.required_fields(), field)
}

#[test]
fn a_fresh_form_shows_no_required_error() {
    let state = opened();
    assert_eq!(reads(&state, FieldId::AddWorktreeName), Required);
    assert_eq!(reads(&state, FieldId::AddWorktreeType), Required);
}

#[test]
fn leaving_a_field_marks_that_field_and_only_that_field() {
    let mut state = opened();

    required::blurred(&mut state, FieldId::AddWorktreeName);

    assert_eq!(reads(&state, FieldId::AddWorktreeName), Missing);
    assert_eq!(
        reads(&state, FieldId::AddWorktreeType),
        Required,
        "a field the user has not left shows nothing until a submit attempt"
    );
}

#[test]
fn leaving_a_field_that_holds_a_value_shows_nothing() {
    let mut state = opened();
    state.update(Message::WorktreeForm(FormMsg::NameChanged("login".into())));

    required::blurred(&mut state, FieldId::AddWorktreeName);

    assert_eq!(reads(&state, FieldId::AddWorktreeName), Required);
}

#[test]
fn a_blur_of_a_field_whose_draft_is_not_open_changes_nothing() {
    let mut state = opened();
    let before = state.worktree_form.form.clone();

    required::blurred(&mut state, FieldId::RunName);
    required::blurred(&mut state, FieldId::RenameProjectName);
    required::blurred(&mut state, FieldId::SettingsScrollback);

    assert_eq!(state.worktree_form.form, before);
    assert!(state.runs.dialog.is_none());
}

#[test]
fn a_blur_in_one_windows_form_leaves_another_windows_form_unmarked() {
    let mut a = opened();
    let b = opened();

    required::blurred(&mut a, FieldId::AddWorktreeName);

    assert_eq!(reads(&a, FieldId::AddWorktreeName), Missing);
    assert_eq!(reads(&b, FieldId::AddWorktreeName), Required);
}

#[test]
fn a_reopened_form_does_not_inherit_the_last_forms_marks() {
    let mut state = opened();
    required::blurred(&mut state, FieldId::AddWorktreeName);

    state.update(Message::WorktreeForm(FormMsg::Opened));

    assert_eq!(reads(&state, FieldId::AddWorktreeName), Required);
}

#[test]
fn leaving_one_daemons_repository_marks_that_daemon_only() {
    use micold_core::daemons::DaemonId;
    let mut state = State::default();
    let (a, b) = (DaemonId(1), DaemonId(2));

    required::blurred(&mut state, FieldId::DaemonRepository(a));

    let empty = micold_core::required::Requirement::text(true, "");
    let marks = &state.daemon_list.repository_marks;
    assert_eq!(marks.state(FieldId::DaemonRepository(a), empty), Missing);
    assert_eq!(marks.state(FieldId::DaemonRepository(b), empty), Required);
}

#[test]
fn leaving_the_rename_field_marks_the_open_rename_draft() {
    use micold_client::features::project::RenameDraft;
    let mut state = State::default();
    state.project.rename_draft = Some(RenameDraft {
        path: "/work/app".into(),
        text: String::new(),
        required: Default::default(),
    });

    required::blurred(&mut state, FieldId::RenameProjectName);

    let draft = state.project.rename_draft.as_ref().unwrap();
    assert_eq!(
        draft
            .required
            .state_in(&draft.required_fields(), FieldId::RenameProjectName),
        Missing
    );
}

#[test]
fn leaving_a_settings_field_marks_the_open_settings_draft_only() {
    use micold_client::features::settings::SettingsDraft;
    let mut state = State::default();
    state.settings.settings_draft = Some(SettingsDraft::default());
    state
        .settings
        .settings_draft
        .as_mut()
        .unwrap()
        .terminal
        .scrollback_lines = String::new();

    required::blurred(&mut state, FieldId::SettingsScrollback);

    let draft = state.settings.settings_draft.as_ref().unwrap();
    let fields =
        draft.required_fields(micold_client::features::settings::SettingsSection::Terminal);
    assert_eq!(
        draft
            .required
            .state_in(&fields, FieldId::SettingsScrollback),
        Missing
    );
}

#[test]
fn reopening_settings_shows_no_stale_repository_error() {
    use micold_client::features::settings::Msg as SettingsMsg;
    use micold_core::daemons::DaemonId;
    let mut state = State::default();
    let a = DaemonId(1);
    required::blurred(&mut state, FieldId::DaemonRepository(a));
    let empty = micold_core::required::Requirement::text(true, "");
    assert_eq!(
        state
            .daemon_list
            .repository_marks
            .state(FieldId::DaemonRepository(a), empty),
        Missing
    );

    state.update(Message::Settings(SettingsMsg::Opened));

    assert_eq!(
        state
            .daemon_list
            .repository_marks
            .state(FieldId::DaemonRepository(a), empty),
        Required
    );
}
