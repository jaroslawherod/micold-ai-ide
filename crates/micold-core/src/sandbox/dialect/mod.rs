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
    /// What this runtime prints when it is installed but its service cannot be reached.
    ///
    /// Lower-case substrings, matched against the runtime's combined output by
    /// [`classify`](crate::sandbox::runtime::classify). Text-matching is unlovely and is the only
    /// signal a CLI gives (research R7) — what matters is that the phrases live in the dialect,
    /// so a new runtime declares its own wording instead of `classify` growing a branch per
    /// runtime. Docker's phrasing recognised for podman is worse than useless: podman's
    /// service-down message would fall through to `Unknown`, and the user is told nothing.
    pub not_running_phrases: &'static [&'static str],
    /// What it prints when it is reachable but this user may not drive it — group membership on
    /// Docker, an uninitialised rootless setup on podman.
    ///
    /// Matched **before** [`Self::not_running_phrases`]: podman says it cannot connect to its
    /// socket *because* permission was denied, and the fix is the permission, not the service.
    pub not_permitted_phrases: &'static [&'static str],
    /// What it prints when it will not make one of the binds it was asked for.
    ///
    /// Here for the same reason the two above are, and the reason is sharper: the runtimes do not
    /// merely word this differently, they describe *different things*. Docker names the mount
    /// configuration (`invalid mount config for type "bind"`); podman names the syscall that failed
    /// (`statfs <path>: no such file or directory`). A shared list written from Docker's wording
    /// classifies podman's refusal as `Unknown`, which is precisely the anonymous failure T103
    /// exists to remove — and it did, until a real podman message was put in front of it.
    pub mount_rejected_phrases: &'static [&'static str],
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
    fn every_runtime_names_the_failures_it_can_report() {
        // A dialect with no phrases is a runtime whose "service is down" and "you are not allowed"
        // both arrive as `Unknown`, which is the anonymous failure FR-034 exists to prevent. That
        // is easy to leave out when adding a runtime and invisible until someone hits it, so it
        // is a property of the table rather than a note in the contract.
        for kind in RuntimeKind::ALL {
            let d = Dialect::for_kind(kind);
            assert!(!d.not_running_phrases.is_empty(), "{kind}");
            assert!(!d.not_permitted_phrases.is_empty(), "{kind}");
            assert!(!d.mount_rejected_phrases.is_empty(), "{kind}");
            for p in d
                .not_running_phrases
                .iter()
                .chain(d.not_permitted_phrases)
                .chain(d.mount_rejected_phrases)
            {
                assert_eq!(
                    *p,
                    p.to_ascii_lowercase(),
                    "{kind}: phrases are matched against lower-cased output, so `{p}` never matches"
                );
            }
        }
    }

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
