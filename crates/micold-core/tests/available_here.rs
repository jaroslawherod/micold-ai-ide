//! `available_here` over a scratch `PATH` (feature 026 T070/FR-006; re-sited by feature 027
//! FR-023c).
//!
//! This suite used to be `Capabilities::available_providers`'s, inside `micold-client`. FR-023c
//! moved the question: availability is settled **where sessions run**, and under the sandboxed
//! placement that is not the client's process. The probe now lives in `micold_core::provider` and
//! is called by the *daemon*, so its test came with it — the assertions are unchanged, because
//! what they assert never depended on who was asking.
//!
//! `crates/micold-daemon/tests/ai_cli_availability.rs` holds the other end: that the service
//! reports this over the protocol rather than a constant. Together they are FR-023c.

use micold_core::provider::available_here;
use micold_core::session::AiCli;

use std::path::{Path, PathBuf};

/// A `PATH` containing only what a test installs into it, restored on drop.
///
/// Two directories rather than one, so the *search* order can be made to disagree with
/// `AiCli::ALL`'s order.
#[cfg(unix)]
struct ScopedPath {
    _base: tempfile::TempDir,
    first: PathBuf,
    second: PathBuf,
    previous: Option<std::ffi::OsString>,
}

#[cfg(unix)]
impl ScopedPath {
    fn empty() -> Self {
        let base = tempfile::tempdir().expect("scratch PATH");
        let (first, second) = (base.path().join("first"), base.path().join("second"));
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let previous = std::env::var_os("PATH");
        std::env::set_var("PATH", format!("{}:{}", first.display(), second.display()));
        Self {
            _base: base,
            first,
            second,
            previous,
        }
    }

    fn install_in(&self, dir: &Path, command: &str) {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(command);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn install(&self, command: &str) {
        self.install_in(&self.first, command);
    }

    fn uninstall(&self, command: &str) {
        let _ = std::fs::remove_file(self.first.join(command));
        let _ = std::fs::remove_file(self.second.join(command));
    }
}

#[cfg(unix)]
impl Drop for ScopedPath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// `available_here` over a scratch `PATH` (T070, FR-006), in one test function because
/// `PATH` is process-global — the same arrangement, and the same reason, as
/// `micold-client`'s `shell/persist.rs` boot-prune test.
///
/// T070 was written expecting to substitute a fake provider registry and assert that the offer
/// follows it. There is no such seam, deliberately: `AiCli::provider` is an exhaustive match in
/// `micold-core` (T011a) precisely so that no code path can resolve a CLI name to nothing, and
/// `micold-client`'s `shell/capabilities.rs` records why the delegating `Capabilities::provider`
/// that would have been the other half of it was deleted — nothing called it.
///
/// It does not need one. Availability *is* a `PATH` probe, so the honest way to drive these
/// few lines is to give the process a `PATH` and read back what they offer. That also keeps
/// the assertion about the real thing: a fake registry would have proven the filter forwards a
/// predicate, not that the offer follows what is installed.
///
/// The ends were already held — `micold-core/tests/copilot_provider.rs` drives the predicate,
/// `micold-client/tests/features_session.rs` drives what the client does with the answer. What
/// was not held is that these two are connected at all: `available_here` returning
/// `AiCli::ALL` unconditionally satisfies both of those suites.
#[cfg(unix)]
#[test]
fn the_offer_is_exactly_what_is_installed_now() {
    let path = ScopedPath::empty();

    // Nothing installed: the offer is empty, not a default guess. `features_session.rs` covers
    // what the client does with an empty offer; this covers that an empty offer is what it is
    // handed.
    assert!(
        available_here().is_empty(),
        "an empty PATH must offer no CLI at all"
    );

    // One installed: the filter is per provider, not all-or-nothing — and note this is the
    // *second* variant, so a stub returning `ALL[..n]` does not pass.
    path.install(AiCli::Copilot.provider().command());
    assert_eq!(
        available_here(),
        vec![AiCli::Copilot],
        "installing one CLI must offer that one and no other"
    );

    // Every variant installed: the offer is every variant. Written over `AiCli::ALL` rather
    // than over the two names we have today, so a third CLI is covered here the day it joins
    // the registry — that totality is what T011a's exhaustive match buys, and this is the
    // place the client collects on it.
    for which in AiCli::ALL {
        path.install(which.provider().command());
    }
    assert_eq!(
        available_here(),
        AiCli::ALL.to_vec(),
        "with every CLI installed the offer is the whole registry"
    );

    // And live at this level too, not only at the provider's: the same `Capabilities` answers
    // differently when the machine changes, because the client refreshes by calling this again
    // rather than by invalidating something it stored (research R11).
    path.uninstall(AiCli::ClaudeCode.provider().command());
    assert_eq!(
        available_here(),
        vec![AiCli::Copilot],
        "uninstalling must shrink the offer; anything memoised would still list it"
    );

    the_order_is_the_declared_one(&path);
}

/// The order is `AiCli::ALL`'s, not `PATH`'s.
///
/// The doc comment says "in `AiCli::ALL`'s order" and the settings select renders the list as
/// it arrives, so the order is user-visible — inheriting `PATH`'s would make a menu's order
/// depend on the shape of the user's machine. `filter` over `into_iter` gives this for free
/// today; the assertion is here so that a rewrite into a "search each PATH entry" shape has to
/// notice.
#[cfg(unix)]
fn the_order_is_the_declared_one(path: &ScopedPath) {
    for which in AiCli::ALL {
        path.uninstall(which.provider().command());
    }
    // Installed in reverse: the *last* variant of `ALL` is the one found in the first PATH
    // entry, so a PATH-ordered answer comes back reversed.
    let reversed: Vec<AiCli> = AiCli::ALL.into_iter().rev().collect();
    for (i, which) in reversed.iter().enumerate() {
        let dir = if i == 0 { &path.first } else { &path.second };
        path.install_in(dir, which.provider().command());
    }

    assert_eq!(
        available_here(),
        AiCli::ALL.to_vec(),
        "the offer follows the declared registry order, not the order PATH resolves in"
    );
}
