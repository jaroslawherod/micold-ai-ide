//! Spec 1857 (`contracts/mcp-parent.md`; quickstart Part A; FR-001 to FR-009, FR-011, FR-014):
//! `create_worktree` records the caller's worktree as the parent, `list_worktrees` and the session
//! tools report it, and the link survives renames and restarts and is tombstoned, never
//! re-adopted, when the parent goes.
//!
//! Fixture: a real repository with worktrees `w` (session S3) and `hand` (made by hand, so it has no
//! parent), and a session S1 in the project root.

use crate::mcp_support;

use std::path::Path;
use std::sync::Arc;

use mcp_support::*;
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, SessionId, TerminalMode};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::ops;
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};

struct Fixture {
    state: Arc<DaemonState>,
    addr: std::net::SocketAddr,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        init_repo(project.path());
        add_worktree(project.path(), "w");
        add_worktree(project.path(), "hand");
        add_worktree(project.path(), "kid");
        let state = state_over_files(project.path(), store.path());
        let addr = serve_tool_server(&state, store.path().join("mcp")).await;
        Self {
            state,
            addr,
            project,
            store,
        }
    }

    fn path(&self) -> &Path {
        self.project.path()
    }

    async fn call(&self, caller: SessionId, tool: &str, args: Value) -> Value {
        call_ok(self.addr, &credential(&self.state, caller), tool, args).await
    }

    async fn err(&self, caller: SessionId, tool: &str, args: Value) -> Value {
        call_err(self.addr, &credential(&self.state, caller), tool, args).await
    }

    /// `list_worktrees` rows by ref, as `caller` sees them.
    async fn rows(&self, caller: SessionId, args: Value) -> Vec<Value> {
        let out = self.call(caller, "list_worktrees", args).await;
        out["worktrees"].as_array().unwrap().clone()
    }

    async fn row(&self, name: &str) -> Value {
        self.rows(sid(1), json!({}))
            .await
            .into_iter()
            .find(|r| r["ref"] == name)
            .unwrap_or_else(|| panic!("no row {name}"))
    }

    /// Create worktree `dir` (a literal branch of the same name) as `caller`.
    async fn create(&self, caller: SessionId, dir: &str, extra: Value) -> Value {
        let mut args = json!({"branch": dir});
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().cloned().unwrap_or_default());
        self.call(caller, "create_worktree", args).await
    }

    async fn delete(&self, dir: &str) {
        ops::delete_worktree(
            &self.state,
            self.path().to_path_buf(),
            dir.into(),
            true,
            false,
        )
        .await
        .expect("the delete succeeds");
    }

    fn refs(rows: &[Value]) -> Vec<String> {
        rows.iter()
            .map(|r| r["ref"].as_str().unwrap().to_string())
            .collect()
    }
}

fn catalog_over(store_dir: &Path, workspace: Option<Workspace>) -> Catalog {
    let store = JsonFileStore::at(store_dir.join("projects.json"));
    if let Some(workspace) = workspace {
        store.save(&workspace).unwrap();
    }
    Catalog::load(
        Box::new(store),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

/// A daemon state over a real project store in `store_dir`, so a second one over the same directory
/// is a restart.
fn state_over_files(project: &Path, store_dir: &Path) -> Arc<DaemonState> {
    let mut workspace = Workspace {
        active: Some(project.to_path_buf()),
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        ..Default::default()
    };
    workspace.sessions.insert(
        project.to_path_buf(),
        vec![
            session(sid(1), None, TerminalMode::AiCli, AiCli::ClaudeCode),
            session(sid(3), Some("w"), TerminalMode::AiCli, AiCli::ClaudeCode),
            session(sid(5), Some("kid"), TerminalMode::AiCli, AiCli::ClaudeCode),
        ],
    );
    Arc::new(DaemonState::new(catalog_over(store_dir, Some(workspace))))
}

fn restarted(f: &Fixture) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(catalog_over(f.store.path(), None)))
}

fn parent_of(row: &Value) -> (Option<&str>, Option<&str>) {
    (row["parent"].as_str(), row["parent_state"].as_str())
}

// --- T004: ids and reconcile -----------------------------------------------------------------

#[tokio::test]
async fn the_first_refresh_mints_an_id_for_every_live_worktree_and_keeps_them() {
    let f = Fixture::new().await;
    assert!(f.state.worktree_ids(f.path()).is_empty());
    f.state.refresh_worktrees(f.path());
    let first = f.state.worktree_ids(f.path());
    assert_eq!(
        first.keys().cloned().collect::<Vec<_>>(),
        ["hand", "kid", "w"]
    );
    f.state.refresh_worktrees(f.path());
    assert_eq!(
        f.state.worktree_ids(f.path()),
        first,
        "ids are kept across refreshes"
    );
    ops::rename_worktree(&f.state, f.path(), "w", "Renamed").unwrap();
    f.state.refresh_worktrees(f.path());
    assert_eq!(f.state.worktree_ids(f.path()), first, "and across renames");
}

#[tokio::test]
async fn a_worktree_removed_outside_the_app_tombstones_its_children_at_the_next_refresh() {
    let f = Fixture::new().await;
    f.create(sid(3), "child", json!({})).await;
    git(
        f.path(),
        &["worktree", "remove", "--force", ".claude/worktrees/w"],
    );
    f.state.refresh_worktrees(f.path());
    assert_eq!(
        parent_of(&f.row("child").await),
        (Some("w"), Some("removed"))
    );
}

#[tokio::test]
async fn default_links_are_never_removed_and_an_unrelated_delete_changes_nothing() {
    let f = Fixture::new().await;
    f.create(sid(1), "root-child", json!({})).await;
    f.delete("hand").await;
    assert_eq!(
        parent_of(&f.row("root-child").await),
        (Some("default"), Some("known"))
    );
}

// --- US1: record, list, filter ---------------------------------------------------------------

#[tokio::test]
async fn a_root_session_records_default_and_an_app_made_worktree_has_none() {
    let f = Fixture::new().await;
    f.create(sid(1), "a", json!({})).await;
    f.create(sid(1), "b", json!({})).await;
    assert_eq!(
        parent_of(&f.row("a").await),
        (Some("default"), Some("known"))
    );
    assert_eq!(
        parent_of(&f.row("b").await),
        (Some("default"), Some("known"))
    );
    assert_eq!(parent_of(&f.row("hand").await), (None, None));
    assert_eq!(parent_of(&f.row("default").await), (None, None));
}

#[tokio::test]
async fn a_session_in_a_worktree_records_it_and_an_explicit_parent_overrides() {
    let f = Fixture::new().await;
    let made = f.create(sid(3), "c1", json!({})).await;
    assert_eq!(
        parent_of(&made),
        (Some("w"), Some("known")),
        "the create result carries it"
    );
    f.create(sid(3), "c2", json!({"parent": "default"})).await;
    f.create(sid(1), "c3", json!({"parent": "w"})).await;
    assert_eq!(parent_of(&f.row("c1").await), (Some("w"), Some("known")));
    assert_eq!(
        parent_of(&f.row("c2").await),
        (Some("default"), Some("known"))
    );
    assert_eq!(parent_of(&f.row("c3").await), (Some("w"), Some("known")));
}

#[tokio::test]
async fn an_explicit_parent_that_is_not_live_is_refused_and_nothing_is_created() {
    let f = Fixture::new().await;
    f.state.refresh_worktrees(f.path());
    let ids_before = f.state.worktree_ids(f.path());
    let on_disk = std::fs::read_dir(f.path().join(".claude/worktrees"))
        .unwrap()
        .count();
    let error = f
        .err(
            sid(3),
            "create_worktree",
            json!({"branch": "nope-child", "parent": "nope"}),
        )
        .await;
    assert_eq!(error["category"], "invalid_input", "{error}");
    assert_eq!(
        std::fs::read_dir(f.path().join(".claude/worktrees"))
            .unwrap()
            .count(),
        on_disk
    );
    assert_eq!(
        f.state.worktree_ids(f.path()),
        ids_before,
        "no id was minted"
    );
}

#[tokio::test]
async fn the_parent_filter_returns_direct_children_only() {
    let f = Fixture::new().await;
    f.create(sid(3), "c1", json!({})).await;
    f.create(sid(3), "c2", json!({})).await;
    f.create(sid(1), "other", json!({})).await;
    let under_w = f.rows(sid(1), json!({"parent": "w"})).await;
    assert_eq!(Fixture::refs(&under_w), ["c1", "c2"]);
    let under_default = f.rows(sid(1), json!({"parent": "default"})).await;
    assert_eq!(Fixture::refs(&under_default), ["other"]);
    assert!(f.rows(sid(1), json!({"parent": "nope"})).await.is_empty());
}

#[tokio::test]
async fn an_unresolvable_caller_creates_an_unparented_worktree() {
    let f = Fixture::new().await;
    git(
        f.path(),
        &["worktree", "remove", "--force", ".claude/worktrees/w"],
    );
    f.state.refresh_worktrees(f.path());
    let made = f.create(sid(3), "orphan", json!({})).await;
    assert_eq!(parent_of(&made), (None, None));
}

// --- US2: rename, restart ---------------------------------------------------------------------

#[tokio::test]
async fn renaming_the_parent_keeps_the_link() {
    let f = Fixture::new().await;
    f.create(sid(3), "c1", json!({})).await;
    ops::rename_worktree(&f.state, f.path(), "w", "A new name").unwrap();
    assert_eq!(parent_of(&f.row("c1").await), (Some("w"), Some("known")));
}

#[tokio::test]
async fn a_restart_keeps_the_link() {
    let f = Fixture::new().await;
    f.create(sid(3), "c1", json!({})).await;
    f.create(sid(1), "c2", json!({})).await;
    let reloaded = restarted(&f);
    let views = reloaded.worktree_parent_views(f.path());
    assert_eq!(views["c1"].name, "w");
    assert_eq!(views["c1"].state.as_str(), "known");
    assert_eq!(views["c2"].name, "default");
    assert_eq!(
        reloaded.worktree_ids(f.path()),
        f.state.worktree_ids(f.path()),
        "ids survive too"
    );
}

// --- US3: removed parent ----------------------------------------------------------------------

#[tokio::test]
async fn deleting_the_parent_leaves_children_removed_and_a_same_name_worktree_adopts_none() {
    let f = Fixture::new().await;
    f.create(sid(3), "c1", json!({})).await;
    f.delete("w").await;
    assert_eq!(
        parent_of(&f.row("c1").await),
        (Some("w"), Some("removed")),
        "the child keeps the last name and no error is raised"
    );
    f.create(sid(1), "w", json!({"mode": "existing_local"})).await;
    assert_eq!(parent_of(&f.row("c1").await), (Some("w"), Some("removed")));
    assert_eq!(
        parent_of(&f.row("w").await),
        (Some("default"), Some("known"))
    );
    assert!(
        f.rows(sid(1), json!({"parent": "w"})).await.is_empty(),
        "the recreated worktree has no children"
    );
}

#[tokio::test]
async fn deleting_a_child_drops_its_entries() {
    let f = Fixture::new().await;
    f.create(sid(3), "c1", json!({})).await;
    f.delete("c1").await;
    assert!(!f.state.worktree_ids(f.path()).contains_key("c1"));
    assert!(!f.state.worktree_parent_views(f.path()).contains_key("c1"));
}

// --- US5: sessions ----------------------------------------------------------------------------

#[tokio::test]
async fn session_rows_carry_the_worktrees_parent_independent_of_origin() {
    let f = Fixture::new().await;
    f.state.refresh_worktrees(f.path());
    f.state.record_worktree_parent(
        f.path(),
        "kid",
        Some(&micold_core::worktree_parent::ParentChoice::Worktree(
            "w".into(),
        )),
    );
    let sessions = f.call(sid(1), "list_sessions", json!({})).await;
    let rows = sessions["sessions"].as_array().unwrap();
    let of = |n: u128| {
        rows.iter()
            .find(|r| r["ref"] == sid(n).0.to_string())
            .unwrap()
            .clone()
    };
    assert_eq!(parent_of(&of(5)), (Some("w"), Some("known")));
    assert_eq!(parent_of(&of(3)), (None, None), "w is unparented");
    assert_eq!(parent_of(&of(1)), (None, None), "default has none");
    assert!(
        of(5).get("origin").is_none(),
        "origin is independent (FR-011)"
    );
    let one = f
        .call(
            sid(1),
            "get_session",
            json!({"session": sid(5).0.to_string()}),
        )
        .await;
    assert_eq!(parent_of(&one), (Some("w"), Some("known")));
}
