//! Spec 1945 (contracts/mcp-neighbourhood.md; US2, US3; FR-003..FR-008, SC-002, SC-003, SC-005):
//! `neighbourhood`, and the refusals of `send_parent_input`, over the fixture of
//! `mcp_worktree_parent_visibility`: `P` (no parent) with children `A` and `B`; `C`, a child of `A`;
//! `X` (no parent) with child `Y`; one session each, and `default` with its own. The fixture's
//! sessions run no process, so a send past its refusals is a conflict here; delivery is tested in
//! `mcp_cross_session`.

use crate::mcp_support::{add_worktree, sid};
use crate::mcp_worktree_parent_visibility::{Fixture, A, B, C, DEFAULT, P, X, Y};

use micold_core::worktree_parent::ParentChoice;
use serde_json::{json, Value};

const IMPLICIT_NOTE: &str = "no recorded parent: \"default\" is taken as the parent for visibility";
const NAMED_DEFAULT_NOTE: &str =
    "this session's worktree directory is named \"default\", which is not the project root, so \
     it has no parent";

async fn near(f: &Fixture, caller: u128) -> Value {
    f.call(caller, "neighbourhood", json!({})).await
}

fn id(n: u128) -> String {
    sid(n).0.to_string()
}

/// The `ref` of each entry of `entries`, in order.
fn refs(entries: &Value) -> Vec<String> {
    entries
        .as_array()
        .unwrap_or_else(|| panic!("not a list: {entries}"))
        .iter()
        .map(|e| e["ref"].as_str().unwrap().to_string())
        .collect()
}

/// The session refs of one worktree entry.
fn sessions(entry: &Value) -> Vec<String> {
    refs(&entry["sessions"])
}

/// Every worktree ref an answer names, the parent's included unless it was removed.
fn named_worktrees(out: &Value) -> Vec<String> {
    let mut names = Vec::new();
    names.extend(out["worktree"]["ref"].as_str().map(String::from));
    if out["parent"]["state"] == "known" {
        names.extend(out["parent"]["ref"].as_str().map(String::from));
    }
    names.extend(refs(&out["siblings"]));
    names.extend(refs(&out["children"]));
    names
}

/// Every session ref an answer names.
fn named_sessions(out: &Value) -> Vec<String> {
    let mut all = Vec::new();
    for entry in [&out["worktree"], &out["parent"]] {
        if !entry.is_null() {
            all.extend(sessions(entry));
        }
    }
    for list in [&out["siblings"], &out["children"]] {
        all.extend(list.as_array().unwrap().iter().flat_map(sessions));
    }
    all
}

/// `Q` with children `R` and `R2`; `Q` is then removed.
async fn removed_parent() -> Fixture {
    let f = Fixture::with(
        &[("Q", 16, true), ("R", 17, true), ("R2", 18, true)],
        &[("R", "Q"), ("R2", "Q")],
    )
    .await;
    f.remove_worktree("Q");
    f
}

async fn send_err(f: &Fixture, caller: u128) -> Value {
    f.err(caller, "send_parent_input", json!({"text": "done"}))
        .await
}

// ---------------------------------------------------------------------------------------
// neighbourhood
// ---------------------------------------------------------------------------------------

/// US2 s1, SC-002: one call holds the caller's worktree, its recorded parent, siblings and children.
#[tokio::test]
async fn a_recorded_parent_its_siblings_and_children_come_in_one_call() {
    let f = Fixture::new().await;
    let out = near(&f, A).await;
    assert_eq!(out["worktree"]["ref"], "A", "{out}");
    assert_eq!(sessions(&out["worktree"]), [id(A)]);
    assert_eq!(out["worktree"]["sessions"][0]["is_caller"], true);
    assert_eq!(out["parent"]["ref"], "P");
    assert_eq!(out["parent"]["state"], "known");
    assert_eq!(out["parent"]["recorded"], true);
    assert_eq!(sessions(&out["parent"]), [id(P)]);
    assert_eq!(refs(&out["siblings"]), ["B"]);
    assert_eq!(sessions(&out["siblings"][0]), [id(B)]);
    assert_eq!(refs(&out["children"]), ["C"]);
    assert_eq!(sessions(&out["children"][0]), [id(C)]);
    assert!(
        out.get("note").is_none(),
        "no note for a recorded parent: {out}"
    );
    // The rows are `list_sessions` rows.
    let listed = f.call(A, "list_sessions", json!({})).await;
    let row = listed["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ref"] == id(P))
        .unwrap()
        .clone();
    assert_eq!(out["parent"]["sessions"][0], row);
}

/// US2 s2, FR-004: an unparented worktree has the implicit parent `default`, and the note says so.
#[tokio::test]
async fn an_unparented_worktree_has_default_as_an_implicit_parent() {
    let f = Fixture::new().await;
    let out = near(&f, P).await;
    assert_eq!(out["parent"]["ref"], "default", "{out}");
    assert_eq!(out["parent"]["state"], "known");
    assert_eq!(out["parent"]["recorded"], false);
    assert_eq!(sessions(&out["parent"]), [id(DEFAULT)]);
    assert_eq!(refs(&out["siblings"]), ["X"]);
    assert_eq!(refs(&out["children"]), ["A", "B"]);
    assert!(out["children"][0].get("implicit").is_none(), "{out}");
    assert_eq!(out["note"], IMPLICIT_NOTE);
}

/// US2 s3: a removed parent has no sessions and links no siblings.
#[tokio::test]
async fn a_removed_parent_has_no_sessions_and_no_siblings() {
    let f = removed_parent().await;
    let out = near(&f, 17).await;
    assert_eq!(
        out["parent"],
        json!({"ref": "Q", "state": "removed", "recorded": true, "sessions": []}),
        "{out}"
    );
    assert_eq!(out["siblings"], json!([]));
    assert!(
        out["note"].as_str().unwrap().contains("was removed"),
        "{out}"
    );
}

/// US2 s4: `default` has no parent; its children include the unparented worktrees, marked implicit.
#[tokio::test]
async fn default_has_no_parent_and_its_unparented_children_are_implicit() {
    let f = Fixture::with(&[("D", 16, true)], &[("D", "default")]).await;
    let out = near(&f, DEFAULT).await;
    assert_eq!(out["worktree"]["ref"], "default", "{out}");
    assert_eq!(sessions(&out["worktree"]), [id(DEFAULT)]);
    assert_eq!(out["parent"], Value::Null);
    assert_eq!(out["siblings"], json!([]));
    assert_eq!(refs(&out["children"]), ["D", "P", "X"]);
    let implicit: Vec<bool> = out["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["implicit"] == true)
        .collect();
    assert_eq!(
        implicit,
        [false, true, true],
        "only unparented children are implicit"
    );
    assert_eq!(out["note"], "default has no parent");
}

/// A caller with a second session in its worktree sees both, itself marked.
#[tokio::test]
async fn every_session_of_the_callers_worktree_is_listed() {
    let f = Fixture::with(&[("A", 16, false)], &[]).await;
    let out = near(&f, 16).await;
    assert_eq!(sessions(&out["worktree"]), [id(A), id(16)], "{out}");
    let callers: Vec<bool> = out["worktree"]["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["is_caller"] == true)
        .collect();
    assert_eq!(callers, [false, true]);
}

/// A caller whose worktree the daemon does not know gets nothing, and the note says why.
#[tokio::test]
async fn an_unknown_worktree_gets_empty_lists_and_a_note() {
    let f = Fixture::with(&[("late", 16, false)], &[("late", "P")]).await;
    add_worktree(f._project.path(), "late");
    assert_eq!(
        near(&f, 16).await,
        json!({
            "worktree": null,
            "parent": null,
            "siblings": [],
            "children": [],
            "note": "this session's worktree is not known to the daemon",
        })
    );
}

/// US2 s5, FR-005, SC-005: no caller is shown a worktree or session it could not list.
#[tokio::test]
async fn nothing_outside_the_callers_scope_appears() {
    let f = Fixture::new().await;
    for caller in [DEFAULT, P, A, B, C, X, Y] {
        let out = near(&f, caller).await;
        let visible = f.worktrees(caller).await;
        for name in named_worktrees(&out) {
            assert!(visible.contains(&name), "{caller}: {name} in {out}");
        }
        let listed = f.call(caller, "list_sessions", json!({})).await;
        let listed = refs(&listed["sessions"]);
        for session in named_sessions(&out) {
            assert!(listed.contains(&session), "{caller}: {session} in {out}");
        }
    }
}

// ---------------------------------------------------------------------------------------
// send_parent_input: refusals (contracts/mcp-neighbourhood.md table; FR-007)
// ---------------------------------------------------------------------------------------

/// US3 s5: `default` has no parent.
#[tokio::test]
async fn a_send_from_default_is_refused_as_default_has_no_parent() {
    let f = Fixture::new().await;
    let error = send_err(&f, DEFAULT).await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("default has no parent"),
        "{error}"
    );
}

/// US3 s6: a removed parent is named, and nothing is sent.
#[tokio::test]
async fn a_send_to_a_removed_parent_is_refused_naming_it() {
    let f = removed_parent().await;
    let error = send_err(&f, 17).await;
    assert_eq!(error["category"], "not_found", "{error}");
    let message = error["message"].as_str().unwrap();
    assert!(
        message.contains("\"Q\"") && message.contains("was removed"),
        "{error}"
    );
}

/// US3 s3: a parent with no running session is named; the implicit `default` too (US3 s4).
#[tokio::test]
async fn a_send_to_a_parent_with_no_running_session_is_refused_naming_it() {
    let f = Fixture::new().await;
    for (caller, parent) in [(A, "\"P\""), (P, "\"default\"")] {
        let error = send_err(&f, caller).await;
        assert_eq!(error["category"], "conflict", "{error}");
        let message = error["message"].as_str().unwrap();
        assert!(
            message.contains(parent) && message.contains("has no running session"),
            "{error}"
        );
    }
}

/// A caller whose worktree the daemon does not know has no parent to send to.
#[tokio::test]
async fn a_send_from_an_unknown_worktree_is_refused() {
    let f = Fixture::with(&[("late", 16, false)], &[("late", "P")]).await;
    add_worktree(f._project.path(), "late");
    let error = send_err(&f, 16).await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("no parent could be determined"),
        "{error}"
    );
}

/// A live worktree directory named `default` is not the project root: its own sessions are its
/// `worktree`, it has no parent, and a send says why.
#[tokio::test]
async fn a_worktree_named_default_is_its_own_worktree_and_a_send_from_it_is_refused() {
    let f = Fixture::with(&[("default", 16, true)], &[]).await;
    let out = near(&f, 16).await;
    assert_eq!(out["worktree"]["ref"], "default", "{out}");
    assert_eq!(sessions(&out["worktree"]), [id(16)], "{out}");
    assert_eq!(out["parent"], Value::Null);
    assert_eq!(out["note"], NAMED_DEFAULT_NOTE, "{out}");
    let error = send_err(&f, 16).await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert_eq!(error["message"], NAMED_DEFAULT_NOTE, "{error}");
}

/// A recorded parent that is a live directory named `default` is that worktree: its sessions are
/// listed, never the project root's, and the root's other children are not siblings. The directory
/// lists the child back.
#[tokio::test]
async fn a_parent_directory_named_default_lists_its_own_sessions() {
    let f = Fixture::with(&[("default", 16, true), ("K", 17, true)], &[]).await;
    f.state.record_worktree_parent(
        f._project.path(),
        "K",
        Some(&ParentChoice::Worktree("default".into())),
    );
    let out = near(&f, 17).await;
    assert_eq!(out["parent"]["ref"], "default", "{out}");
    assert_eq!(out["parent"]["recorded"], true, "{out}");
    assert_eq!(sessions(&out["parent"]), [id(16)], "{out}");
    assert_eq!(out["siblings"], json!([]), "{out}");
    assert_eq!(refs(&near(&f, 16).await["children"]), ["K"]);
    let root = near(&f, DEFAULT).await;
    assert!(
        !refs(&root["children"]).contains(&"K".to_string()),
        "{root}"
    );
}
