//! Why a Kubernetes daemon could not start or keep running, in the user's words (feature 686,
//! contract §6), and the classifiers that build them from cluster answers (M2).

use std::fmt;
use std::path::PathBuf;

use crate::kube::start::StartPhase;
use crate::kube::{ApiFailure, EventView, PodStatusView};

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
    /// Copying, installing or starting the daemon over SSH failed (806's reason text).
    Provision {
        /// The step, its cause and the tool's last output lines.
        reason: String,
    },
    /// A daemon of another build already runs in the pod (FR-017); no session starts on it.
    VersionMismatch {
        /// What the daemon refused with.
        reason: String,
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
    /// A call that got no answer is [`KubeFailure::Unreachable`] without context and server yet
    /// (see [`KubeFailure::at`]); any other answer is `Api`, until a classifier names it closer.
    fn from(failure: crate::kube::ApiFailure) -> Self {
        if failure.code == 0 && failure.reason == "Unreachable" {
            return KubeFailure::Unreachable {
                context: String::new(),
                server: String::new(),
                error: failure.message,
            };
        }
        KubeFailure::Api {
            status: failure.code,
            message: failure.message,
        }
    }
}

/// Where a call went, for the failure that names it.
#[derive(Debug, Clone, Copy)]
pub struct CallTarget<'a> {
    /// The kube context.
    pub context: &'a str,
    /// The API server URL.
    pub server: &'a str,
    /// The namespace.
    pub namespace: &'a str,
}

impl KubeFailure {
    /// Classify a failed call (`verb` on `resource`): `403` is a denial naming both, a `404` for
    /// the namespace is a missing namespace, an unanswered call is an unreachable cluster, and
    /// anything else is the cluster's own answer.
    pub fn from_status(
        failure: &ApiFailure,
        target: &CallTarget<'_>,
        verb: &str,
        resource: &str,
    ) -> Self {
        match failure.code {
            403 => KubeFailure::Forbidden {
                verb: verb.to_string(),
                resource: resource.to_string(),
                namespace: target.namespace.to_string(),
            },
            404 if failure.message.starts_with("namespaces \"") => KubeFailure::NamespaceMissing {
                namespace: target.namespace.to_string(),
            },
            _ => KubeFailure::from(failure.clone()).at(target),
        }
    }

    /// Name the context and server in an unreachable-cluster failure that has none yet (what a
    /// call that got no answer converts to).
    pub fn at(self, target: &CallTarget<'_>) -> Self {
        match self {
            KubeFailure::Unreachable {
                context,
                server,
                error,
            } if context.is_empty() && server.is_empty() => KubeFailure::Unreachable {
                context: target.context.to_string(),
                server: target.server.to_string(),
                error,
            },
            other => other,
        }
    }

    /// The failure a pod's status shows on first sight, if any: an image that cannot be pulled, a
    /// pod no node can run, a claim the scheduler found unbound, a container that ended.
    pub fn from_pod(status: &PodStatusView, image: &str, claim: &str) -> Option<Self> {
        if let Some((reason, message)) = &status.waiting {
            if matches!(
                reason.as_str(),
                "ErrImagePull" | "ImagePullBackOff" | "InvalidImageName"
            ) {
                return Some(KubeFailure::ImagePull {
                    image: image.to_string(),
                    message: message.clone(),
                });
            }
            if matches!(reason.as_str(), "Error" | "Completed" | "StartError") {
                return Some(KubeFailure::ImageContract {
                    image: image.to_string(),
                    missing: format!("the container exited: {reason}"),
                });
            }
        }
        match &status.schedule_reason {
            Some((reason, message)) if reason == "Unschedulable" => Some(
                if message.contains("unbound immediate PersistentVolumeClaims") {
                    KubeFailure::ClaimNotBound {
                        claim: claim.to_string(),
                        message: message.clone(),
                    }
                } else {
                    KubeFailure::Unschedulable {
                        message: message.clone(),
                    }
                },
            ),
            _ => None,
        }
    }

    /// The failure a claim's events show: the provisioner gave up.
    pub fn from_pvc(claim: &str, events: &[EventView]) -> Option<Self> {
        events
            .iter()
            .rev()
            .find(|e| e.reason == "ProvisioningFailed")
            .map(|e| KubeFailure::ClaimNotBound {
                claim: claim.to_string(),
                message: e.message.clone(),
            })
    }
}

/// Why the daemon's pod is no longer there (contract §4). The daemon shows `stopped` with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodGone {
    /// Deleted, or being deleted.
    Deleted,
    /// Evicted by the kubelet.
    Evicted {
        /// The kubelet's message.
        message: String,
    },
    /// The node it ran on is lost.
    NodeLost {
        /// The node.
        node: String,
    },
}

impl PodGone {
    /// Read a lookup of the pod: `None` for a pod that was not found, `node_exists` for whether
    /// its node is still known. `None` back means the pod is there.
    pub fn classify(pod: Option<&PodStatusView>, node_exists: bool) -> Option<Self> {
        let Some(pod) = pod.filter(|p| !p.deleting) else {
            return Some(PodGone::Deleted);
        };
        if let Some((reason, message)) = &pod.reason {
            if reason == "Evicted" {
                return Some(PodGone::Evicted {
                    message: message.clone(),
                });
            }
        }
        let node_lost = pod.node.is_some() && !node_exists;
        (pod.phase == "Unknown" || node_lost).then(|| PodGone::NodeLost {
            node: pod.node.clone().unwrap_or_default(),
        })
    }
}

impl fmt::Display for PodGone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PodGone::Deleted => f.write_str("stopped: the pod was deleted"),
            PodGone::Evicted { message } => write!(f, "stopped: the pod was evicted: {message}"),
            PodGone::NodeLost { node } => write!(f, "stopped: the pod's node {node} was lost"),
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
            K::Provision { reason } | K::VersionMismatch { reason } => f.write_str(reason),
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

    const TARGET: CallTarget<'static> = CallTarget {
        context: "prod",
        server: "https://10.0.0.1:6443",
        namespace: "dev",
    };

    fn api(code: u16, message: &str) -> ApiFailure {
        ApiFailure {
            code,
            reason: "X".into(),
            message: message.into(),
        }
    }

    #[test]
    fn a_status_is_classified_by_its_code() {
        let from = |f: ApiFailure| KubeFailure::from_status(&f, &TARGET, "create", "pods");
        assert_eq!(
            from(api(403, "pods is forbidden")),
            KubeFailure::Forbidden {
                verb: "create".into(),
                resource: "pods".into(),
                namespace: "dev".into(),
            }
        );
        assert_eq!(
            from(api(404, "namespaces \"dev\" not found")),
            KubeFailure::NamespaceMissing {
                namespace: "dev".into()
            }
        );
        assert_eq!(
            from(ApiFailure {
                reason: "Unreachable".into(),
                ..api(0, "connection refused")
            }),
            KubeFailure::Unreachable {
                context: "prod".into(),
                server: "https://10.0.0.1:6443".into(),
                error: "connection refused".into(),
            }
        );
        assert_eq!(
            from(api(404, "pods \"x\" not found")),
            KubeFailure::Api {
                status: 404,
                message: "pods \"x\" not found".into()
            }
        );
    }

    fn waiting(reason: &str, message: &str) -> PodStatusView {
        PodStatusView {
            phase: "Pending".into(),
            waiting: Some((reason.into(), message.into())),
            ..PodStatusView::default()
        }
    }

    #[test]
    fn a_pod_that_cannot_pull_its_image_fails_on_first_sight() {
        for reason in ["ErrImagePull", "ImagePullBackOff", "InvalidImageName"] {
            assert_eq!(
                KubeFailure::from_pod(&waiting(reason, "not found"), "x:1", "d-data"),
                Some(KubeFailure::ImagePull {
                    image: "x:1".into(),
                    message: "not found".into()
                }),
                "{reason}"
            );
        }
        assert_eq!(
            KubeFailure::from_pod(&waiting("ContainerCreating", ""), "x:1", "d-data"),
            None
        );
    }

    #[test]
    fn a_pod_no_node_can_run_is_unschedulable_and_an_unbound_claim_is_named() {
        let pending = |message: &str| PodStatusView {
            phase: "Pending".into(),
            scheduled: Some(false),
            schedule_reason: Some(("Unschedulable".into(), message.into())),
            ..PodStatusView::default()
        };
        assert_eq!(
            KubeFailure::from_pod(&pending("0/3 nodes are available"), "x:1", "d-data"),
            Some(KubeFailure::Unschedulable {
                message: "0/3 nodes are available".into()
            })
        );
        let unbound = "0/3 nodes: pod has unbound immediate PersistentVolumeClaims";
        assert_eq!(
            KubeFailure::from_pod(&pending(unbound), "x:1", "d-data"),
            Some(KubeFailure::ClaimNotBound {
                claim: "d-data".into(),
                message: unbound.into()
            })
        );
    }

    #[test]
    fn a_container_that_ended_does_not_meet_the_image_contract() {
        assert_eq!(
            KubeFailure::from_pod(&waiting("Error", "exit 1"), "x:1", "d-data"),
            Some(KubeFailure::ImageContract {
                image: "x:1".into(),
                missing: "the container exited: Error".into()
            })
        );
    }

    #[test]
    fn a_provisioning_failure_event_leaves_the_claim_not_bound() {
        let event = |reason: &str, message: &str| EventView {
            reason: reason.into(),
            message: message.into(),
        };
        assert_eq!(
            KubeFailure::from_pvc(
                "d-data",
                &[
                    event("Scheduled", "ok"),
                    event("ProvisioningFailed", "no class")
                ]
            ),
            Some(KubeFailure::ClaimNotBound {
                claim: "d-data".into(),
                message: "no class".into()
            })
        );
        assert_eq!(
            KubeFailure::from_pvc("d-data", &[event("Bound", "ok")]),
            None
        );
    }

    fn pod(phase: &str) -> PodStatusView {
        PodStatusView {
            phase: phase.into(),
            running: phase == "Running",
            node: Some("n1".into()),
            ..PodStatusView::default()
        }
    }

    #[test]
    fn a_pod_that_is_gone_says_why() {
        let evicted = PodStatusView {
            phase: "Failed".into(),
            reason: Some(("Evicted".into(), "node was low on memory".into())),
            ..pod("Failed")
        };
        let terminating = PodStatusView {
            deleting: true,
            ..pod("Running")
        };
        let cases = [
            (None, true, Some("stopped: the pod was deleted")),
            (
                Some(&terminating),
                true,
                Some("stopped: the pod was deleted"),
            ),
            (
                Some(&evicted),
                true,
                Some("stopped: the pod was evicted: node was low on memory"),
            ),
            (
                Some(&pod("Unknown")),
                true,
                Some("stopped: the pod's node n1 was lost"),
            ),
            (
                Some(&pod("Running")),
                false,
                Some("stopped: the pod's node n1 was lost"),
            ),
            (Some(&pod("Running")), true, None),
            (
                Some(&pod("Pending")),
                false,
                Some("stopped: the pod's node n1 was lost"),
            ),
        ];
        for (status, node_exists, expected) in cases {
            let got = PodGone::classify(status, node_exists).map(|g| g.to_string());
            assert_eq!(
                got.as_deref(),
                expected,
                "{status:?} node_exists={node_exists}"
            );
        }
        let unscheduled = PodStatusView {
            node: None,
            ..pod("Pending")
        };
        assert_eq!(PodGone::classify(Some(&unscheduled), false), None);
    }
}
