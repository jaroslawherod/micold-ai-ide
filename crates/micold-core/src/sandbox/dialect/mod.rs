//! Per-runtime argument dialects.
//!
//! A dialect is a table of one runtime's differences — flag names, defaults, identity mapping —
//! not a second client. That is what makes FR-020's "replaceable runtime" cheap: podman's CLI is
//! near-identical to Docker's, so supporting it is a table, not a rewrite.
//!
//! A new dialect claims support by passing the conformance suite in
//! `specs/027-sandboxed-daemon-runtime/contracts/container-runtime.md`, not by existing.

pub mod docker;
pub mod podman;

use super::runtime::{IdentityMapping, RuntimeKind};

/// One runtime's spelling of the things the sandbox needs.
///
/// Everything here is data, deliberately. A dialect that needed *behaviour* would be a sign the
/// abstraction is in the wrong place, and the contract says so: if a new runtime forces a change to
/// `argv.rs`'s callers, the abstraction is wrong and that is the signal to revisit it rather than
/// special-case around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialect {
    pub kind: RuntimeKind,
    /// The executable.
    pub program: &'static str,
    /// How the host user's identity is carried in (research R3).
    pub identity: IdentityMapping,
    /// The option that disables outbound NAT on a user-defined bridge (research R4). Both runtimes
    /// take the same driver option, but a future one may not.
    pub no_masquerade_opt: &'static str,
    /// The minimum version this feature has been exercised against.
    pub minimum_version: &'static str,
}

impl Dialect {
    /// The dialect for a runtime.
    pub fn for_kind(kind: RuntimeKind) -> Self {
        match kind {
            RuntimeKind::Docker => docker::dialect(),
            RuntimeKind::Podman => podman::dialect(),
        }
    }

    /// The identity arguments for a host uid/gid.
    pub fn identity_args(&self, uid: u32, gid: u32) -> Vec<String> {
        match self.identity {
            // Read at start time, never baked into the image: one image serves every user, and
            // files written into a project come out owned by the person who ran the app.
            IdentityMapping::ExplicitUidGid => vec!["--user".into(), format!("{uid}:{gid}")],
            // Podman rootless already runs as the invoking user; `keep-id` maps them to the same
            // uid *inside*, which is what makes the written files match.
            IdentityMapping::KeepId => vec!["--userns=keep-id".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_runtime_has_a_dialect() {
        for kind in RuntimeKind::ALL {
            let d = Dialect::for_kind(kind);
            assert_eq!(d.kind, kind);
            assert_eq!(d.program, kind.program());
        }
    }

    #[test]
    fn identity_arguments_differ_by_runtime_and_never_run_as_root() {
        // Conformance check K-6. The failure this guards against is silent: a container that runs
        // as root writes root-owned files into the user's project, and the user finds out when
        // they try to edit one.
        let docker = Dialect::for_kind(RuntimeKind::Docker).identity_args(1000, 1000);
        assert_eq!(docker, vec!["--user", "1000:1000"]);

        let podman = Dialect::for_kind(RuntimeKind::Podman).identity_args(1000, 1000);
        assert_eq!(podman, vec!["--userns=keep-id"]);

        for kind in RuntimeKind::ALL {
            let args = Dialect::for_kind(kind).identity_args(0, 0).join(" ");
            assert!(!args.contains("root"), "{kind} named root explicitly");
        }
    }
}
