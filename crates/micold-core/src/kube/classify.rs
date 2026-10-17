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
    /// The image runs but does not provide what Micold needs (shared SSH image contract).
    ImageContract {
        /// The image.
        image: String,
        /// What is missing, e.g. `no SSH server answered on port 2222`.
        missing: String,
    },
    /// The pod's SSH host key is not the one its access secret holds.
    HostKeyMismatch {
        /// The pod.
        pod: String,
    },
    /// A phase did not finish within its bound.
    PhaseTimeout {
        /// The phase.
        phase: StartPhase,
    },
    /// The user cancelled the start.
    Cancelled,
    /// Stop deleted the pod and it was still there when the wait ended.
    StopTimeout {
        /// The pod.
        pod: String,
    },
    /// Any other answer of the API server.
    Api {
        /// The HTTP status, 0 when no response came.
        status: u16,
        /// The status message.
        message: String,
    },
    /// This build cannot run Kubernetes daemons: the shared SSH image (#806) is missing.
    Unavailable,
}

impl From<crate::kube::ApiFailure> for KubeFailure {
    /// An answer the sequences have no closer reason for (the classifiers come later).
    fn from(failure: crate::kube::ApiFailure) -> Self {
        KubeFailure::Api {
            status: failure.code,
            message: failure.message,
        }
    }
}

/// How long Stop waits for a deleted pod to be gone (contract §5).
pub const STOP_WAIT_SECS: u64 = 120;

fn paths(list: &[PathBuf]) -> String {
    list.iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A phase bound as the user reads it: whole minutes from ten minutes up, else seconds.
fn bound_text(phase: StartPhase) -> String {
    let secs = phase.bound().as_secs();
    if secs >= 600 {
        format!("{} min", secs / 60)
    } else {
        format!("{secs} s")
    }
}

impl fmt::Display for KubeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use KubeFailure as K;
        match self {
            K::NoKubeconfig { looked_in } => {
                write!(f, "no kubeconfig found; looked in {}", paths(looked_in))
            }
            K::ContextMissing { context, looked_in } => write!(
                f,
                "context {context} is not in the kubeconfig ({})",
                paths(looked_in)
            ),
            K::Unreachable {
                context,
                server,
                error,
            } => {
                write!(
                    f,
                    "cannot reach the cluster of {context} at {server}: {error}"
                )
            }
            K::ExecPlugin {
                command,
                error,
                search_path,
            } => write!(
                f,
                "the kubeconfig's credential plugin {command} failed: {error} (searched {})",
                paths(search_path)
            ),
            K::NamespaceMissing { namespace } => write!(
                f,
                "namespace {namespace} does not exist; Micold does not create namespaces"
            ),
            K::Forbidden {
                verb,
                resource,
                namespace,
            } => {
                write!(f, "not allowed to {verb} {resource} in {namespace}")
            }
            K::ImagePull { image, message } => {
                write!(f, "the cluster cannot pull {image}: {message}")
            }
            K::Unschedulable { message } => write!(f, "no node can run the pod: {message}"),
            K::ClaimNotBound { claim, message } => {
                write!(f, "volume {claim} is not bound: {message}")
            }
            K::Conflict {
                kind,
                name,
                namespace,
            } => write!(
                f,
                "a {kind} named {name} already exists in {namespace} and was not created by \
                 Micold; rename the daemon or remove it"
            ),
            K::CredentialSource { credential, path } => write!(
                f,
                "{credential} is allowed but {} does not exist",
                path.display()
            ),
            K::ImageContract { image, missing } => write!(
                f,
                "{image} does not provide the SSH server Micold needs: {missing}"
            ),
            K::HostKeyMismatch { pod } => write!(
                f,
                "the SSH host key of {pod} does not match the one in its access secret; refusing \
                 to connect"
            ),
            K::PhaseTimeout { phase } => {
                write!(f, "{phase} did not finish within {}", bound_text(*phase))
            }
            K::Cancelled => f.write_str("start cancelled"),
            K::StopTimeout { pod } => {
                write!(f, "pod {pod} was not gone within {STOP_WAIT_SECS} s")
            }
            K::Api { status, message } => write!(f, "the cluster answered {status}: {message}"),
            K::Unavailable => f.write_str(
                "Kubernetes daemons need the shared SSH image (#806), which this build does not \
                 include",
            ),
        }
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
        let looked = vec![
            PathBuf::from("/home/u/.kube/config"),
            PathBuf::from("/etc/k"),
        ];
        let cases = [
            (
                KubeFailure::NoKubeconfig {
                    looked_in: looked.clone(),
                },
                "no kubeconfig found; looked in /home/u/.kube/config, /etc/k",
            ),
            (
                KubeFailure::ContextMissing {
                    context: "prod".into(),
                    looked_in: looked.clone(),
                },
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
                KubeFailure::NamespaceMissing {
                    namespace: "dev".into(),
                },
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
                KubeFailure::ImagePull {
                    image: "x:1".into(),
                    message: "not found".into(),
                },
                "the cluster cannot pull x:1: not found",
            ),
            (
                KubeFailure::Unschedulable {
                    message: "0/1 nodes".into(),
                },
                "no node can run the pod: 0/1 nodes",
            ),
            (
                KubeFailure::ClaimNotBound {
                    claim: "d-data".into(),
                    message: "no class".into(),
                },
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
                KubeFailure::PhaseTimeout {
                    phase: StartPhase::Pull,
                },
                "pulling the image did not finish within 15 min",
            ),
            (
                KubeFailure::PhaseTimeout {
                    phase: StartPhase::Tunnel,
                },
                "opening the tunnel did not finish within 120 s",
            ),
            (KubeFailure::Cancelled, "start cancelled"),
            (
                KubeFailure::ImageContract {
                    image: "ghcr.io/x/ssh:1".into(),
                    missing: "no SSH server answered on port 2222".into(),
                },
                "ghcr.io/x/ssh:1 does not provide the SSH server Micold needs: no SSH server \
                 answered on port 2222",
            ),
            (
                KubeFailure::HostKeyMismatch {
                    pod: "micold-x-1".into(),
                },
                "the SSH host key of micold-x-1 does not match the one in its access secret; \
                 refusing to connect",
            ),
            (
                KubeFailure::StopTimeout {
                    pod: "micold-x-1".into(),
                },
                "pod micold-x-1 was not gone within 120 s",
            ),
            (
                KubeFailure::Api {
                    status: 500,
                    message: "etcdserver: request timed out".into(),
                },
                "the cluster answered 500: etcdserver: request timed out",
            ),
            (
                KubeFailure::Unavailable,
                "Kubernetes daemons need the shared SSH image (#806), which this build does not \
                 include",
            ),
        ];
        for (failure, expected) in cases {
            assert_eq!(text(failure), expected);
        }
    }

    #[test]
    fn an_api_failure_without_a_closer_reason_is_api() {
        let failure = KubeFailure::from(crate::kube::ApiFailure {
            code: 422,
            reason: "Invalid".into(),
            message: "spec.containers: Required value".into(),
        });
        assert_eq!(
            failure,
            KubeFailure::Api {
                status: 422,
                message: "spec.containers: Required value".into(),
            }
        );
    }
}
