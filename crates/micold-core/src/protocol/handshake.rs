//! Handshake evaluation (contracts/protocol.md §4).
//!
//! Strict exact-match, no negotiation, no compatibility range (FR-021). Both `protocol_version`
//! **and** `schema_hash` must match; on either mismatch the daemon refuses and names both sides'
//! version and hash plus its own build (FR-022), so the client can render an actionable diagnostic
//! and offer the restart action.
//!
//! A matching contract does not mean matching *builds*: most releases (e.g. a daemon-only bugfix)
//! don't touch the wire schema, so `protocol_version`/`schema_hash` alone never catch a `.deb`
//! upgrade over an already-running daemon. [`PACKAGE_VERSION`] changes on every release and closes
//! that gap as a second, independent check (FR-022a, BUG-002).

use crate::protocol::auth::Token;
use crate::protocol::messages::RefusalReason;
use crate::protocol::version::{BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};

/// Everything the daemon needs from a `Hello` to decide (feature 027).
///
/// A struct rather than eight positional arguments: the previous signature was already five wide,
/// and two of the new fields are `String`s that would sit next to each other at the call site with
/// nothing but order to tell them apart.
#[derive(Debug, Clone)]
pub struct Introduction {
    /// The client's protocol version.
    pub protocol_version: u32,
    /// The client's schema hash.
    pub schema_hash: [u8; 32],
    /// The client's package version.
    pub package_version: String,
    /// The client's build string.
    pub build: String,
    /// The token the client presented, if any (feature 027, research R1).
    pub auth_token: Option<String>,
    /// The client's build fingerprint (feature 027, research R8).
    pub fingerprint: String,
    /// Whether a fingerprint mismatch refuses.
    pub require_fingerprint_match: bool,
}

/// What this daemon compares an [`Introduction`] against.
#[derive(Debug, Clone, Default)]
pub struct Expectation {
    /// The token this daemon requires, if it was given one.
    ///
    /// `None` for the host-process placement: its `0700`-guarded socket already proves the caller
    /// is the user, and requiring a token there would break every existing installation for no
    /// gain. `Some` whenever the daemon was started with a token file — which the sandbox always
    /// does, and which is the only configuration where the transport proves nothing by itself.
    pub token: Option<Token>,
    /// This daemon's build string, named in refusals.
    pub build: String,
    /// The image reference this daemon is running from, named in a stale-image refusal.
    pub image: String,
}

/// Evaluate a full introduction, including feature 027's token and fingerprint.
///
/// # Order, and why it is this order
///
/// The contract checks come **first**: a client that disagrees about the wire cannot be told
/// anything meaningful about its token, and answering it at all would let an unauthenticated peer
/// learn whether a token is required. Authentication comes next, before the fingerprint, so a
/// caller who cannot authenticate learns nothing about what this daemon was built from.
pub fn evaluate_introduction(
    intro: &Introduction,
    expected: &Expectation,
) -> Result<(), RefusalReason> {
    evaluate(
        intro.protocol_version,
        intro.schema_hash,
        &intro.package_version,
        intro.build.clone(),
        expected.build.clone(),
    )?;

    if let Some(token) = &expected.token {
        let presented = intro.auth_token.as_deref().unwrap_or("");
        if !token.verify(presented) {
            return Err(RefusalReason::AuthRejected);
        }
    }

    if intro.require_fingerprint_match && intro.fingerprint != BUILD_FINGERPRINT {
        return Err(RefusalReason::StaleDevImage {
            client_fingerprint: intro.fingerprint.clone(),
            daemon_fingerprint: BUILD_FINGERPRINT.to_string(),
            image: expected.image.clone(),
        });
    }

    Ok(())
}

/// Evaluate a client's handshake against this daemon build.
///
/// Returns `Ok(())` only when the client's version, hash, **and** package version all match this
/// build's compiled constants. A contract mismatch (version or hash) refuses with
/// [`RefusalReason::VersionMismatch`] naming both sides (FR-021/022); a same-contract package-version
/// difference refuses with [`RefusalReason::BuildMismatch`] instead (FR-022a, BUG-002).
pub fn evaluate(
    client_version: u32,
    client_hash: [u8; 32],
    client_package_version: impl AsRef<str>,
    client_build: impl Into<String>,
    daemon_build: impl Into<String>,
) -> Result<(), RefusalReason> {
    if client_version != PROTOCOL_VERSION || client_hash != SCHEMA_HASH {
        return Err(RefusalReason::VersionMismatch {
            client: client_version,
            daemon: PROTOCOL_VERSION,
            client_hash,
            daemon_hash: SCHEMA_HASH,
            daemon_build: daemon_build.into(),
        });
    }
    if client_package_version.as_ref() != PACKAGE_VERSION {
        return Err(RefusalReason::BuildMismatch {
            client_build: client_build.into(),
            daemon_build: daemon_build.into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intro() -> Introduction {
        Introduction {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            package_version: PACKAGE_VERSION.to_string(),
            build: "client".to_string(),
            auth_token: None,
            fingerprint: BUILD_FINGERPRINT.to_string(),
            require_fingerprint_match: false,
        }
    }

    fn expecting(token: Option<Token>) -> Expectation {
        Expectation {
            token,
            build: "daemon".to_string(),
            image: "micold-daemon:dev".to_string(),
        }
    }

    #[test]
    fn a_matching_introduction_is_accepted() {
        assert!(evaluate_introduction(&intro(), &expecting(None)).is_ok());
    }

    #[test]
    fn a_daemon_expecting_no_token_does_not_require_one() {
        // The host-process placement, unchanged: its `0700`-guarded socket already proves the
        // caller is the user, and requiring a token there would break every existing install.
        let mut i = intro();
        i.auth_token = None;
        assert!(evaluate_introduction(&i, &expecting(None)).is_ok());
    }

    #[test]
    fn the_right_token_is_accepted_and_the_wrong_one_is_not() {
        let token = Token::generate();
        let mut i = intro();

        i.auth_token = Some(token.as_str().to_string());
        assert!(evaluate_introduction(&i, &expecting(Some(token.clone()))).is_ok());

        i.auth_token = Some("not the token".to_string());
        assert_eq!(
            evaluate_introduction(&i, &expecting(Some(token.clone()))),
            Err(RefusalReason::AuthRejected)
        );

        // Absent is refused the same way as wrong, and with the same message: a refusal that told
        // them apart would say whether a token is required at all.
        i.auth_token = None;
        assert_eq!(
            evaluate_introduction(&i, &expecting(Some(token))),
            Err(RefusalReason::AuthRejected)
        );
    }

    #[test]
    fn a_contract_mismatch_is_reported_before_authentication_is_attempted() {
        // Order matters. A client that disagrees about the wire cannot be told anything meaningful
        // about its token, and answering it would let an unauthenticated peer learn whether one is
        // required.
        let mut i = intro();
        i.protocol_version = PROTOCOL_VERSION - 1;
        i.auth_token = Some("wrong".to_string());
        let refusal = evaluate_introduction(&i, &expecting(Some(Token::generate()))).unwrap_err();
        assert!(matches!(refusal, RefusalReason::VersionMismatch { .. }));
    }

    #[test]
    fn a_fingerprint_mismatch_refuses_only_when_the_client_asked_it_to() {
        // The asymmetry research R8 requires. A released client and a released daemon are built
        // separately and legitimately differ; refusing there would break every normal install.
        let mut i = intro();
        i.fingerprint = "0000000000000000".to_string();

        i.require_fingerprint_match = false;
        assert!(evaluate_introduction(&i, &expecting(None)).is_ok());

        i.require_fingerprint_match = true;
        match evaluate_introduction(&i, &expecting(None)).unwrap_err() {
            RefusalReason::StaleDevImage {
                image,
                daemon_fingerprint,
                ..
            } => {
                // The remedy has to name something the user can act on: every other constant the
                // handshake compares matches here, which is the whole reason this refusal exists.
                assert_eq!(image, "micold-daemon:dev");
                assert_eq!(daemon_fingerprint, BUILD_FINGERPRINT);
            }
            other => panic!("expected StaleDevImage, got {other:?}"),
        }
    }

    #[test]
    fn an_unauthenticated_caller_learns_nothing_about_the_build() {
        // Authentication is checked before the fingerprint, so a wrong token cannot be used to
        // probe what this daemon was built from.
        let mut i = intro();
        i.auth_token = Some("wrong".to_string());
        i.fingerprint = "0000000000000000".to_string();
        i.require_fingerprint_match = true;
        assert_eq!(
            evaluate_introduction(&i, &expecting(Some(Token::generate()))),
            Err(RefusalReason::AuthRejected)
        );
    }

    #[test]
    fn the_fingerprint_is_not_empty() {
        // A build that emitted nothing would make every comparison trivially equal, and the stale
        // image check would silently stop working.
        assert!(!BUILD_FINGERPRINT.is_empty());
        assert!(BUILD_FINGERPRINT.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
