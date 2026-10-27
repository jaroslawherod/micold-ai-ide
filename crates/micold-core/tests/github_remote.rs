//! Which GitHub repository a project's remotes name (feature 034, contracts/remote-list-rpc.md §4,
//! research R5).

use micold_core::git::GitRemote;
use micold_core::github::{choose_remote, GithubRepo, RemoteChoice};

fn remote(name: &str, url: &str) -> GitRemote {
    GitRemote {
        name: name.into(),
        url: url.into(),
    }
}

fn repo_of(url: &str) -> Option<String> {
    GithubRepo::from_remote_url(url).map(|r| r.to_string())
}

#[test]
fn accepted_url_forms() {
    for url in [
        "https://github.com/o/r",
        "https://github.com/o/r.git",
        "https://github.com/o/r/",
        "https://github.com/o/r.git/",
        "http://github.com/o/r.git",
        "git@github.com:o/r.git",
        "git@github.com:o/r",
        "ssh://git@github.com/o/r.git",
        "ssh://git@github.com:22/o/r.git",
        "ssh://git@ssh.github.com:443/o/r.git",
        "git://github.com/o/r",
        "https://GITHUB.COM/o/r.git",
    ] {
        assert_eq!(
            repo_of(url).as_deref(),
            Some("o/r"),
            "`{url}` is a github.com remote naming o/r"
        );
    }
    let repo = GithubRepo::from_remote_url("git@github.com:Micold-Org/my.repo-1.git").unwrap();
    assert_eq!(
        (repo.owner(), repo.name()),
        ("Micold-Org", "my.repo-1"),
        "owner and name are kept as written, with only `.git` stripped"
    );
}

#[test]
fn userinfo_is_discarded() {
    assert_eq!(
        repo_of("https://user@github.com/o/r").as_deref(),
        Some("o/r"),
        "a user name in the URL is not part of the repository"
    );
    let token = "ghp_SECRET123";
    let repo =
        GithubRepo::from_remote_url(&format!("https://x-access-token:{token}@github.com/o/r"))
            .expect("a token-in-URL remote is still a GitHub remote");
    assert_eq!(
        repo.to_string(),
        "o/r",
        "the token is not part of the repository"
    );
    assert!(
        !format!("{repo:?}").contains(token),
        "the token never reaches the value, so it can never be shown or sent (FR-022)"
    );
}

#[test]
fn non_github_urls_are_rejected() {
    for url in [
        "https://www.github.com/o/r",
        "https://github.example.com/o/r",
        "git@github.example.com:o/r.git",
        "https://gitlab.com/o/r.git",
        "git@bitbucket.org:o/r.git",
        "/home/me/src/r",
        "../r.git",
        "file:///home/me/r.git",
        "gh:o/r",
        "https://github.com/o",
        "https://github.com//r",
        "https://github.com/o/r/issues",
        "",
    ] {
        assert_eq!(repo_of(url), None, "`{url}` is not a github.com repository");
    }
}

#[test]
fn origin_wins_when_on_github() {
    let remotes = [
        remote("upstream", "https://github.com/up/r"),
        remote("origin", "git@github.com:me/r.git"),
    ];
    match choose_remote(&remotes) {
        RemoteChoice::Github { remote, repo } => {
            assert_eq!(
                (remote.as_str(), repo.to_string().as_str()),
                ("origin", "me/r"),
                "origin on GitHub is chosen over an earlier GitHub remote"
            );
        }
        other => panic!("origin on GitHub is chosen over an earlier GitHub remote, got {other:?}"),
    }
}

#[test]
fn first_github_remote_otherwise() {
    let origin_elsewhere = [
        remote("origin", "https://gitlab.com/me/r.git"),
        remote("upstream", "https://github.com/up/r"),
    ];
    assert!(
        matches!(choose_remote(&origin_elsewhere),
            RemoteChoice::Github { ref remote, .. } if remote == "upstream"),
        "origin not on GitHub: the GitHub remote is chosen"
    );

    let no_origin = [
        remote("fork", "https://github.com/me/r"),
        remote("upstream", "https://github.com/up/r"),
    ];
    match choose_remote(&no_origin) {
        RemoteChoice::Github { remote, repo } => {
            assert_eq!(
                (remote.as_str(), repo.to_string().as_str()),
                ("fork", "me/r"),
                "without origin, the first GitHub remote in config order"
            );
        }
        other => panic!("expected a GitHub remote, got {other:?}"),
    }
}

#[test]
fn no_github_remote() {
    assert_eq!(
        choose_remote(&[]),
        RemoteChoice::NoGithubRemote,
        "a repository without remotes has no GitHub remote"
    );
    assert_eq!(
        choose_remote(&[
            remote("origin", "https://gitlab.com/me/r.git"),
            remote("mirror", "https://github.example.com/me/r"),
        ]),
        RemoteChoice::NoGithubRemote,
        "remotes on other hosts, a look-alike host included, are no GitHub remote"
    );
}

#[test]
fn managed_user_owner_with_underscore() {
    // Enterprise Managed Users on github.com have logins like `jdoe_acme`.
    assert_eq!(
        repo_of("git@github.com:jdoe_acme/tool.git").as_deref(),
        Some("jdoe_acme/tool"),
        "an underscore in the owner is kept"
    );
}

fn stub_ssh_g(host: &str) -> Option<String> {
    match host {
        "github-micold" => Some("github.com".into()),
        "gh-443" => Some("ssh.github.com".into()),
        "gitlab-alias" => Some("gitlab.com".into()),
        _ => None,
    }
}

fn resolved_repo(url: &str) -> Option<String> {
    let remotes = vec![remote("origin", url)];
    let resolved = micold_core::github::resolve_ssh_aliases_with(remotes, &stub_ssh_g);
    match choose_remote(&resolved) {
        RemoteChoice::Github { repo, .. } => Some(repo.to_string()),
        RemoteChoice::NoGithubRemote => None,
    }
}

#[test]
fn ssh_alias_resolving_to_github_is_a_github_remote() {
    for url in [
        "git@github-micold:o/r.git",
        "github-micold:o/r",
        "ssh://git@github-micold/o/r.git",
        "ssh://git@github-micold:22/o/r.git",
        "git@gh-443:o/r.git",
    ] {
        assert_eq!(resolved_repo(url).as_deref(), Some("o/r"), "`{url}`");
    }
}

#[test]
fn ssh_alias_elsewhere_or_unresolved_is_not_github() {
    for url in [
        "git@gitlab-alias:o/r.git",
        "git@unknown-alias:o/r.git",
        // Only SSH remotes are resolved: an https host is never run through `ssh -G`.
        "https://github-micold/o/r.git",
        "git@-oProxyCommand=x:o/r.git",
    ] {
        assert_eq!(resolved_repo(url), None, "`{url}`");
    }
}

#[test]
fn resolver_runs_only_for_non_github_ssh_hosts() {
    let calls = std::cell::RefCell::new(Vec::new());
    let spy = |h: &str| {
        calls.borrow_mut().push(h.to_string());
        None
    };
    let remotes = [
        "git@github.com:o/r",
        "https://x.example/o/r",
        "git@alias:o/r",
        "/tmp/repo",
    ]
    .iter()
    .map(|u| remote("origin", u))
    .collect();
    let out = micold_core::github::resolve_ssh_aliases_with(remotes, &spy);
    assert_eq!(*calls.borrow(), vec!["alias".to_string()]);
    assert_eq!(
        out[2].url, "git@alias:o/r",
        "an unresolved alias keeps its URL"
    );
}

#[test]
fn hostname_is_read_from_ssh_g_output() {
    use micold_core::github::hostname_from_ssh_g;
    let out = "user git\nhostname GitHub.com\nport 22\n";
    assert_eq!(hostname_from_ssh_g(out).as_deref(), Some("github.com"));
    assert_eq!(hostname_from_ssh_g("port 22\n"), None);
    assert_eq!(hostname_from_ssh_g(""), None);
}
