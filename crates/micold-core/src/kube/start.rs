//! The start sequence of a Kubernetes daemon (feature 686, research K6, contract §3).

use std::fmt;
use std::time::Duration;

/// One phase of a start, shown as the row's activity while it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartPhase {
    /// Read the kubeconfig and build the client.
    Config,
    /// Check the namespace and permissions.
    Preflight,
    /// Build and push the image.
    Image,
    /// Create or adopt the volume claim.
    Volume,
    /// Write the allowed credentials' secrets.
    Secrets,
    /// Create or adopt the pod.
    Pod,
    /// Wait for the pod to be scheduled.
    Schedule,
    /// Wait for the image to be pulled and the container to run.
    Pull,
    /// Open the port-forward tunnel.
    Tunnel,
}

impl StartPhase {
    /// How long the phase may take before the start fails (FR-012).
    pub fn bound(self) -> Duration {
        match self {
            StartPhase::Image => Duration::from_secs(30 * 60),
            StartPhase::Pull => Duration::from_secs(15 * 60),
            _ => Duration::from_secs(120),
        }
    }

    /// The row's activity text.
    pub fn activity(self) -> &'static str {
        match self {
            StartPhase::Config => "reading the kubeconfig",
            StartPhase::Preflight => "checking the namespace",
            StartPhase::Image => "building the image",
            StartPhase::Volume => "preparing the volume",
            StartPhase::Secrets => "writing the secrets",
            StartPhase::Pod => "creating the pod",
            StartPhase::Schedule => "scheduling the pod",
            StartPhase::Pull => "pulling the image",
            StartPhase::Tunnel => "opening the tunnel",
        }
    }
}

impl fmt::Display for StartPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.activity())
    }
}
