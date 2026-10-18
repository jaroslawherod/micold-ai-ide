//! The Kubernetes daemon runtime's client side (feature 686): the live cluster adapter, the
//! loopback relay and the SSH key files.

pub mod keys;
pub mod live;
pub mod relay;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use micold_core::kube::classify::KubeFailure;
use micold_core::kube::Cluster;

/// Connects to the cluster of the named kubeconfig context.
pub type KubeConnector = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<Arc<dyn Cluster>, KubeFailure>> + Send>>
        + Send
        + Sync,
>;
