//! The Kubernetes daemon runtime (feature 686): pure rules and the `Cluster` port. No `kube` crate
//! here; the live adapter is `micold-client::kube::live`.

pub mod classify;
pub mod credentials;
#[cfg(any(test, feature = "kube-fake"))]
pub mod fake;
pub mod kubeconfig;
pub mod names;
pub mod settings;
pub mod start;

use std::collections::BTreeMap;
use std::fmt;

use futures_util::future::BoxFuture;

use tokio::io::{AsyncRead, AsyncWrite};

/// A byte stream, such as the one [`Cluster::port_forward`] opens to a pod's port.
pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncReadWrite for T {}

/// The kinds of object the client creates or reads (contract §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectKind {
    /// The daemon's pod.
    Pod,
    /// The daemon's volume claim.
    PersistentVolumeClaim,
    /// A credential or pull secret.
    Secret,
}

impl ObjectKind {
    /// The API kind, as a failure names it.
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectKind::Pod => "Pod",
            ObjectKind::PersistentVolumeClaim => "PersistentVolumeClaim",
            ObjectKind::Secret => "Secret",
        }
    }

    /// The kind as a sentence names it (`a pod named …`).
    pub fn noun(self) -> &'static str {
        match self {
            ObjectKind::Pod => "pod",
            ObjectKind::PersistentVolumeClaim => "volume claim",
            ObjectKind::Secret => "secret",
        }
    }
}

impl fmt::Display for ObjectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A failed API call, as the API server's `Status` names it. A connection that never reached the
/// server has `code` 0 and `reason` `Unreachable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiFailure {
    /// The HTTP status code, 0 when no response came.
    pub code: u16,
    /// The `Status.reason` (`NotFound`, `AlreadyExists`, `Forbidden`, …).
    pub reason: String,
    /// The `Status.message`.
    pub message: String,
}

impl fmt::Display for ApiFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.code, self.reason, self.message)
    }
}

/// What a create returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Created {
    /// `201`: this call created it.
    Created {
        /// The new object's UID.
        uid: String,
    },
    /// `409 AlreadyExists`: something by that name is there already.
    AlreadyExists,
}

/// The parts of an object the sequences decide on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectView {
    /// Its UID, for delete preconditions.
    pub uid: String,
    /// Its `resourceVersion`, for replace preconditions.
    pub resource_version: String,
    /// Its labels, for ownership.
    pub labels: BTreeMap<String, String>,
    /// Whether `deletionTimestamp` is set.
    pub deleting: bool,
    /// The claim's requested storage, for a `PersistentVolumeClaim`.
    pub storage: Option<String>,
}

/// A pod's status, reduced to what start, reconnect and the classifiers read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodStatusView {
    /// `status.phase` (`Pending`, `Running`, `Failed`, …).
    pub phase: String,
    /// The `PodScheduled` condition: `Some(true)` once scheduled.
    pub scheduled: Option<bool>,
    /// That condition's reason and message, when not scheduled.
    pub schedule_reason: Option<(String, String)>,
    /// The container's waiting reason and message (`ContainerCreating`, `ErrImagePull`, …).
    pub waiting: Option<(String, String)>,
    /// Whether the container is running.
    pub running: bool,
}

/// An event about an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventView {
    /// The event's reason (`ProvisioningFailed`, `FailedScheduling`, …).
    pub reason: String,
    /// Its message.
    pub message: String,
}

/// A boxed result future of the [`Cluster`] port.
pub type ClusterFuture<'a, T> = BoxFuture<'a, Result<T, ApiFailure>>;

/// The cluster as the start, stop and remove sequences see it (research K0). The live adapter is
/// `micold-client::kube::live::LiveCluster`; tests use [`fake::FakeCluster`].
pub trait Cluster: Send + Sync {
    /// Create `manifest` (a JSON object of `kind`) in `namespace`.
    fn create(
        &self,
        namespace: &str,
        kind: ObjectKind,
        manifest: serde_json::Value,
    ) -> ClusterFuture<'_, Created>;
    /// Replace the object `name` of `kind` with `manifest`, if its `resourceVersion` is still
    /// `resource_version`; a stale one answers `409 Conflict` and changes nothing.
    fn replace(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        resource_version: &str,
        manifest: serde_json::Value,
    ) -> ClusterFuture<'_, ()>;
    /// Read an object; `None` on `404`.
    fn get(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
    ) -> ClusterFuture<'_, Option<ObjectView>>;
    /// Delete by name with `preconditions.uid`; `404` counts as done.
    fn delete(
        &self,
        namespace: &str,
        kind: ObjectKind,
        name: &str,
        uid: &str,
    ) -> ClusterFuture<'_, ()>;
    /// Whether the namespace exists.
    fn namespace_exists(&self, namespace: &str) -> ClusterFuture<'_, bool>;
    /// A `SelfSubjectAccessReview`: may this user `verb` `resource` in `namespace`?
    fn access_review(&self, namespace: &str, verb: &str, resource: &str)
        -> ClusterFuture<'_, bool>;
    /// The pod's status; `None` when it does not exist.
    fn pod_status(&self, namespace: &str, pod: &str) -> ClusterFuture<'_, Option<PodStatusView>>;
    /// Events about the named object, oldest first.
    fn events(&self, namespace: &str, name: &str) -> ClusterFuture<'_, Vec<EventView>>;
    /// Open a port-forward to `port` of the pod.
    fn port_forward(
        &self,
        namespace: &str,
        pod: &str,
        port: u16,
    ) -> ClusterFuture<'_, Box<dyn AsyncReadWrite>>;
    /// The last `tail` lines of the pod's log.
    fn logs(&self, namespace: &str, pod: &str, tail: u32) -> ClusterFuture<'_, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_reads_as_a_lowercase_noun() {
        assert_eq!(ObjectKind::Pod.noun(), "pod");
        assert_eq!(ObjectKind::PersistentVolumeClaim.noun(), "volume claim");
        assert_eq!(ObjectKind::Secret.noun(), "secret");
    }
}
