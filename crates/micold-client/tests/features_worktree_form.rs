//! The worktree-creation form, exercised in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module and the domain types its API mentions. It builds no
//! `State`, references no other feature's types, and needs no application shell — which is the
//! whole point: the form is the one feature whose intermediate state nothing else reads, and this
//! test is the executable form of that claim (research.md §5).
//!
//! If a later change makes this file need another feature's types to compile, that feature
//! boundary has eroded and the isolation SC-004 asserts is gone.

use micold_client::features::worktree_form::{
    BranchSource, ResolutionState, SessionChoice, WorktreeForm, WorktreeFormStatus,
};
use micold_core::naming::ConventionalType;
use micold_core::session::AiCli;

#[test]
fn a_fresh_form_is_editing_a_new_branch_and_cannot_be_submitted() {
    let form = WorktreeForm::default();

    assert_eq!(form.status, WorktreeFormStatus::Editing);
    assert_eq!(form.source, BranchSource::New);
    assert!(
        !form.can_submit(),
        "an empty form has no name, so there is nothing to create"
    );
}

#[test]
fn a_typed_name_makes_the_form_submittable() {
    let mut form = WorktreeForm {
        type_: Some(ConventionalType::Feat),
        ..WorktreeForm::default()
    };
    form.name = "add-a-thing".into();

    assert!(
        form.can_submit(),
        "a type and a name are what a create needs"
    );
}

#[test]
fn resolution_state_reports_whether_it_is_prompting() {
    let idle = ResolutionState::default();

    assert!(
        !idle.is_prompting(),
        "the default state prompts for nothing"
    );
    assert!(
        idle.situation().is_none(),
        "with nothing to resolve there is no situation"
    );
}

#[test]
fn a_create_in_flight_cannot_be_submitted_again() {
    let mut form = WorktreeForm {
        type_: Some(ConventionalType::Feat),
        ..WorktreeForm::default()
    };
    form.name = "add-a-thing".into();
    assert!(form.can_submit(), "precondition: this form is submittable");

    form.status = WorktreeFormStatus::Creating;

    assert!(
        !form.can_submit(),
        "a create already in flight must refuse a second submit, or one click could make two worktrees"
    );
}

#[test]
fn a_form_reports_no_stage_until_the_create_reaches_one() {
    let form = WorktreeForm::default();

    assert!(
        form.stage_label().is_none(),
        "stage is what the create has reached, not merely that one started"
    );
}

// --- Feature 776 (M4): the session is defined in the dialog -----------------------------------

#[test]
fn a_chosen_cli_and_a_prompt_become_the_definition_sent_with_the_create() {
    let form = WorktreeForm {
        session_cli: Some(AiCli::ClaudeCode),
        first_prompt: "fix the login".into(),
        ..WorktreeForm::default()
    };

    let definition = form.session_definition().expect("a CLI was chosen");
    assert_eq!(definition.ai_cli, AiCli::ClaudeCode);
    assert_eq!(definition.prompt.as_deref(), Some("fix the login"));
}

#[test]
fn a_blank_prompt_is_no_prompt() {
    let form = WorktreeForm {
        session_cli: Some(AiCli::ClaudeCode),
        first_prompt: "   ".into(),
        ..WorktreeForm::default()
    };

    assert_eq!(form.session_definition().unwrap().prompt, None);
}

#[test]
fn none_sends_no_session_and_a_prompt_with_none_is_not_representable() {
    let form = WorktreeForm {
        session_cli: None,
        first_prompt: "typed before choosing None".into(),
        ..WorktreeForm::default()
    };

    assert_eq!(form.session_definition(), None);
    assert_eq!(form.session_choice(), SessionChoice::NoSession);
}
