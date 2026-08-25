//! Podman's argument dialect: rootless defaults and `--userns=keep-id` (research R3).

use super::Dialect;
use crate::sandbox::runtime::{IdentityMapping, RuntimeKind};

/// Podman's dialect.
///
/// Written alongside Docker's rather than after it. An abstraction with one implementation is a
/// guess, and FR-020's claim that the runtime is replaceable is only demonstrable with a second
/// one that passes the same conformance suite.
pub fn dialect() -> Dialect {
    Dialect {
        kind: RuntimeKind::Podman,
        program: "podman",
        // Rootless podman already runs as the invoking user; `keep-id` maps them to the same uid
        // inside the container, which is what makes files written into a project come out owned by
        // the host user rather than by podman's subuid mapping.
        identity: IdentityMapping::KeepId,
        no_masquerade_opt: "com.docker.network.bridge.enable_ip_masquerade=false",
        minimum_version: "4.0",
        // Rootless podman has no daemon to start, so "not running" means the user-level service or
        // the `podman machine` VM is not up. Its message says so by telling the user which command
        // would fix it.
        not_running_phrases: &[
            "cannot connect to podman",
            "unable to connect to podman socket",
            "podman machine start",
            "is the podman service running",
        ],
        // The subuid range is podman's characteristic first-run failure and does not contain the
        // words "permission denied" anywhere: it is a permission problem stated in podman's own
        // terms, and left unlisted it reads as an unknown error to a user one `usermod` away from
        // a working sandbox.
        not_permitted_phrases: &[
            "permission denied",
            "no subuid ranges found",
            "no subgid ranges found",
            "check rootless mode",
        ],
        // Podman does not talk about mount configuration at all: it reports the syscall it tried.
        // `statfs <path>: no such file or directory` is what a bind onto a path podman cannot reach
        // looks like, and nothing in it matches Docker's wording — which is how this failure spent
        // T103 landing in `Unknown` for one of the two supported runtimes.
        mount_rejected_phrases: &["statfs ", "invalid mount", "read-only file system"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn podman_maps_identity_by_keeping_the_invoking_user() {
        assert_eq!(dialect().identity, IdentityMapping::KeepId);
    }
}
