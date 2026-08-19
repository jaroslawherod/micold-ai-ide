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
