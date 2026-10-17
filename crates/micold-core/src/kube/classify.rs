//! Why a Kubernetes daemon could not start or keep running, in the user's words (feature 686,
//! contract §6). The classifiers that build these from cluster answers arrive with M2, M4 and M6.

use std::fmt;
use std::path::PathBuf;

use crate::kube::start::StartPhase;

/// A Kubernetes daemon failure. `Display` is the text the row and the recent errors show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KubeFailure {
    /// The context is not in any kubeconfig file.
    ContextMissing {
        /// The context.
        context: String,
        /// The files read.
        looked_in: Vec<PathBuf>,
    },
    /// No kubeconfig file exists.
    NoKubeconfig {
        /// The files looked for.
        looked_in: Vec<PathBuf>,
    },
    /// The API server could not be reached.
    Unreachable {
        /// The context.
        context: String,
        /// The API server URL.
        server: String,
        /// What went wrong.
        error: String,
    },
    /// The kubeconfig's exec credential plugin failed or was not found.
    ExecPlugin {
        /// The plugin command.
        command: String,
        /// What went wrong.
        error: String,
        /// Where it was looked for.
        search_path: Vec<PathBuf>,
    },
    /// The namespace does not exist.
    NamespaceMissing {
        /// The namespace.
        namespace: String,
    },
    /// The user may not do something the start needs.
    Forbidden {
        /// The verb, e.g. `create`.
        verb: String,
        /// The resource, e.g. `pods/portforward`.
        resource: String,
        /// The namespace.
        namespace: String,
    },
    /// No container runtime to build with.
    NoRuntime,
    /// The image build failed.
    Build {
        /// The last lines of its output.
        output_tail: String,
    },
    /// The push failed.
    Push {
        /// The registry.
        registry: String,
        /// The runtime's error.
        error: String,
    },
    /// The cluster cannot pull the image.
    ImagePull {
        /// The image.
        image: String,
        /// The kubelet's message.
        message: String,
    },
    /// No node can run the pod.
    Unschedulable {
        /// The scheduler's message.
        message: String,
    },
    /// The volume claim is not bound.
    ClaimNotBound {
        /// The claim.
        claim: String,
        /// Why.
        message: String,
    },
    /// An object of the daemon's name exists and is not ours (contract §2).
    Conflict {
        /// Its kind, e.g. `pod`.
        kind: String,
        /// Its name.
        name: String,
        /// The namespace.
        namespace: String,
    },
    /// An allowed credential's source file is missing.
    CredentialSource {
        /// The credential, as the form names it.
        credential: String,
        /// The missing file.
        path: PathBuf,
    },
    /// The settings ask for a smaller volume than the existing claim has.
    VolumeTooSmall {
        /// The claim.
        claim: String,
    },
    /// A phase did not finish within its bound.
    PhaseTimeout {
        /// The phase.
        phase: StartPhase,
    },
    /// The user cancelled the start.
    Cancelled,
    /// Any other answer of the API server.
    Api {
        /// The HTTP status.
        code: u16,
        /// The status reason.
        reason: String,
        /// The status message.
        message: String,
    },
}

fn paths(list: &[PathBuf]) -> String {
    let _ = list;
    String::new()
}

impl fmt::Display for KubeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = paths(&[]);
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(f: KubeFailure) -> String {
        f.to_string()
    }

    #[test]
    fn every_failure_reads_as_the_contract_says() {
        let looked = vec![PathBuf::from("/home/u/.kube/config"), PathBuf::from("/etc/k")];
        let cases = [
            (
                KubeFailure::NoKubeconfig { looked_in: looked.clone() },
                "no kubeconfig found; looked in /home/u/.kube/config, /etc/k",
            ),
            (
                KubeFailure::ContextMissing { context: "prod".into(), looked_in: looked.clone() },
                "context prod is not in the kubeconfig (/home/u/.kube/config, /etc/k)",
            ),
            (
                KubeFailure::Unreachable {
                    context: "prod".into(),
                    server: "https://10.0.0.1:6443".into(),
                    error: "connection refused".into(),
                },
                "cannot reach the cluster of prod at https://10.0.0.1:6443: connection refused",
            ),
            (
                KubeFailure::ExecPlugin {
                    command: "gke-gcloud-auth-plugin".into(),
                    error: "not found".into(),
                    search_path: vec!["/usr/bin".into(), "/opt/homebrew/bin".into()],
                },
                "the kubeconfig's credential plugin gke-gcloud-auth-plugin failed: not found \
                 (searched /usr/bin, /opt/homebrew/bin)",
            ),
            (
                KubeFailure::NamespaceMissing { namespace: "dev".into() },
                "namespace dev does not exist; Micold does not create namespaces",
            ),
            (
                KubeFailure::Forbidden {
                    verb: "create".into(),
                    resource: "pods/portforward".into(),
                    namespace: "dev".into(),
                },
                "not allowed to create pods/portforward in dev",
            ),
            (
                KubeFailure::NoRuntime,
                "a container runtime (docker or podman) is needed to build the image; pulling a \
                 given image needs none",
            ),
            (
                KubeFailure::Build { output_tail: "step 3 failed".into() },
                "image build failed: step 3 failed",
            ),
            (
                KubeFailure::Push { registry: "reg:5000/t".into(), error: "denied".into() },
                "pushing to reg:5000/t failed: denied",
            ),
            (
                KubeFailure::ImagePull { image: "x:1".into(), message: "not found".into() },
                "the cluster cannot pull x:1: not found",
            ),
            (
                KubeFailure::Unschedulable { message: "0/1 nodes".into() },
                "no node can run the pod: 0/1 nodes",
            ),
            (
                KubeFailure::ClaimNotBound { claim: "d-data".into(), message: "no class".into() },
                "volume d-data is not bound: no class",
            ),
            (
                KubeFailure::Conflict {
                    kind: "pod".into(),
                    name: "micold-x-1".into(),
                    namespace: "dev".into(),
                },
                "a pod named micold-x-1 already exists in dev and was not created by Micold; \
                 rename the daemon or remove it",
            ),
            (
                KubeFailure::CredentialSource {
                    credential: "Git credentials".into(),
                    path: "/home/u/.git-credentials".into(),
                },
                "Git credentials is allowed but /home/u/.git-credentials does not exist",
            ),
            (
                KubeFailure::PhaseTimeout { phase: StartPhase::Pull },
                "pulling the image did not finish within 15 min",
            ),
            (
                KubeFailure::PhaseTimeout { phase: StartPhase::Tunnel },
                "opening the tunnel did not finish within 120 s",
            ),
            (KubeFailure::Cancelled, "start cancelled"),
        ];
        for (failure, expected) in cases {
            assert_eq!(text(failure), expected);
        }
    }
}
