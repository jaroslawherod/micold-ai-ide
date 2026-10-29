//! The delete prompt's loss list keeps its buttons on screen and its rows inside the dialog
//! (#1602, FR-006, SC-003): 50 files scroll rather than push Allow and Deny away.

mod support;

use micold_client::app::State;
use micold_client::features::agent_confirm::Prompt;
use micold_core::protocol::messages::{ConfirmOperation, LossCommit, LossSummary};
use support::layout::{painted_text_settled, renderer, view_of, StateUnderTest, WINDOW};

fn painted(files: usize) -> Vec<support::layout::Overflow> {
    let loss = LossSummary {
        files: (0..files)
            .map(|i| format!("crates/micold-client/src/deeply/nested/module_{i}/file_{i}.rs"))
            .collect(),
        files_more: 3,
        commits: vec![LossCommit {
            short_hash: "abc1234".into(),
            subject: "feat: a commit that is on no remote branch yet".into(),
        }],
        commits_more: 0,
        unreadable: false,
    };
    let mut state = State::default();
    state.agent_confirm.pending = vec![Prompt {
        session: micold_core::session::SessionId(Default::default()),
        id: 1,
        project: "/p".into(),
        caller_label: "planner".into(),
        operation: ConfirmOperation::DeleteWorktree {
            stop_sessions: false,
            delete_branch: true,
        },
        target_label: "feat-x".into(),
        self_targeted: true,
        loss: Some(loss),
    }];
    painted_text_settled(view_of(&StateUnderTest::new(state)), &mut renderer())
}

#[test]
fn fifty_files_scroll_and_leave_both_buttons_on_screen() {
    let texts = painted(50);
    for label in ["Allow", "Deny"] {
        let t = texts
            .iter()
            .find(|t| t.content == label)
            .unwrap_or_else(|| panic!("{label} is painted"));
        assert!(
            t.origin.y > 0.0 && t.origin.y < WINDOW.height,
            "{label} is inside the window: {:?}",
            t.origin
        );
    }
}

#[test]
fn the_headings_count_every_entry_and_rows_are_not_cut_by_their_width() {
    let texts = painted(50);
    assert!(texts
        .iter()
        .any(|t| t.content == "Uncommitted changes (53)"));
    assert!(texts.iter().any(|t| t.content == "Unpushed commits (1)"));
    let rows: Vec<_> = texts
        .iter()
        .filter(|t| t.content.contains("module_0/") || t.content.starts_with("abc1234"))
        .collect();
    assert_eq!(
        rows.len(),
        2,
        "first file row and the commit row are painted"
    );
    for row in rows {
        assert!(
            row.natural_width <= row.allowed_width + 0.5,
            "{} fits its width",
            row.content
        );
    }
}

#[test]
fn a_few_files_make_a_shorter_dialog_than_fifty() {
    let top = |texts: &[support::layout::Overflow]| {
        texts
            .iter()
            .find(|t| t.content.starts_with("“planner”"))
            .expect("headline")
            .origin
            .y
    };
    assert!(
        top(&painted(50)) <= top(&painted(2)),
        "the dialog grows with the list"
    );
}
