//! §B.7 of the quickstart — the development loop's staleness check, against a real image.
//!
//! `mise run image` builds a `:dev` image from the working tree, and the client refuses one whose
//! daemon was built from a *different* tree (FR-024d, research R8). Both halves of that have to be
//! true or the loop is worse than useless:
//!
//! - a **fresh** image must be accepted with the check armed, or `mise run image` never produces a
//!   usable image and every sandboxed developer is stuck;
//! - a **stale** one must be refused, or the developer debugs yesterday's daemon against today's
//!   code.
//!
//! Only the first half is a stable test: the second needs two images built from two different
//! trees, which is a build, not an assertion. It is recorded in `evidence/us7-dev-loop.md`, and the
//! message the refusal produces is held by
//! `micold-client/src/daemon.rs::the_stale_image_advice_names_the_tag_and_the_rebuild_command`.
//!
//! What this catches, and nothing else does: the image built by `mise run image` carrying a
//! fingerprint that does not match the tree it was built from — a release-profile build, a stale
//! layer cache, a `build.rs` that did not re-run. Each would refuse every client on a correct
//! image, and the first person to see it would be told to do the rebuild they had just done.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use micold_core::connect::{connect_at, Connected, Credentials};
use micold_core::endpoint::DialAddress;
use micold_core::protocol::auth::Token;
use micold_core::protocol::messages::PresentedToken;

use sandbox_real_support::{credentials, seed, start_sandbox, wait_for_accept, SandboxSpec};

const CONTAINER: &str = "micold-fingerprint-probe";
const NETWORK: &str = "micold-fingerprint-probe-net";
const PORT: u16 = 17734;

#[tokio::test]
async fn sandbox_real_a_freshly_built_image_passes_the_strict_fingerprint_check() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "fingerprint");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        extra: &[],
    });

    // Lenient first, so a failure below is a *fingerprint* refusal and not "the daemon is not up
    // yet" wearing one.
    let (_lenient, _) = wait_for_accept(PORT, &credentials(&token)).await;

    // Then the check the client arms for a `LocalBuild` image (`startup.rs`:
    // `image.refuses_fingerprint_mismatch()`).
    let strict = Credentials {
        auth_token: Some(PresentedToken::new(token.as_str())),
        require_fingerprint_match: true,
    };
    match connect_at(&DialAddress::Loopback { port: PORT }, "fp-probe", &strict).await {
        Ok(Some(Connected::Ready(..))) => {}
        Ok(Some(Connected::Refused(reason))) => panic!(
            "a `:dev` image built from this very tree was refused as stale: {reason:?}\n\
             The ordinary cause is the honest one — micold-core's `src/` has changed since the last \
             `mise run image`, so rebuild and re-run. If it has not, then either that task did not \
             rebuild the daemon, or the fingerprint is not a function of the source tree it claims \
             to be."
        ),
        Ok(None) => panic!("nothing listening on the strict attempt"),
        Err(e) => panic!("strict connect failed for another reason: {e}"),
    }
}
