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
fn a_fresh_form_is_editing_a_new_branch_and_creates_nothing_yet() {
    let form = WorktreeForm::default();

    assert_eq!(form.status, WorktreeFormStatus::Editing);
    assert_eq!(form.source, BranchSource::New);
    assert!(
        form.preview().is_err(),
        "an empty form has no name, so there is nothing to create"
    );
    assert!(
        form.can_press(),
        "Create stays pressable: pressing it marks the empty required fields (FR-012)"
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
        form.can_press(),
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
    assert!(form.can_press(), "precondition: this form is submittable");

    form.status = WorktreeFormStatus::Creating;

    assert!(
        !form.can_press(),
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

// ---------------------------------------------------------------------------------------------
// Required fields (feature 937, FR-002, R5)
// ---------------------------------------------------------------------------------------------

mod required {
    use micold_client::features::window::FieldId;
    use micold_client::features::worktree_form::{BranchSource, WorktreeForm};
    use micold_core::naming::{ConventionalType, NamingError};
    use micold_core::worktree::{BranchCandidate, BranchOrigin};

    fn ids(form: &WorktreeForm) -> Vec<FieldId> {
        form.required_fields().iter().map(|(id, _)| *id).collect()
    }

    fn requirement(form: &WorktreeForm, id: FieldId) -> micold_core::required::Requirement {
        form.required_fields()
            .into_iter()
            .find(|(f, _)| *f == id)
            .map(|(_, r)| r)
            .unwrap_or_else(|| panic!("{id:?} is listed"))
    }

    #[test]
    fn new_and_issue_sources_list_type_and_name_in_display_order() {
        for source in [BranchSource::New, BranchSource::Issue] {
            let form = WorktreeForm {
                source,
                ..WorktreeForm::default()
            };
            assert_eq!(
                ids(&form),
                vec![FieldId::AddWorktreeType, FieldId::AddWorktreeName],
                "{source:?}: Type and Name are what preview() refuses empty; Ticket, daemon, AI \
                 CLI and first prompt are optional"
            );
            assert!(form.required_fields().iter().all(|(_, r)| r.required));
        }
    }

    #[test]
    fn the_existing_source_lists_the_branch_only() {
        let form = WorktreeForm {
            source: BranchSource::Existing,
            ..WorktreeForm::default()
        };
        assert_eq!(ids(&form), vec![FieldId::AddWorktreeBranch]);
        assert!(requirement(&form, FieldId::AddWorktreeBranch).required);
    }

    /// For a table of form states, a listed field is `required && empty` exactly when `preview()`
    /// refuses it for emptiness.
    #[test]
    fn a_field_is_empty_exactly_when_preview_refuses_it_for_emptiness() {
        let types = [None, Some(ConventionalType::Feat)];
        let names = ["", "   ", "login", " a "];
        for type_ in types {
            for name in names {
                let form = WorktreeForm {
                    type_,
                    name: name.into(),
                    ..WorktreeForm::default()
                };
                let preview = form.preview();
                let t = requirement(&form, FieldId::AddWorktreeType);
                assert_eq!(
                    t.required && t.empty,
                    preview == Err(NamingError::NoType),
                    "type {type_:?}, name {name:?}"
                );
                let n = requirement(&form, FieldId::AddWorktreeName);
                assert_eq!(
                    n.required && n.empty,
                    name.trim().is_empty(),
                    "type {type_:?}, name {name:?}: Name is empty when blank after trim"
                );
                if type_.is_some() {
                    assert_eq!(
                        n.empty,
                        preview == Err(NamingError::EmptyNameAfterSlug),
                        "type set, name {name:?}: preview refuses a blank name for emptiness"
                    );
                }
            }
        }

        let existing = |selected_branch| WorktreeForm {
            source: BranchSource::Existing,
            selected_branch,
            ..WorktreeForm::default()
        };
        let none = existing(None);
        assert!(requirement(&none, FieldId::AddWorktreeBranch).empty);
        assert_eq!(none.preview(), Err(NamingError::EmptyNameAfterSlug));
        let picked = existing(Some(BranchCandidate {
            name: "feat/login".into(),
            origin: BranchOrigin::Local,
            blocked_by: None,
        }));
        assert!(!requirement(&picked, FieldId::AddWorktreeBranch).empty);
        assert!(picked.preview().is_ok());
    }

    /// A non-blank name that slugs to nothing is not empty: preview() refuses it with its own
    /// non-emptiness error, which the field shows instead of the required one (FR-010).
    #[test]
    fn a_name_that_slugs_to_nothing_is_not_empty() {
        let form = WorktreeForm {
            type_: Some(ConventionalType::Feat),
            name: "!!!".into(),
            ..WorktreeForm::default()
        };
        assert!(!requirement(&form, FieldId::AddWorktreeName).empty);
        assert_eq!(form.preview(), Err(NamingError::EmptyNameAfterSlug));
    }
}

// --- Feature 1102: the GitHub issues switch off (T011; FR-003, FR-004, US1 AS1-3, US2 AS2) ------

mod github_issues_switch {
    use std::path::PathBuf;

    use micold_client::app::{Message, State};
    use micold_client::features::worktree_form::{BranchSource, Msg, WorktreeForm};
    use micold_core::git::GitRemote;
    use micold_core::jira::{connection_key, IssueProvider, JiraConnection};
    use micold_core::naming::ConventionalType;

    fn project() -> PathBuf {
        PathBuf::from("/work/acme")
    }

    fn github_remote() -> Vec<GitRemote> {
        vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }]
    }

    /// A state with the switch as given and, when `jira`, Jira on and connected for `project()`.
    fn state(github_issues_enabled: bool, jira: bool) -> State {
        let mut state = State::default();
        state.workspace.active = Some(project());
        state.settings.github_issues_enabled = github_issues_enabled;
        if jira {
            state.settings.jira_enabled = true;
            state.settings.jira_connections.insert(
                connection_key(&project()),
                JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap(),
            );
        }
        state
    }

    fn opened(mut state: State) -> State {
        state.update(Message::WorktreeForm(Msg::Opened));
        state.update(Message::WorktreeForm(Msg::RemotesListed(Ok(
            github_remote(),
        ))));
        state
    }

    fn form(state: &State) -> &WorktreeForm {
        state.worktree_form.form.as_ref().expect("the form is open")
    }

    /// US1 AS1: no GitHub source, so no request can be built.
    #[test]
    fn off_offers_no_github_source() {
        let state = opened(state(false, false));
        let form = form(&state);
        assert!(form.github_repo().is_none());
        assert!(!form.issue_source_ready());
        assert_eq!(form.provider, IssueProvider::Github);
        assert_eq!(
            form.source_caption().as_deref(),
            Some("GitHub issues are turned off in Settings.")
        );
    }

    /// US1 AS2: Jira is the sole provider and no choice is shown.
    #[test]
    fn off_with_jira_connected_makes_jira_the_only_source() {
        let state = opened(state(false, true));
        let form = form(&state);
        assert_eq!(form.provider, IssueProvider::Jira);
        assert!(!form.offers_issue_provider_choice());
        assert!(form.issue_source_ready());
        assert!(form.github_repo().is_none());
    }

    /// US1 AS3: no issue source at all, and a worktree without an issue is still creatable.
    #[test]
    fn off_with_jira_off_has_no_source_but_creating_still_works() {
        let mut state = opened(state(false, false));
        state.update(Message::WorktreeForm(Msg::SourceChanged(
            BranchSource::Issue,
        )));
        assert_ne!(form(&state).source, BranchSource::Issue);
        state.update(Message::WorktreeForm(Msg::NameChanged("fix-it".into())));
        assert_eq!(form(&state).name, "fix-it", "a name can still be typed");
        // Creating still works: a valid, issue-free submission names the worktree from the form.
        state.worktree_form.form.as_mut().unwrap().type_ = Some(ConventionalType::Fix);
        state.update(Message::WorktreeForm(Msg::Submitted));
        let form = form(&state);
        assert!(form.error.is_none(), "the submission is accepted");
        assert!(form.ticket.is_empty(), "no issue is attached");
        assert!(form.can_press());
        let names = form.preview().expect("the form names a worktree");
        assert_eq!(names.branch, "fix/fix-it");
        assert_eq!(names.dir_name, "fix-fix-it");
    }

    /// US2 AS2: on (the default) the form is the form of before.
    #[test]
    fn on_behaves_as_before() {
        let state = opened(state(true, true));
        let form = form(&state);
        assert!(form.github_repo().is_some());
        assert!(form.offers_issue_provider_choice());
        assert!(form.issue_source_ready());
        assert!(State::default().settings.github_issues_enabled);
        assert!(!WorktreeForm::default().github_issues_off);
    }

    // --- T017: the switch changes while the form is open (FR-005, US1 AS4, US3 AS3) ---------

    fn send(state: &mut State, msg: Msg) {
        state.update(Message::WorktreeForm(msg));
    }

    /// An open form with the GitHub issue load in flight.
    fn loading(state: State) -> State {
        let mut state = opened(state);
        send(&mut state, Msg::SourceChanged(BranchSource::Issue));
        assert!(state.worktree_form.awaited_issue_load().is_some());
        state
    }

    /// US1 AS4: off drops the list, discards the load in flight and leaves the Issue source.
    #[test]
    fn turning_off_drops_the_list_and_discards_the_load() {
        let mut state = loading(state(true, false));
        state
            .workspace
            .issue_sources
            .insert(project(), IssueProvider::Github);
        let before = state.worktree_form.issue_request_seq;
        let seq = state.worktree_form.awaited_issue_load().expect("in flight");
        send(&mut state, Msg::GithubIssuesChanged(false));
        send(
            &mut state,
            Msg::IssuesLoaded {
                seq,
                result: Err(micold_core::github::IssueLoadError::Offline),
                resolved_env: None,
            },
        );
        let f = form(&state);
        assert!(f.github_repo().is_none());
        assert_eq!(
            f.issues,
            micold_client::features::worktree_form::IssueList::NotRequested
        );
        assert_eq!(f.source, BranchSource::New);
        assert!(state.worktree_form.issue_request_seq > before);
        assert!(state.worktree_form.awaited_issue_load().is_none());
        assert_eq!(
            state.workspace.issue_sources.get(&project()),
            Some(&IssueProvider::Github),
            "the remembered provider is untouched"
        );
    }

    /// US1 AS4 with Jira connected: Jira becomes the only source.
    #[test]
    fn turning_off_falls_back_to_jira() {
        let mut state = loading(state(true, true));
        send(&mut state, Msg::GithubIssuesChanged(false));
        let f = form(&state);
        assert_eq!(f.provider, IssueProvider::Jira);
        assert!(!f.offers_issue_provider_choice());
        assert_eq!(f.source, BranchSource::New);
    }

    /// US3 AS3: on offers GitHub again, and choosing the chip loads.
    #[test]
    fn turning_on_offers_github_and_loads_on_choosing_the_chip() {
        let mut state = opened(state(false, false));
        assert!(form(&state).github_repo().is_none());
        send(&mut state, Msg::GithubIssuesChanged(true));
        assert!(form(&state).github_repo().is_some());
        assert!(form(&state).issue_source_ready());
        send(&mut state, Msg::SourceChanged(BranchSource::Issue));
        assert!(state.worktree_form.awaited_issue_load().is_some());
    }

    /// An unchanged value changes nothing, not even the request counter.
    #[test]
    fn an_unchanged_value_is_a_no_op() {
        let mut state = loading(state(true, false));
        let before = state.worktree_form.issue_request_seq;
        send(&mut state, Msg::GithubIssuesChanged(true));
        assert_eq!(state.worktree_form.issue_request_seq, before);
        assert!(state.worktree_form.awaited_issue_load().is_some());
    }
}
