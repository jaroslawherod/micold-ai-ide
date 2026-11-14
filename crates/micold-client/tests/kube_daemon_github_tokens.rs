//! #1760 T013 (US1, FR-001, FR-002, FR-008): a pod start with the `github_tokens` opt-in on puts
//! the stored tokens that match the daemon's saved projects in the startup Secret and names them
//! in the pod's `daemon.toml`; off, it reads no token at all. Every token is fake.

use crate::kube_daemon_common;

use kube_daemon_common::*;
use micold_client::daemon::{kube_attempt, PodTokenSource};
use micold_core::daemon_config::RepoPattern;
use micold_core::github::GithubRepo;
use micold_core::kube::names::ResourceNames;
use micold_core::kube::settings::KubeSettings;
use micold_core::kube::start::DriftChoice;
use micold_core::kube::ObjectKind;
use micold_core::scoped_credential::pat_store::{PatStore, StoredPat};
use micold_core::scoped_credential::SecretToken;

const SENTINEL: &str = "github_pat_FAKE_SENTINEL_1760";
const FORGE: &str = "http://172.18.0.1:8123";

fn names() -> ResourceNames {
    ResourceNames::new(CONTEXT, NS, NAME)
}

/// A PAT store holding `forge` for `owner/repo` and `elsewhere` for another owner.
fn store() -> (tempfile::TempDir, PatStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = PatStore::new(dir.path());
    let pat = |repo: &str| StoredPat {
        token: SecretToken::new(SENTINEL),
        expires_at: 4_000_000_000,
        repos: vec![RepoPattern::parse(repo).unwrap()],
    };
    store.put("forge", &pat("owner/repo")).unwrap();
    store.put("elsewhere", &pat("other/*")).unwrap();
    (dir, store)
}

/// Widens the `forge` token file to group and other: listed by the store, refused for a pod.
fn loosen(store: &PatStore) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file = store.token_file("forge").unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    #[cfg(not(unix))]
    let _ = store;
}

fn projects() -> Vec<GithubRepo> {
    vec![GithubRepo::from_remote_url("https://github.com/owner/repo.git").unwrap()]
}

fn opted_in(on: bool) -> KubeSettings {
    KubeSettings {
        github_tokens: on,
        ..settings()
    }
}

async fn start(client: &Client, settings: &KubeSettings) -> (Result<(), String>, String) {
    let (sink, seen) = sink();
    let result = kube_attempt(
        &client.env,
        NAME,
        settings,
        None,
        &Default::default(),
        &projects(),
        DriftChoice::Ask,
        &sink,
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string());
    let log = format!("{:?}", seen.lock().unwrap());
    (result, log)
}

fn config(cluster: &FakeClusterRef) -> String {
    let map = cluster
        .manifest(NS, ObjectKind::ConfigMap, &names().config_map())
        .expect("the config map");
    map["data"]["daemon.toml"].as_str().unwrap().to_string()
}

type FakeClusterRef = std::sync::Arc<micold_core::kube::fake::FakeCluster>;

fn token_keys(cluster: &FakeClusterRef) -> Vec<String> {
    let keys: Vec<String> = cluster
        .manifest(NS, ObjectKind::Secret, &names().startup_secret())
        .and_then(|secret| {
            secret["stringData"]
                .as_object()
                .map(|data| data.keys().cloned().collect())
        })
        .unwrap_or_default();
    keys.into_iter()
        .filter(|key| key.starts_with("github-token-"))
        .collect()
}

/// Off (the default): the store is not read at all, here proved (on Unix) by a file the read
/// would refuse, and the pod gets no token key and no `[[github.tokens]]` entry.
#[tokio::test]
async fn with_the_opt_in_off_no_token_is_read_or_delivered() {
    let (dir, store) = store();
    loosen(&store);
    let cluster = cluster();
    let mut client = client(&cluster, welcoming());
    client.env.tokens = PodTokenSource {
        pat_root: Some(dir.path().to_path_buf()),
        host: Some(FORGE.into()),
    };
    let (result, _) = start(&client, &opted_in(false)).await;
    result.expect("the start connects");
    assert!(token_keys(&cluster).is_empty());
    assert!(!config(&cluster).contains("[[github.tokens]]"));
}

/// On: the matching token's file is a key of the startup Secret, the entry names it at the
/// Secret's mount with the source's host, and its value is nowhere else (FR-002).
#[tokio::test]
async fn with_the_opt_in_on_the_matching_token_reaches_the_secret_and_daemon_toml() {
    let (dir, store) = store();
    let stored = std::fs::read_to_string(store.token_file("forge").unwrap()).unwrap();
    let cluster = cluster();
    let mut client = client(&cluster, welcoming());
    client.env.tokens = PodTokenSource {
        pat_root: Some(dir.path().to_path_buf()),
        host: Some(FORGE.into()),
    };
    let (result, log) = start(&client, &opted_in(true)).await;
    result.expect("the start connects");

    assert_eq!(token_keys(&cluster), ["github-token-0"]);
    let secret = cluster
        .manifest(NS, ObjectKind::Secret, &names().startup_secret())
        .unwrap();
    assert_eq!(secret["stringData"]["github-token-0"], stored.as_str());
    let file = config(&cluster);
    assert!(
        file.contains(&format!(
            "[[github.tokens]]\nname = \"forge\"\ntoken_file = \"/run/micold/secrets/startup/github-token-0\"\nrepos = [\"owner/repo\"]\nhost = \"{FORGE}\"\n"
        )),
        "{file}"
    );
    assert!(!file.contains("elsewhere"), "{file}");
    assert!(!file.contains(SENTINEL), "{file}");
    let pod = cluster
        .manifest(NS, ObjectKind::Pod, &names().pod())
        .expect("the pod")
        .to_string();
    assert!(!pod.contains(SENTINEL), "{pod}");
    assert!(!log.contains(SENTINEL), "{log}");
    let calls = cluster.calls().join("\n");
    assert!(!calls.contains(SENTINEL), "{calls}");
}

/// On, with a token file the pod cannot take: the start fails naming the token, before any call
/// reaches the cluster.
#[cfg(unix)]
#[tokio::test]
async fn a_bad_token_fails_the_start_before_any_cluster_call() {
    let (dir, store) = store();
    loosen(&store);
    let cluster = cluster();
    let mut client = client(&cluster, welcoming());
    client.env.tokens = PodTokenSource {
        pat_root: Some(dir.path().to_path_buf()),
        host: None,
    };
    let (result, _) = start(&client, &opted_in(true)).await;
    let reason = result.expect_err("the start fails");
    assert!(
        reason.starts_with("cannot deliver the GitHub token \"forge\" to the pod: "),
        "{reason}"
    );
    assert!(cluster.calls().is_empty(), "{:?}", cluster.calls());
}
