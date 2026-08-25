//! The sandbox handshake: the shared secret and the build fingerprint (feature 027, protocol v6).
//!
//! `contracts/protocol-delta.md`'s obligations P-1 … P-6. The unit-level rules live beside the code
//! in `protocol/auth.rs` and `protocol/handshake.rs`; what is here is the *integration* claim —
//! that a token generated the way the client generates one, written the way the client writes it,
//! and read the way the daemon reads it, ends up authenticating, and that nothing along that path
//! leaks it.

use micold_core::protocol::auth::{host_token_path, Token, CONTAINER_TOKEN_PATH, TOKEN_PATH_ENV};
use micold_core::protocol::handshake::{evaluate_introduction, Expectation, Introduction};
use micold_core::protocol::messages::RefusalReason;
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::sandbox::image::{ImageSource, ImageSourceKind};

fn introduction(token: Option<&Token>) -> Introduction {
    Introduction {
        protocol_version: PROTOCOL_VERSION,
        schema_hash: SCHEMA_HASH,
        package_version: PACKAGE_VERSION.to_string(),
        build: "test-client".to_string(),
        auth_token: token.map(|t| t.as_str().to_string()),
        fingerprint: BUILD_FINGERPRINT.to_string(),
        require_fingerprint_match: false,
    }
}

fn expecting(token: Option<Token>) -> Expectation {
    Expectation {
        token,
        build: "test-daemon".to_string(),
        image: "micold-daemon:dev".to_string(),
    }
}

/// P-1, end to end: the client writes a token, the daemon reads that file, and the handshake
/// accepts. Everything in between is the path a real sandbox start takes.
#[test]
fn a_token_written_by_the_client_authenticates_when_read_by_the_daemon() {
    let dir = tempfile::tempdir().unwrap();
    let path = host_token_path(dir.path());

    let issued = Token::generate();
    issued.write_to(&path).unwrap();

    let adopted = Token::read_from(&path).unwrap();
    assert!(evaluate_introduction(&introduction(Some(&issued)), &expecting(Some(adopted))).is_ok());
}

/// P-1: a token from a *different* sandbox start does not authenticate. This is the case the whole
/// mechanism exists for — another local process on the same loopback port.
#[test]
fn a_token_from_another_start_is_refused() {
    let mine = Token::generate();
    let theirs = Token::generate();
    assert_eq!(
        evaluate_introduction(&introduction(Some(&theirs)), &expecting(Some(mine))),
        Err(RefusalReason::AuthRejected)
    );
}

/// P-1: presenting nothing is refused exactly as presenting the wrong thing is.
#[test]
fn an_absent_token_is_refused_indistinguishably_from_a_wrong_one() {
    let expected = Token::generate();
    let absent = evaluate_introduction(&introduction(None), &expecting(Some(expected.clone())));
    let wrong = evaluate_introduction(
        &introduction(Some(&Token::generate())),
        &expecting(Some(expected)),
    );
    assert_eq!(absent, wrong, "the two refusals must be the same value");
    assert_eq!(absent, Err(RefusalReason::AuthRejected));
}

/// P-3: the token must not be reconstructible from anything the system writes down.
///
/// The three places it could leak are the `Debug` form (which reaches logs), the settings document
/// (which users copy between machines and paste into bug reports), and the container's argument
/// vector (which `docker inspect` shows to anyone). The first is checked here; the second is
/// checked in `settings_roundtrip.rs`; the third in `sandbox::argv`'s mount-set test, because the
/// token reaches the container as a *file*, not an argument.
#[test]
fn the_token_is_not_recoverable_from_its_debug_form() {
    let t = Token::generate();
    let shown = format!("{t:?}");
    assert!(!shown.contains(t.as_str()));
    // Not even a prefix: a partial leak is still a leak when the search space shrinks by it.
    assert!(!shown.contains(&t.as_str()[..8]));
}

/// P-4: the asymmetry. Only a locally built image refuses on a fingerprint mismatch.
#[test]
fn a_fingerprint_mismatch_refuses_a_local_build_and_accepts_a_released_one() {
    let local = ImageSource {
        kind: ImageSourceKind::LocalBuild,
        reference: "micold-daemon:dev".to_string(),
        path: None,
    };
    let released = ImageSource::default();

    // The client decides, because the client is what knows where the image came from.
    let mut intro = introduction(None);
    intro.fingerprint = "0000000000000000".to_string();

    intro.require_fingerprint_match = released.refuses_fingerprint_mismatch();
    assert!(
        evaluate_introduction(&intro, &expecting(None)).is_ok(),
        "a released client and daemon are built separately and legitimately differ"
    );

    intro.require_fingerprint_match = local.refuses_fingerprint_mismatch();
    match evaluate_introduction(&intro, &expecting(None)) {
        Err(RefusalReason::StaleDevImage { image, .. }) => assert_eq!(image, "micold-daemon:dev"),
        other => panic!("expected StaleDevImage, got {other:?}"),
    }
}

/// P-4: a matching fingerprint is accepted even under the strict policy — the check must not refuse
/// a correctly rebuilt image, or `mise run image` would never produce a usable one.
#[test]
fn a_matching_fingerprint_is_accepted_under_the_strict_policy() {
    let mut intro = introduction(None);
    intro.require_fingerprint_match = true;
    assert!(evaluate_introduction(&intro, &expecting(None)).is_ok());
}

/// P-5: each refusal renders a distinct remedy. A catalogue where two failures read the same is a
/// catalogue that does not help (FR-034).
#[test]
fn the_two_new_refusals_are_distinct_values() {
    let auth = RefusalReason::AuthRejected;
    let stale = RefusalReason::StaleDevImage {
        client_fingerprint: "a".into(),
        daemon_fingerprint: "b".into(),
        image: "micold-daemon:dev".into(),
    };
    assert_ne!(auth, stale);
    // `AuthRejected` carries no detail at all, by design: see its doc comment.
    assert_eq!(auth, RefusalReason::AuthRejected);
}

/// P-6: the version moved, and it moved *with* these fields rather than ahead of them.
///
/// Six was the sandbox handshake (the auth token and the stale-dev-image refusal). Seven is
/// `ClientMsg::RepoRootQuery` / `OperationResult::RepoRoot`, which moves the open-project gate to
/// whichever side can actually see the folder (feature 027, research R2 part 2).
///
/// The literal is the point. `SCHEMA_HASH` is generated and moves on its own; this integer does
/// not, so a message added without touching it ships a wire change under an unchanged version and
/// two builds that disagree will shake hands anyway. Failing here is the reminder.
#[test]
fn the_protocol_version_is_seven() {
    assert_eq!(PROTOCOL_VERSION, 7);
}

/// The daemon finds its token where the image says it will. If these two drift, a sandbox starts
/// and then refuses every connection, with the cause a mount path away from anything visible.
#[test]
fn the_container_token_path_matches_what_the_image_declares() {
    let containerfile = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packaging/sandbox/Containerfile"
    ))
    .expect("read the Containerfile");
    assert!(
        containerfile.contains(&format!("{TOKEN_PATH_ENV}={CONTAINER_TOKEN_PATH}")),
        "the image must set {TOKEN_PATH_ENV} to {CONTAINER_TOKEN_PATH}"
    );
}
