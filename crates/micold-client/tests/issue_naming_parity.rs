//! 550 FR-016: the form's issue pick and the MCP tool's `github_issue` derive from one function,
//! and a test fails when they disagree.

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::{IssueList, Msg};
use micold_core::git::GitRemote;
use micold_core::github::{Issue, IssueListing};
use micold_core::issue_types::{default_mapping, LabelTypeEntry};
use micold_core::naming::{naming_for_issue, ConventionalType};
use std::path::PathBuf;

fn send(state: &mut State, msg: Msg) {
    state.update(Message::WorktreeForm(msg));
}

fn picked(issue: Issue, mapping: Vec<LabelTypeEntry>) -> State {
    let number = issue.number();
    let mut state = State::default();
    send(&mut state, Msg::Opened);
    send(
        &mut state,
        Msg::RemotesListed(Ok(vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }])),
    );
    send(
        &mut state,
        Msg::SourceChanged(micold_client::features::worktree_form::BranchSource::Issue),
    );
    let seq = match &state.worktree_form.form.as_ref().unwrap().issues {
        IssueList::Loading { seq } => *seq,
        other => panic!("not loading: {other:?}"),
    };
    send(
        &mut state,
        Msg::IssuesLoaded {
            seq,
            result: Ok((
                IssueListing {
                    total_open: 1,
                    complete: true,
                    issues: vec![issue],
                },
                PathBuf::from("/usr/bin/gh"),
            )),
            resolved_env: None,
        },
    );
    send(&mut state, Msg::IssuePicked { number, mapping });
    state
}

#[test]
fn the_forms_pick_equals_naming_for_issue() {
    let long = "Login crash when the user opens the project settings dialog twice";
    let cases = [
        ("Login crash", vec!["bug"]),
        (long, vec!["enhancement"]),
        ("Unmapped", vec!["question"]),
        ("Mixed case", vec!["DOCUMENTATION"]),
    ];
    for (title, labels) in cases {
        let issue = Issue::new(
            123,
            title.to_string(),
            labels.iter().map(|l| l.to_string()).collect(),
            String::new(),
        );
        let state = picked(issue.clone(), default_mapping());
        let form = state.worktree_form.form.as_ref().unwrap();
        let shared = naming_for_issue(&issue, &default_mapping());
        assert_eq!(form.type_, shared.type_, "{title}");
        assert_eq!(Some(form.ticket.clone()), shared.ticket, "{title}");
        assert_eq!(form.name, shared.name, "{title}");
    }
    let _ = ConventionalType::Fix;
}
