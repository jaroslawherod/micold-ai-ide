//! Docker's argument dialect and capability baseline (FR-021).

use super::Dialect;
use crate::sandbox::runtime::{IdentityMapping, RuntimeKind};

/// Docker's dialect.
pub fn dialect() -> Dialect {
    Dialect {
        kind: RuntimeKind::Docker,
        program: "docker",
        identity: IdentityMapping::ExplicitUidGid,
        no_masquerade_opt: "com.docker.network.bridge.enable_ip_masquerade=false",
        // 20.10 is where `--pids-limit` and the network driver options this feature relies on are
        // uniformly available. Measured against 29.5.1 (research R4, R5).
        minimum_version: "20.10",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_masquerade_option_is_the_one_measured_to_work() {
        // Research R4: this exact option is what keeps a published port working while blocking
        // outbound connections. The obvious alternative (`--internal`) leaves the container
        // unreachable from the host, which severs the control channel.
        assert_eq!(
            dialect().no_masquerade_opt,
            "com.docker.network.bridge.enable_ip_masquerade=false"
        );
    }
}
