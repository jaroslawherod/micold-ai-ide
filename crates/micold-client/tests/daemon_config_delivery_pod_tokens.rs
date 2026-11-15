//! Scoped GitHub tokens for a pod (#1760, FR-001, FR-002, FR-009): which stored tokens a pod
//! daemon receives, and how their files are read for the pod's startup Secret. Every token is fake.

use std::path::PathBuf;

use micold_client::daemon_config_delivery::{select_tokens, PodGithubTokens};
use micold_client::daemon_config_render::GithubTokenSetting;
use micold_core::daemon_config::RepoPattern;
use micold_core::github::GithubRepo;
use micold_core::kube::manifest::STARTUP_ENV_MOUNT;

fn token(name: &str, repos: &[&str]) -> GithubTokenSetting {
    GithubTokenSetting {
        name: name.into(),
        token_file: PathBuf::from(format!("/pats/{name}")),
        repos: repos
            .iter()
            .map(|r| RepoPattern::parse(r).expect("a pattern"))
            .collect(),
        host: None,
    }
}

fn repo(url: &str) -> GithubRepo {
    GithubRepo::from_remote_url(url).expect("a GitHub remote")
}

fn names(tokens: &[GithubTokenSetting]) -> Vec<&str> {
    tokens.iter().map(|t| t.name.as_str()).collect()
}

/// T006: a token is kept when a pattern names a project (`Exact` or `Owner`, either case), or
/// when it has no filter; Settings order is kept, and two tokens for one repository both stay.
#[test]
fn select_tokens_keeps_matching_and_unfiltered_tokens_in_settings_order() {
    let stored = vec![
        token("other", &["someone/else"]),
        token("owner", &["acme/*"]),
        token("any", &[]),
        token("exact", &["Acme/App"]),
        token("other-owner", &["elsewhere/*"]),
    ];
    let projects = [repo("https://github.com/ACME/app.git")];
    assert_eq!(
        names(&select_tokens(stored, &projects)),
        ["owner", "any", "exact"]
    );
}

/// T006, FR-009: a daemon with no saved project with a GitHub remote receives no token, not even
/// an unfiltered one.
#[test]
fn select_tokens_with_no_project_returns_nothing() {
    let stored = vec![token("any", &[]), token("owner", &["acme/*"])];
    assert!(select_tokens(stored, &[]).is_empty());
}

const FAKE: &str = "github_pat_FAKE_pod_0123456789";

/// A stored token named `name` whose file holds `bytes`, owner-only like the PAT store writes it.
fn stored(dir: &std::path::Path, name: &str, bytes: &[u8]) -> GithubTokenSetting {
    let file = dir.join(name);
    std::fs::write(&file, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    GithubTokenSetting {
        token_file: file,
        ..token(name, &["acme/app"])
    }
}

fn refusal(selected: Vec<GithubTokenSetting>) -> String {
    PodGithubTokens::read(selected)
        .expect_err("the token is refused")
        .to_string()
}

/// T008, FR-001: each selected token's file is read verbatim into a key of the startup Secret,
/// and its entry names that key at the Secret's mount, with the name, filter and host kept.
#[test]
fn read_pairs_each_entry_with_its_secret_key_at_the_startup_mount() {
    let dir = tempfile::tempdir().unwrap();
    let mut forge = stored(dir.path(), "forge", format!("{FAKE}\n").as_bytes());
    forge.host = Some("http://172.18.0.1:8123".into());
    let other = stored(dir.path(), "other", b"github_pat_FAKE_other");
    let read = PodGithubTokens::read(vec![forge, other]).expect("both read");

    assert_eq!(read.entries.len(), 2);
    assert_eq!(read.files.len(), 2);
    for (entry, (key, _)) in read.entries.iter().zip(&read.files) {
        assert_eq!(
            entry.token_file,
            PathBuf::from(format!("{STARTUP_ENV_MOUNT}/{key}"))
        );
    }
    assert_eq!(names(&read.entries), ["forge", "other"]);
    assert_eq!(
        read.entries[0].host.as_deref(),
        Some("http://172.18.0.1:8123")
    );
    assert_eq!(read.entries[0].repos, token("x", &["acme/app"]).repos);
    assert_eq!(
        read.files[0].1.expose(),
        format!("{FAKE}\n"),
        "sent verbatim"
    );
    assert_ne!(read.files[0].0, read.files[1].0, "one key per token");
}

/// Review A F1: a token's key is its (lower-case) name, never its position, so a key keeps
/// naming the same token when another is deleted or the order changes on a kept pod.
#[test]
fn a_token_keeps_its_key_whatever_its_position() {
    let dir = tempfile::tempdir().unwrap();
    let both = PodGithubTokens::read(vec![
        stored(dir.path(), "Forge", FAKE.as_bytes()),
        stored(dir.path(), "other", b"github_pat_FAKE_other"),
    ])
    .unwrap();
    let alone = PodGithubTokens::read(vec![stored(dir.path(), "other", b"x")]).unwrap();
    let keys = |read: &PodGithubTokens| -> Vec<String> {
        read.files.iter().map(|(key, _)| key.clone()).collect()
    };
    assert_eq!(keys(&both), ["github-token-forge", "github-token-other"]);
    assert_eq!(keys(&alone), ["github-token-other"]);
}

/// Review A F1: a name that is no token name (so no Secret key) is refused naming the token.
#[test]
fn a_token_with_an_invalid_name_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut bad = stored(dir.path(), "ok", FAKE.as_bytes());
    bad.name = "bad name/..".into();
    let error = refusal(vec![bad]);
    assert!(
        error.starts_with("cannot deliver the GitHub token \"bad name/..\" to the pod: "),
        "{error}"
    );
}

/// T008, FR-002: nothing that prints a read set shows a token's value.
#[test]
fn a_read_set_never_prints_a_value() {
    let dir = tempfile::tempdir().unwrap();
    let read = PodGithubTokens::read(vec![stored(dir.path(), "forge", FAKE.as_bytes())]).unwrap();
    let printed = format!("{read:?}");
    assert!(!printed.contains(FAKE), "{printed}");
}

/// T008: an empty, non-text, oversize or missing token file stops the start naming the token.
#[test]
fn an_unusable_token_file_is_refused_naming_the_token() {
    let dir = tempfile::tempdir().unwrap();
    let cases = [
        stored(dir.path(), "empty", b""),
        stored(dir.path(), "binary", &[0xff, 0xfe, 0x00]),
        stored(dir.path(), "huge", &vec![b'a'; 512 * 1024 + 1]),
        GithubTokenSetting {
            token_file: dir.path().join("missing"),
            ..token("missing", &[])
        },
    ];
    for case in cases {
        let name = case.name.clone();
        let error = refusal(vec![case]);
        assert!(
            error.starts_with(&format!(
                "cannot deliver the GitHub token \"{name}\" to the pod: "
            )),
            "{error}"
        );
    }
}

/// T008: on Unix a token file another user can read is refused, as the store never writes one.
#[cfg(unix)]
#[test]
fn a_token_file_readable_by_others_is_refused_naming_the_token() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    for mode in [0o640, 0o604] {
        let loose = stored(dir.path(), "loose", FAKE.as_bytes());
        std::fs::set_permissions(&loose.token_file, std::fs::Permissions::from_mode(mode)).unwrap();
        let error = refusal(vec![loose]);
        assert!(
            error.starts_with("cannot deliver the GitHub token \"loose\" to the pod: "),
            "{error}"
        );
        assert!(!error.contains(FAKE), "{error}");
    }
}
