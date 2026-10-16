//! US1 (feature 552) — the sidebar lists every worktree and decides nothing from a name.
//!
//! Features 014 and 029 hid worktrees by directory name, then by a creation record. 552 removes
//! both: no name, branch or record changes whether a worktree is listed, how it is ordered or which
//! tags it carries, and there is no control that reveals anything.

use micold_client::app::State;
use micold_client::features::sidebar::SidebarEntry;
use micold_client::features::worktree;
use micold_core::project::{Availability, Project};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;

const AGENT_DIR: &str = "agent-0123456789abcdef0";
const AGENT_BRANCH: &str = "worktree-agent-0123456789abcdef0";

fn project() -> PathBuf {
    PathBuf::from("/repo")
}

fn managed(dir_name: &str, branch: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: project().join(".claude/worktrees").join(dir_name),
        branch: Some(branch.to_string()),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

fn elsewhere(dir_name: &str) -> Worktree {
    Worktree {
        dir_name: dir_name.to_string(),
        path: PathBuf::from("/somewhere/else").join(dir_name),
        branch: Some(format!("feat/{dir_name}")),
        status: WorktreeStatus::Valid,
        included: true,
    }
}

fn state(worktrees: Vec<Worktree>) -> State {
    let mut st = State {
        worktree: worktree::State {
            worktrees,
            ..Default::default()
        },
        ..Default::default()
    };
    st.workspace.projects.push(Project {
        path: project(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    st.workspace.active = Some(project());
    st
}

fn listed(st: &State) -> Vec<String> {
    st.worktree_tree()
        .into_iter()
        .map(|node| node.display_name)
        .collect()
}

fn dirs(st: &State) -> Vec<String> {
    st.worktree
        .worktrees
        .iter()
        .map(|w| w.dir_name.clone())
        .collect()
}

#[test]
fn an_agent_named_worktree_is_listed_like_an_ordinary_one() {
    let st = state(vec![
        managed("feat-login", "feat/login"),
        managed(AGENT_DIR, AGENT_BRANCH),
        managed("fix-the-parser", "fix/parser"),
    ]);
    assert_eq!(
        listed(&st),
        vec!["Login", "Agent 0123456789abcdef0", "The parser"]
    );
}

#[test]
fn every_worktree_is_listed() {
    let st = state(vec![
        managed("recorded", "feat/recorded"),
        managed("unrecorded", "feat/unrecorded"),
        elsewhere("borrowed"),
    ]);
    assert_eq!(listed(&st), vec!["Recorded", "Unrecorded", "Borrowed"]);
}

#[test]
fn renaming_to_or_from_the_old_convention_changes_neither_listing_nor_order() {
    let before = state(vec![
        managed("aaa", "feat/aaa"),
        managed("bbb", "feat/bbb"),
        managed("ccc", "feat/ccc"),
    ]);
    let after = state(vec![
        managed(AGENT_DIR, AGENT_BRANCH),
        managed("bbb", "feat/bbb"),
        managed("ccc", "worktree-agent-ffffffffffffffff0"),
    ]);
    assert_eq!(listed(&before), vec!["Aaa", "Bbb", "Ccc"]);
    assert_eq!(
        listed(&after),
        vec!["Agent 0123456789abcdef0", "Bbb", "Ccc"]
    );
    // The order is the discovered order, unchanged by any name.
    assert_eq!(dirs(&after), vec![AGENT_DIR, "bbb", "ccc"]);
    assert_eq!(
        after
            .worktree_tree()
            .iter()
            .map(|n| n.display_name.clone())
            .collect::<Vec<_>>(),
        listed(&after)
    );
}

#[test]
fn the_sidebar_view_has_no_reveal_control() {
    let st = state(vec![
        managed("feat-login", "feat/login"),
        managed(AGENT_DIR, AGENT_BRANCH),
    ]);
    // The rows are the project root then one row per worktree, in order: no extra entry stands
    // for a "show agent worktrees" control.
    let rows: Vec<String> = st
        .sidebar_entries()
        .iter()
        .map(|entry| match entry {
            SidebarEntry::Default(d) => d.display_name.to_string(),
            SidebarEntry::Worktree(w) => w.display_name.clone(),
            SidebarEntry::Group(g) => panic!("unexpected group row {}", g.name),
        })
        .collect();
    assert_eq!(rows, vec!["Default", "Login", "Agent 0123456789abcdef0"]);
}

#[test]
fn a_created_worktree_is_listed_in_the_frame_it_appears() {
    let mut st = state(vec![]);
    worktree::created(&mut st, managed("feat-login", "feat/login"));
    assert_eq!(listed(&st), vec!["Login"]);
}

#[test]
fn a_refresh_keeps_every_worktree_listed() {
    let mut st = state(vec![managed("feat-login", "feat/login")]);
    let _ = st.set_worktrees(vec![
        managed("feat-login", "feat/login"),
        managed(AGENT_DIR, AGENT_BRANCH),
    ]);
    assert_eq!(listed(&st), vec!["Login", "Agent 0123456789abcdef0"]);
}

#[test]
fn the_project_root_is_listed_beside_every_worktree() {
    let st = state(vec![managed(AGENT_DIR, AGENT_BRANCH)]);
    let rows: Vec<String> = st
        .sidebar_entries()
        .iter()
        .map(|entry| match entry {
            SidebarEntry::Default(d) => d.display_name.to_string(),
            SidebarEntry::Worktree(w) => w.display_name.clone(),
            SidebarEntry::Group(g) => g.name.clone(),
        })
        .collect();
    assert_eq!(rows, vec!["Default", "Agent 0123456789abcdef0"]);
}
