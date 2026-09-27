//! What the sandbox can and cannot see (feature 027, FR-004a/b/c, FR-006, FR-007).
//!
//! The load-bearing rule of the whole feature is rule M-1: only what the mount set names is
//! mounted. Everything else in this specification is configuration or presentation on top of it, so
//! this file is where a regression would matter most and where it would be least visible — a
//! sandbox that quietly mounts one extra directory still starts, still runs sessions, and still
//! looks exactly like a working sandbox.

use micold_core::sandbox::pathmap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use micold_core::sandbox::{
    onboarding_done, CredentialLayout, CredentialShare, MountSet, SandboxProfile, SecretMount,
};

fn layout() -> CredentialLayout {
    CredentialLayout::conventional(Path::new("/home/u"), Some(Path::new("/run/user/1000/ssh")))
}

fn secret() -> SecretMount {
    SecretMount {
        host: PathBuf::from("/run/user/1000/micold/sandbox.token"),
        container: PathBuf::from("/run/micold/token"),
    }
}

fn build(profile: &SandboxProfile) -> MountSet {
    MountSet::build(
        &[PathBuf::from("/home/u/projects/micold")],
        profile,
        &layout(),
        std::path::PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        secret(),
    )
}

/// FR-004a: the default shares nothing. The one default in this feature that is a security property
/// rather than a convenience — an upgrade must never opt anyone in.
#[test]
fn the_default_profile_mounts_no_credentials() {
    let mounts = build(&SandboxProfile::default());
    assert!(mounts.credentials.is_empty());

    // And the host paths it *can* reach are exactly the project, the daemon's own state
    // directory, the sandbox's own home under it, and the token. Four, and no fifth: the count is
    // the assertion, because a convenience mount added later would pass every other check here.
    //
    // The home is the application's own directory (FR-004d), which is why it appears in a test
    // about sharing *nothing*: it is reachable from the sandbox but is not the user's, and the
    // assertion below that it lives under the state directory is what keeps those apart.
    let paths: Vec<String> = mounts
        .host_paths()
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    assert_eq!(paths.len(), 4, "reachable host paths: {paths:?}");
    assert!(paths.iter().any(|p| p.ends_with("projects/micold")));
    assert!(paths.iter().any(|p| p.ends_with("micold-ai-ide")));
    assert!(paths.iter().any(|p| p.ends_with("sandbox.token")));
    // Spelled with the platform's own separator, so this reads the same on Windows, where the
    // host half of a mount keeps its native spelling (`pathmap` maps only the container half).
    let home_under_state = Path::new("micold-ai-ide")
        .join("sandbox-home")
        .display()
        .to_string();
    assert!(
        paths.iter().any(|p| p.ends_with(&home_under_state)),
        "reachable host paths: {paths:?}"
    );

    // The one thing this mount must never become: the user's home shared back in. It is mounted
    // *at* the home path, so only the host half distinguishes the two.
    assert_eq!(
        mounts.home.container,
        pathmap::map_for(Path::new("/home/u"), cfg!(windows)),
        "the sandbox's home belongs at the user's home path, under this platform's mapping"
    );
    assert!(
        !paths.iter().any(|p| p == "/home/u"),
        "the sandbox's home must shadow the user's, never share it: {paths:?}"
    );
}

/// FR-004b: each opt-in adds exactly its own mount and no other. Checked one at a time, because an
/// opt-in that pulled in a second path would be invisible when they are all enabled together.
#[test]
fn each_opt_in_adds_exactly_one_mount() {
    for share in CredentialShare::ALL {
        let profile = SandboxProfile {
            credentials: BTreeSet::from([share]),
            ..SandboxProfile::default()
        };
        let mounts = build(&profile);
        assert_eq!(
            mounts.credentials.len(),
            1,
            "{share:?} added more than itself"
        );
        assert_eq!(mounts.credentials[0].share, share);
    }
}

/// FR-004c: the view renders what is shared *from the set*, so each active share must be
/// individually identifiable rather than collapsed into a count.
#[test]
fn every_active_share_is_individually_identifiable() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from(CredentialShare::ALL),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);

    let shared: BTreeSet<CredentialShare> = mounts.credentials.iter().map(|c| c.share).collect();
    assert_eq!(shared, BTreeSet::from(CredentialShare::ALL));
    for c in &mounts.credentials {
        assert!(
            !c.share.label().is_empty(),
            "{:?} has no label to show",
            c.share
        );
    }
}

/// An opt-in whose path the host layout does not know is skipped, not substituted.
///
/// The edge case the spec names: a credential opt-in enabled while the item it shares is absent —
/// no authentication agent running, or the socket it named has gone. Mounting *something* nearby
/// would be worse than mounting nothing, because the user would believe the opt-in worked.
#[test]
fn an_opt_in_with_no_known_path_is_skipped_rather_than_substituted() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::SshAgent]),
        ..SandboxProfile::default()
    };
    let mounts = MountSet::build(
        &[],
        &profile,
        // No agent socket: the user has one enabled, and there is nothing to enable it with.
        &CredentialLayout::conventional(Path::new("/home/u"), None),
        std::path::PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        secret(),
    );
    assert!(mounts.credentials.is_empty());
}

/// FR-006/FR-007: the mount set holds registered projects and nothing near them.
#[test]
fn only_registered_projects_are_mounted() {
    let profile = SandboxProfile::default();
    let mounts = MountSet::build(
        &[
            PathBuf::from("/home/u/projects/a"),
            PathBuf::from("/home/u/projects/b"),
        ],
        &profile,
        &layout(),
        std::path::PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        secret(),
    );
    assert_eq!(mounts.projects.len(), 2);

    // Not the parent that contains them both, and not the home directory above it. Mounting either
    // would be convenient and would defeat the feature.
    let hosts: Vec<String> = mounts
        .projects
        .iter()
        .map(|p| p.host.display().to_string())
        .collect();
    assert!(!hosts.iter().any(|h| h == "/home/u"));
    assert!(!hosts.iter().any(|h| h == "/home/u/projects"));
}

/// The runtime's own control socket is never in the set. A sandbox that can drive its own runtime
/// can start an unconfined container, which is the whole boundary undone in one command.
#[test]
fn the_runtimes_control_socket_is_never_reachable() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from(CredentialShare::ALL),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    for path in mounts.host_paths() {
        let p = path.display().to_string();
        assert!(!p.contains("docker.sock"), "{p} is reachable");
        assert!(!p.contains("podman.sock"), "{p} is reachable");
    }
}

/// Credentials keep their own absolute paths inside the container, for the same reason projects do:
/// the tools that read them look where they always are.
#[test]
fn credentials_appear_at_the_paths_their_tools_expect() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::GitConfig]),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    let c = &mounts.credentials[0];
    if cfg!(not(windows)) {
        assert_eq!(c.host, c.container);
        assert!(c.container.ends_with(".gitconfig"));
    }
}

/// FR-004e (BUG-006): the AI CLI sign-in share is the token file, not the CLI's directory.
///
/// `~/.claude` also holds the CLI's session store, which a read-only share makes unwritable, and
/// settings and hooks the host's own `claude` runs, which a writable share hands to the session.
/// The token file is the only part of it the sandbox needs.
#[test]
fn the_ai_cli_sign_in_share_is_the_token_file_not_the_cli_directory() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    assert_eq!(
        mounts.credentials[0].host,
        Path::new("/home/u/.claude/.credentials.json"),
        "the sign-in share must name the token file and nothing around it"
    );
}

/// Research R11 (BUG-006): a credential mounted inside the sandbox's home needs its parent there.
///
/// The runtime creates a missing mount target's parents as root. Left to it, `<sandbox-home>/.claude`
/// comes out root-owned, and `claude` cannot write its sessions again, which is the bug this share
/// was narrowed to fix. So the mount set names the directory, and the bring-up creates it first.
#[test]
fn a_shared_sign_in_names_its_directory_in_the_sandbox_home_to_create() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    assert_eq!(
        mounts.home_dirs_to_create(),
        vec![mounts.home.host.join(".claude")],
        "the sign-in's parent must exist, user-owned, before the runtime mounts the file into it"
    );
}

/// BUG-006, review finding F3: the sign-in's mount *target* in the sandbox home is named too.
///
/// Naming the directory is half the answer. The runtime creates a missing target file as well, and
/// as root, and that file survives the container being removed — so the next sandbox, with the
/// share turned off again, hands its `claude` a token file it cannot write. That is the same bug
/// the share was narrowed to fix, approached from the other side.
#[test]
fn a_shared_sign_in_names_its_file_in_the_sandbox_home_to_create() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    assert_eq!(
        mounts.home_files_to_create(),
        vec![mounts.home.host.join(".claude").join(".credentials.json")],
        "the sign-in's target must exist, user-owned, before the runtime mounts onto it"
    );
}

/// BUG-007 (FR-004f): with the sign-in shared, the sandbox home's `.claude.json` is where the
/// finished first-run setup is recorded. `claude` reads that file, not the token, to decide whether
/// to run its setup — and the setup asks for a login method over a perfectly good shared token.
#[test]
fn a_shared_sign_in_names_the_sandbox_homes_onboarding_record() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    assert_eq!(
        mounts.onboarding_record(),
        Some(mounts.home.host.join(".claude.json"))
    );
}

/// Without a shared token, `claude`'s own setup is how the user signs in, so nothing is recorded.
/// Every other share is held to this too: none of them is a sign-in.
#[test]
fn without_a_shared_sign_in_no_onboarding_record_is_named() {
    for share in [
        CredentialShare::GitConfig,
        CredentialShare::SshAgent,
        CredentialShare::GitCredentials,
    ] {
        let profile = SandboxProfile {
            credentials: BTreeSet::from([share]),
            ..SandboxProfile::default()
        };
        assert_eq!(build(&profile).onboarding_record(), None, "{share:?}");
    }
    assert_eq!(build(&SandboxProfile::default()).onboarding_record(), None);
}

/// A token the host does not have is dropped before the mount set is built, and then there is no
/// sign-in to skip the setup for.
#[test]
fn a_sign_in_share_with_no_token_names_no_onboarding_record() {
    let mut layout = layout();
    layout.ai_cli_auth = None;
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
        ..SandboxProfile::default()
    };
    let mounts = MountSet::build(
        &[],
        &profile,
        &layout,
        PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        secret(),
    );
    assert_eq!(mounts.onboarding_record(), None);
}

fn parsed(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("the merge writes JSON")
}

/// No file yet: the record is written holding the one key.
#[test]
fn the_onboarding_merge_writes_the_key_when_there_is_no_file() {
    let written = onboarding_done(None).expect("an absent file is written");
    assert_eq!(
        parsed(&written),
        serde_json::json!({ "hasCompletedOnboarding": true })
    );
}

/// A file `claude` already wrote keeps everything in it; only the key is added. This is the file
/// a sandbox that already asked once leaves behind.
#[test]
fn the_onboarding_merge_keeps_every_other_key() {
    let existing = r#"{"firstStartVersion":"2.1.283","projects":{"/p":{"hasTrustDialogAccepted":true}},"hasCompletedOnboarding":false}"#;
    let written = onboarding_done(Some(existing)).expect("a missing `true` is written");
    assert_eq!(
        parsed(&written),
        serde_json::json!({
            "firstStartVersion": "2.1.283",
            "projects": { "/p": { "hasTrustDialogAccepted": true } },
            "hasCompletedOnboarding": true
        })
    );
}

/// Already recorded: nothing to write, so a running `claude`'s file is not rewritten under it.
#[test]
fn the_onboarding_merge_leaves_a_recorded_file_alone() {
    assert_eq!(
        onboarding_done(Some(r#"{"hasCompletedOnboarding":true,"x":1}"#)),
        None
    );
}

/// Text that is not a JSON object is `claude`'s to deal with. Replacing it would lose whatever it
/// was; the application writes into another tool's file only by adding one key to it.
#[test]
fn the_onboarding_merge_never_replaces_a_file_it_cannot_read_as_an_object() {
    for text in ["", "not json", "[1,2]", "true", "{\"truncated\":"] {
        assert_eq!(onboarding_done(Some(text)), None, "{text:?}");
    }
}

/// FR-004e (T208): no credential mount names a *directory* under the sandbox home.
///
/// Held over every share rather than over the sign-in alone, because the cost is paid by whichever
/// share is added next. Two things are checked of each mount that lands in the home: that its
/// parent is a directory the bring-up creates, so the runtime never has to create one as root, and
/// that it does not contain the AI CLI's own directory there. That directory is where a sandboxed
/// session writes its transcript, and a mount over it is BUG-006 exactly: read-only it cannot be
/// written at all, and either way the host's copy shadows the sandbox's.
#[test]
fn no_credential_mounts_a_directory_under_the_sandbox_home() {
    // The AI CLI's own directory, spelled as `provider.rs` spells it. Inside the sandbox home this
    // is where a session's transcript is written.
    let cli_dir = Path::new(".claude");
    let profile = SandboxProfile {
        credentials: BTreeSet::from(CredentialShare::ALL),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    let dirs = mounts.home_dirs_to_create();

    for c in &mounts.credentials {
        let Ok(relative) = c.container.strip_prefix(&mounts.home.container) else {
            // Outside the home: the agent socket, and anything else the host keeps elsewhere.
            continue;
        };
        assert!(
            !cli_dir.starts_with(relative),
            "{:?} is mounted over {}, the directory a sandboxed session writes its transcript in",
            c.share,
            relative.display()
        );
        // Its parent is either the sandbox home itself, which the bring-up creates before
        // anything else, or one of the directories this set asks for.
        let target = mounts.home.host.join(relative);
        let parent = target.parent().expect("a mount target has a parent");
        assert!(
            parent == mounts.home.host || dirs.contains(&parent.to_path_buf()),
            "{:?} is mounted into {}, which nothing creates before the runtime runs",
            c.share,
            parent.display()
        );
    }
}

/// Daemon state is the host's own data directory, bind-mounted.
///
/// It is deliberately *not* a runtime-managed volume: the client has to read `projects.json` to
/// know what to mount before the sandbox exists, and inside a volume that file is unreachable from
/// the host. See `StateMount`'s doc comment for why this satisfies FR-011 anyway.
#[test]
fn daemon_state_is_the_hosts_own_data_directory() {
    let mounts = build(&SandboxProfile::default());
    assert!(mounts.state.host.ends_with("micold-ai-ide"));
    assert_eq!(
        mounts.state.container,
        std::path::PathBuf::from(micold_core::sandbox::STATE_CONTAINER_DIR)
    );
    // It is reachable, and it must be: that is the point.
    assert!(mounts
        .host_paths()
        .iter()
        .any(|p| p.ends_with("micold-ai-ide")));
}
