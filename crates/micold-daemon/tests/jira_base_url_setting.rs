//! Each project's Jira base URL on the service (feature 1122, D4, contracts/wire.md W1, W2):
//! `SettingsSet { jira_base_url }` carries one project's change, which the service merges into
//! the map in the settings file and pushes to every connected client as `SettingsChanged`, so two
//! windows saving different projects never overwrite each other (FR-010, FR-012).
//!
//! Drives `server::serve_connection` over an in-memory duplex against a settings file in a
//! temporary directory, so "persisted" is checked by starting a second service over that file.

#[path = "support/conn.rs"]
mod conn;

use std::path::Path;
use std::process::Command;

use conn::{
    answer, both_registered, connect_with_settings, next_settings_changed, service, set, stored,
};
use futures_util::SinkExt;
use micold_core::jira::{connection_key, JiraBaseUrl};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::settings::JiraBaseUrlChange;

fn base(url: &str) -> JiraBaseUrl {
    JiraBaseUrl::parse(url).expect("valid base URL")
}

fn set_base(project: &str, url: &str) -> Option<JiraBaseUrlChange> {
    Some(JiraBaseUrlChange {
        project: project.to_owned(),
        url: Some(base(url)),
    })
}

fn clear_base(project: &str) -> Option<JiraBaseUrlChange> {
    Some(JiraBaseUrlChange {
        project: project.to_owned(),
        url: None,
    })
}

/// A set for one project on one connection reaches the other connection.
#[tokio::test]
async fn a_set_is_broadcast_to_every_connection() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut first, welcomed) = connect_with_settings(&state).await;
    assert!(welcomed.jira_base_urls.is_empty(), "unset by default");
    let (mut second, _) = connect_with_settings(&state).await;
    both_registered(&state).await;

    set(
        &mut first,
        1,
        None,
        None,
        set_base("/work/a", "https://a.atlassian.net"),
        None,
    )
    .await;

    let pushed = next_settings_changed(&mut second).await;
    assert_eq!(
        pushed.jira_base_urls["/work/a"],
        base("https://a.atlassian.net")
    );
}

/// Two clients setting different projects both survive, on disk and in the next settings.
#[tokio::test]
async fn two_clients_setting_different_projects_both_survive() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut first, _) = connect_with_settings(&state).await;
    let (mut second, _) = connect_with_settings(&state).await;

    set(
        &mut first,
        1,
        None,
        None,
        set_base("/work/a", "https://a.atlassian.net"),
        None,
    )
    .await;
    set(
        &mut second,
        1,
        None,
        None,
        set_base("/work/b", "https://b.atlassian.net"),
        None,
    )
    .await;

    let on_disk = stored(store.path()).jira_base_urls;
    assert_eq!(on_disk.len(), 2, "{on_disk:?}");
    assert_eq!(on_disk["/work/a"], base("https://a.atlassian.net"));
    assert_eq!(on_disk["/work/b"], base("https://b.atlassian.net"));
    let (_third, welcomed) = connect_with_settings(&state).await;
    assert_eq!(welcomed.jira_base_urls, on_disk);
}

/// A clear removes only its project's entry; a save naming no project changes none.
#[tokio::test]
async fn a_clear_removes_only_its_project_and_none_changes_nothing() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, _) = connect_with_settings(&state).await;
    set(
        &mut client,
        1,
        None,
        None,
        set_base("/work/a", "https://a.atlassian.net"),
        None,
    )
    .await;
    set(
        &mut client,
        2,
        None,
        None,
        set_base("/work/b", "https://b.atlassian.net"),
        None,
    )
    .await;

    set(&mut client, 3, None, None, None, Some(4321)).await;
    let unchanged = stored(store.path());
    assert_eq!(unchanged.scrollback_lines, 4321);
    assert_eq!(unchanged.jira_base_urls.len(), 2);

    set(&mut client, 4, None, None, clear_base("/work/a"), None).await;
    let on_disk = stored(store.path()).jira_base_urls;
    let keys: Vec<_> = on_disk.keys().cloned().collect();
    assert_eq!(keys, vec!["/work/b".to_owned()]);
    let (_next, welcomed) = connect_with_settings(&state).await;
    assert_eq!(welcomed.jira_base_urls, on_disk);
}

/// The values survive a service restart.
#[tokio::test]
async fn the_values_survive_a_restart() {
    let store = tempfile::tempdir().unwrap();
    let state = service(store.path());
    let (mut client, _) = connect_with_settings(&state).await;
    set(
        &mut client,
        1,
        None,
        None,
        set_base("/work/a", "https://jira.example.com/jira/"),
        None,
    )
    .await;

    let restarted = service(store.path());
    let (_again, welcomed) = connect_with_settings(&restarted).await;
    assert_eq!(
        welcomed.jira_base_urls["/work/a"].as_str(),
        "https://jira.example.com/jira"
    );
}

/// A stored entry that no longer parses is dropped on read and never sent; the valid one is kept.
#[tokio::test]
async fn a_stored_entry_that_does_not_parse_is_never_sent() {
    let store = tempfile::tempdir().unwrap();
    let document = serde_json::json!({
        "settings_version": 4,
        "jira_base_urls": {
            "/work/a": "https://a.atlassian.net",
            "/work/b": "https://user@b.atlassian.net",
        },
    });
    std::fs::write(store.path().join("settings.json"), document.to_string()).unwrap();

    let state = service(store.path());
    let (_client, welcomed) = connect_with_settings(&state).await;

    let keys: Vec<_> = welcomed.jira_base_urls.keys().cloned().collect();
    assert_eq!(keys, vec!["/work/a".to_owned()]);
}

fn git_init(dir: &Path) {
    assert!(Command::new("git")
        .args(["init", "-q"])
        .arg(dir)
        .status()
        .unwrap()
        .success());
}

async fn request(client: &mut conn::Client, msg: ClientMsg, req: u64) {
    client.send(Frame::Control(msg)).await.unwrap();
    match answer(client, req).await {
        DaemonMsg::OperationOk { .. } => {}
        other => panic!("refused: {other:?}"),
    }
}

/// Forgetting a project keeps its entry, which it has again when it is added back
/// (spec edge case *Project forgotten*).
#[tokio::test]
async fn forgetting_a_project_keeps_its_entry() {
    let store = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let a = work.path().join("a");
    std::fs::create_dir_all(&a).unwrap();
    git_init(&a);
    let key = connection_key(&a);
    let state = service(store.path());
    let (mut client, _) = connect_with_settings(&state).await;

    request(
        &mut client,
        ClientMsg::ProjectAdd {
            req: 1,
            path: a.clone(),
        },
        1,
    )
    .await;
    set(
        &mut client,
        2,
        None,
        None,
        set_base(&key, "https://a.atlassian.net"),
        None,
    )
    .await;
    request(
        &mut client,
        ClientMsg::ProjectRemove {
            req: 3,
            path: a.clone(),
        },
        3,
    )
    .await;

    assert_eq!(
        stored(store.path()).jira_base_urls.get(&key),
        Some(&base("https://a.atlassian.net"))
    );
    let (_next, welcomed) = connect_with_settings(&state).await;
    assert!(welcomed.jira_base_urls.contains_key(&key));

    request(
        &mut client,
        ClientMsg::ProjectAdd {
            req: 4,
            path: a.clone(),
        },
        4,
    )
    .await;
    let (_after, welcomed) = connect_with_settings(&state).await;
    assert_eq!(
        welcomed.jira_base_urls.get(&connection_key(&a)),
        Some(&base("https://a.atlassian.net"))
    );
}
